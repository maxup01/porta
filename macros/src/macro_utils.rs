use proc_macro2::TokenStream;
use quote::{quote, format_ident};
use std::vec::Vec;
use syn::{FnArg, ItemFn, Lit, Meta, NestedMeta, Pat, PatType, punctuated::Punctuated, token::Comma};
use utils::request::route::Method;

/// Searches the attribute argument list for a `path = "..."` key-value pair
/// and returns the path string if found.
///
/// # Arguments
/// * `args` - Slice of [`NestedMeta`] parsed from the attribute's argument list.
///
/// # Returns
/// `Some(String)` containing the path value if a `path = "..."` argument is present,
/// or `None` if no such argument exists.
///
/// # Example
/// ```ignore
/// // Given: #[get(path = "/users/{id}")]
/// // args would contain: path = "/users/{id}"
/// let path = get_route_path_attribute_value(&args);
/// assert_eq!(path, Some("/users/{id}".to_string()));
/// ```
pub fn get_route_path_attribute_value(
    args: &[NestedMeta],
) -> Option<String> {
    for arg in args {
        if let NestedMeta::Meta(Meta::NameValue(nv)) = arg 
            && nv.path.is_ident("path") 
            && let Lit::Str(lit_str) = &nv.lit
        {
            return Some(lit_str.value());
        }
    }

    None
}

/// Extracts the name and type of each typed argument from a function's parameter list,
/// skipping `self` receivers.
///
/// # Arguments
/// * `args` - Punctuated list of [`FnArg`] from the function signature.
///
/// # Returns
/// A `Vec` of `(Ident, Type)` pairs, one per typed parameter in order.
///
/// # Example
/// ```ignore
/// // Given: fn handler(id: u32, name: String) -> String { ... }
/// let pairs = get_input_arg_idents_and_types(&input_fn.sig.inputs);
/// // pairs == [("id", u32), ("name", String)]
/// ```
pub fn get_input_arg_idents_and_types(args: &Punctuated<FnArg, Comma>) -> Vec<(syn::Ident, syn::Type)> {
    let mut fn_args: Vec<(syn::Ident, syn::Type)> = vec![];

    for arg in args {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg && let Pat::Ident(pat_ident) = &**pat {
            fn_args.push((pat_ident.ident.clone(), (**ty).clone()));
        }
    }

    fn_args
}

pub fn generate_route_handler_tokens(path: &str, http_method: Method, input_fn: &ItemFn) -> TokenStream {
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let fn_vis = &input_fn.vis;

    let fn_args = get_input_arg_idents_and_types(&input_fn.sig.inputs);

    let deserialized_args = generate_deserialization_block(&fn_args);

    let path_params: Vec<String> =
        utils::request::path_param::extract_path_param_names_from_path(path).collect();

    let register_fn_name = format_ident!("register_route_{}", fn_name);

    let method_as_tokens = method_tokens(&http_method); 

    let method_related_block = match http_method {
        Method::POST | Method::PATCH => {
            let mut not_path_param: String = String::new();

            for (arg_name, _) in &fn_args {
                let arg_name_str = arg_name.to_string();

                if !path_params.contains(&arg_name_str) {
                    not_path_param = arg_name.to_string();
                    break;
                }
            }

            quote! {
                map_with_params.insert(#not_path_param.to_string(),
                utils::request::request_body::extract_request_body(request).unwrap().to_string());
            }
        },
        Method::GET | Method::DELETE => {
            quote! {
                let query_params = utils::request::query::extract_params(path_from_request.as_str());

                if let Some(extracted_query_params) = query_params {
                    map_with_params.extend(extracted_query_params);
                } 
            }
        }
    };

    let fn_expanded = quote! {
        #fn_vis fn #fn_name(request: &str) -> String {
            let path_from_request = utils::request::route::extract_path_from_request(request).unwrap();
            let mut map_with_params = utils::request::path_param::extract_path_params(
                #path, path_from_request.as_str()
            ).unwrap();

            #method_related_block 

            #( #deserialized_args )*

            let fn_result = (|| #fn_block )();
            utils::response::format_response(fn_result)
        }

        #[ctor::ctor]
        fn #register_fn_name() {
            utils::request::route::register_route(utils::request::route::Method::#method_as_tokens, #path, #fn_name);
        }
    };

    fn_expanded
}

fn method_tokens(http_method: &Method) -> TokenStream {
    match http_method {
        Method::GET => quote! {GET},
        Method::POST => quote! {POST},
        Method::PATCH => quote! {PATCH},
        Method::DELETE => quote! {DELETE}
    }
}

fn generate_deserialization_block(fn_args: &Vec<(syn::Ident, syn::Type)>) -> Vec<TokenStream> {
    let mut deserialized = vec![];

    for (arg_name, arg_type) in fn_args {
        let arg_str = arg_name.to_string();
        let ty_str = quote!(#arg_type).to_string();

        if ty_str == "u8"
            || ty_str == "u16"
            || ty_str == "u32"
            || ty_str == "u64"
            || ty_str == "usize"
            || ty_str == "i8"
            || ty_str == "i16"
            || ty_str == "i32"
            || ty_str == "i64"
            || ty_str == "isize"
            || ty_str == "f32"
            || ty_str == "f64"
        {
            deserialized.push(quote! {
                let param_val_orig = map_with_params.get(&#arg_str[..]).unwrap().as_str();
                let #arg_name: #arg_type = match param_val_orig.parse() {
                    Ok(val) => val,
                    Err(_) => {
                        return format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                        "Not Found".len(),
                        "Not Found"
                    )}
                };
            });
        } else if ty_str == "bool" {
            deserialized.push(quote! {
                let param_val_orig = map_with_params.get(&#arg_str[..]).unwrap().as_str();
                let #arg_name: #arg_type = match param_val_orig {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => {
                        return format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                        "Not Found".len(),
                        "Not Found"
                    )}
                };
            });
        } else if ty_str == "String" {
            deserialized.push(quote! {
                let param_val_orig = map_with_params.get(&#arg_str[..]).unwrap().as_str();
                let #arg_name: #arg_type = param_val_orig.to_string();
            });
        } else {
            deserialized.push(quote! {
                let param_val_orig = map_with_params.get(&#arg_str[..]).unwrap().as_str();
                let param_val: &str;
                let formatted;

                if !(param_val_orig.starts_with("{") && param_val_orig.ends_with("}")) {
                    formatted = format!("\"{}\"", param_val_orig);
                    param_val = &formatted;
                } else {
                    param_val = param_val_orig;
                }

                let #arg_name: #arg_type = match serde_json::from_str(param_val) {
                    Ok(val) => val,
                    Err(_) => {
                        return format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                        "Not Found".len(),
                        "Not Found"
                    )}
                };
            });
        }
    }

    deserialized
}

#[cfg(test)]
#[path = "macro_utils_tests.rs"]
mod tests;
