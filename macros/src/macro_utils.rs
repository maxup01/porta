use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::vec::Vec;
use syn::{
    FnArg, Ident, ItemFn, Lit, LitInt, LitStr, Meta, NestedMeta, Pat, PatType, Token, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
};
use utils::request::route::Method;

/// The arguments to `#[http_server(..)]`, parsed.
///
/// This is parsed by hand rather than through [`syn::AttributeArgs`] because
/// `AttributeArgs` cannot represent `allow_origins = ["a", "b"]`: its
/// `Meta::NameValue` takes a single [`Lit`] on the right, and a list literal is
/// not a literal. The choice is between a list-of-lists syntax the attribute
/// would have to invent — `allow_origins("a", "b")` — and reading the tokens
/// directly, which is what this does.
///
/// Doing so is also how the attribute reports mistakes properly. Every failure
/// below carries the span of the token that caused it, so a misspelled argument
/// underlines the argument rather than aborting the whole expansion with a
/// panic message and no location.
#[derive(Debug, PartialEq, Eq)]
pub struct HttpServerArgs {
    /// IP address to bind to.
    pub ip: String,

    /// TCP port to listen on.
    pub port: u16,

    /// Origins permitted to call this server from a browser. Empty means no CORS
    /// headers are emitted at all, which is the default.
    pub allow_origins: Vec<String>,
}

impl Parse for HttpServerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut ip: Option<String> = None;
        let mut port: Option<u16> = None;
        let mut allow_origins: Option<Vec<String>> = None;

        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match name.to_string().as_str() {
                "ip" => ip = Some(parse_once(&name, ip, input.parse::<LitStr>()?.value())?),
                "port" => {
                    port = Some(parse_once(
                        &name,
                        port,
                        input.parse::<LitInt>()?.base10_parse::<u16>()?,
                    )?)
                }
                "allow_origins" => {
                    allow_origins = Some(parse_once(
                        &name,
                        allow_origins,
                        parse_string_array(input)?,
                    )?)
                }
                unknown => {
                    return Err(syn::Error::new(
                        name.span(),
                        format!(
                            "unknown argument `{}`; expected one of `ip`, `port`, `allow_origins`",
                            unknown
                        ),
                    ));
                }
            }

            // A trailing comma is allowed; a missing one between arguments is not.
            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(HttpServerArgs {
            ip: ip.ok_or_else(|| input.error("`ip` is required, e.g. ip = \"127.0.0.1\""))?,
            port: port.ok_or_else(|| input.error("`port` is required, e.g. port = 8443"))?,
            allow_origins: allow_origins.unwrap_or_default(),
        })
    }
}

/// The arguments to `#[component(..)]`, parsed.
///
/// Read directly from the token stream rather than through [`syn::AttributeArgs`]
/// for the same reason [`HttpServerArgs`] is: every failure below carries the span
/// of the token that caused it, so a mistake underlines the argument that caused
/// it instead of the whole attribute.
#[derive(Debug, PartialEq, Eq)]
pub struct ComponentArgs {
    /// Name of the accessor generated on `AppContext`, so `name = "ticket_handler"`
    /// produces `AppContext::ticket_handler()`.
    ///
    /// Required rather than derived from the annotated type, because turning
    /// `TicketHandler` into `ticket_handler` has no answer a caller would predict
    /// for `HTTPHandler` or `TicketDb`. Naming it explicitly also puts the accessor
    /// next to the type it hands out, which is where a reader looks for it.
    pub name: String,
}

/// Accepts `name = "..."`, with a trailing comma allowed.
///
/// `name` is rejected when given twice rather than letting the last one win, for
/// the reason described on [`parse_once`], and when it is empty or holds a space.
/// It becomes an identifier in generated code, and an invalid one aborts the
/// expansion from inside `format_ident!` with no span to report.
impl Parse for ComponentArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<String> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;

            match key.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    name = Some(parse_once(&key, name, input.parse::<LitStr>()?.value())?);
                }
                unknown => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown argument `{unknown}`; expected `name` or `sync`"),
                    ));
                }
            }

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        if name.is_none() {
            return Err(syn::Error::new(
                input.span(),
                "`name` argument not specified",
            ));
        }

        let name = name.unwrap();

        if name.contains(' ') {
            return Err(syn::Error::new(
                input.span(),
                "`name` argument is invalid, it shouldn't contain space",
            ));
        } else if name.trim().is_empty() {
            return Err(syn::Error::new(input.span(), "`name` argument is empty"));
        }

        Ok(ComponentArgs { name })
    }
}

/// Returns `value`, or an error if this argument was already given.
///
/// Silently letting the last one win would make `ip = "0.0.0.0", ip = "127.0.0.1"`
/// bind somewhere the source does not obviously say.
fn parse_once<T>(name: &Ident, existing: Option<T>, value: T) -> syn::Result<T> {
    match existing {
        Some(_) => Err(syn::Error::new(
            name.span(),
            format!("`{}` is given more than once", name),
        )),
        None => Ok(value),
    }
}

/// Parses `["a", "b"]` into the strings it contains.
fn parse_string_array(input: ParseStream) -> syn::Result<Vec<String>> {
    let content;
    bracketed!(content in input);

    Ok(Punctuated::<LitStr, Comma>::parse_terminated(&content)?
        .into_iter()
        .map(|literal| literal.value())
        .collect())
}

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
pub fn get_route_path_attribute_value(args: &[NestedMeta]) -> Option<String> {
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
pub fn get_input_arg_idents_and_types(
    args: &Punctuated<FnArg, Comma>,
) -> Vec<(syn::Ident, syn::Type)> {
    let mut fn_args: Vec<(syn::Ident, syn::Type)> = vec![];

    for arg in args {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg
            && let Pat::Ident(pat_ident) = &**pat
        {
            fn_args.push((pat_ident.ident.clone(), (**ty).clone()));
        }
    }

    fn_args
}

/// Splits a handler's parameter list into the components it asks for and the
/// parameters that come from the request.
///
/// A component is a parameter marked `#[component]`. It is taken out of the list
/// rather than passed along, because everything downstream —
/// [`get_input_arg_idents_and_types`], the deserialization block, the choice of
/// which parameter binds the request body — assumes every parameter it is given was
/// parsed out of the request. A component is not, and left in place it would be
/// looked up in the parameter map, miss, and answer `400` on every request.
///
/// The type returned for a component is the referent rather than the reference:
/// `&TicketHandler` comes back as `TicketHandler`. That is the type the instance is
/// stored and implemented on, so it is the one the caller needs to name.
///
/// # Arguments
/// * `inputs` - Punctuated list of [`FnArg`] from the handler's signature.
///
/// # Returns
/// The component parameters as `(Ident, Type)` pairs, and the parameters that
/// remain, in their original order.
///
/// # Errors
/// A `#[component]` parameter that is not a shared reference, or that is not a plain
/// name. Neither can be resolved to the `&'static` instance held in a static, and
/// reporting it here puts the error on the parameter the caller wrote instead of on
/// generated code they cannot see.
///
/// # Example
/// ```ignore
/// // Given: fn handler(id: u32, #[component] tickets: &TicketHandler) { ... }
/// let (components, request_inputs) = split_component_args(&input_fn.sig.inputs)?;
/// // components     == [("tickets", TicketHandler)]
/// // request_inputs == id: u32
/// ```
pub fn split_component_args(
    inputs: &Punctuated<FnArg, Comma>,
) -> syn::Result<(Vec<(syn::Ident, syn::Type)>, Punctuated<FnArg, Comma>)> {
    let mut component_args: Vec<(syn::Ident, syn::Type)> = vec![];
    let mut request_inputs: Punctuated<FnArg, Comma> = Punctuated::new();

    for arg in inputs {
        let FnArg::Typed(pat_type) = arg else {
            // A `self` receiver is not a component and not a request parameter.
            // Passing it through leaves it to the existing extractor, which skips it.
            request_inputs.push(arg.clone());
            continue;
        };

        if !pat_type
            .attrs
            .iter()
            .any(|attr| attr.path.is_ident("component"))
        {
            request_inputs.push(arg.clone());
            continue;
        }

        let Pat::Ident(pat_ident) = &*pat_type.pat else {
            return Err(syn::Error::new_spanned(
                &pat_type.pat,
                "a `#[component]` parameter must be a plain name, e.g. `tickets: &TicketHandler`",
            ));
        };

        // The instance lives in a `static`, so it can only ever be lent out. A
        // component taken by value would have to be moved out of that static, and a
        // `&mut` would hand one request exclusive access to state every other request
        // shares — synchronization belongs inside the component's own fields.
        let syn::Type::Reference(reference) = &*pat_type.ty else {
            return Err(syn::Error::new_spanned(
                &pat_type.ty,
                "a `#[component]` parameter must be a shared reference, e.g. `&TicketHandler`",
            ));
        };

        if reference.mutability.is_some() {
            return Err(syn::Error::new_spanned(
                &pat_type.ty,
                "a `#[component]` parameter cannot be `&mut`; put a `Mutex` or `RwLock` \
                 on the field that needs mutating",
            ));
        }

        component_args.push((pat_ident.ident.clone(), (*reference.elem).clone()));
    }

    Ok((component_args, request_inputs))
}

/// Generates the full [`TokenStream`] for a route handler function and its
/// corresponding route registration constructor.
///
/// The generated function:
/// - Accepts a raw HTTP request string as `&str`
/// - Extracts path parameters from the URL using the provided `path` template
/// - For `GET`/`DELETE`: additionally extracts query parameters and merges them
/// - For `POST`/`PATCH`: extracts the request body and maps it to the non-path parameter
/// - Deserializes all parameters into their declared Rust types
/// - Calls the original function body and formats the return value as an HTTP response
///
/// A `#[ctor::ctor]` registration function is also emitted to register the handler
/// with the global route table at program startup.
///
/// # Arguments
/// * `path` - The route path template, e.g. `"/users/{id}"`.
/// * `http_method` - The HTTP method this handler responds to.
/// * `input_fn` - The parsed function item to transform.
///
/// # Returns
/// A [`TokenStream`] containing the transformed handler function and route registration.
pub fn generate_route_handler_tokens(
    path: &str,
    http_method: Method,
    input_fn: &ItemFn,
) -> TokenStream {
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
        // The verbs that carry a body. PUT replaces a resource where PATCH amends
        // one, but both bind the request body the same way.
        Method::POST | Method::PUT | Method::PATCH => {
            let mut not_path_param: String = String::new();

            for (arg_name, _) in &fn_args {
                let arg_name_str = arg_name.to_string();

                if !path_params.contains(&arg_name_str) {
                    not_path_param = arg_name.to_string();
                    break;
                }
            }

            quote! {
                let body = match ::embedded_web_server::utils::request::request_body::extract_request_body(request) {
                    Some(body) => body,
                    None => {
                        return ::embedded_web_server::utils::response::status_response(
                            ::embedded_web_server::utils::response::HttpStatus::BadRequest
                        )}
                };

                map_with_params.insert(#not_path_param.to_string(), body);
            }
        }
        Method::GET | Method::DELETE => {
            quote! {
                let query_params = ::embedded_web_server::utils::request::query::extract_params(path_from_request.as_str());

                if let Some(extracted_query_params) = query_params {
                    map_with_params.extend(extracted_query_params);
                }
            }
        }
    };

    let fn_expanded = quote! {
        #fn_vis fn #fn_name(request: &str) -> String {
            let path_from_request = match ::embedded_web_server::utils::request::route::extract_path_from_request(request) {
                Ok(path_from_request) => path_from_request,
                Err(_) => {
                    return ::embedded_web_server::utils::response::status_response(
                        ::embedded_web_server::utils::response::HttpStatus::BadRequest
                    )}
            };

            // Path params are matched against the route pattern, which has no query string.
            // `path_from_request` is kept intact for query param extraction below.
            let path_without_query = match path_from_request.split_once('?') {
                Some((path_only, _)) => path_only,
                None => path_from_request.as_str(),
            };

            let mut map_with_params = match ::embedded_web_server::utils::request::path_param::extract_path_params(
                #path, path_without_query
            ) {
                Ok(map_with_params) => map_with_params,
                Err(_) => {
                    return ::embedded_web_server::utils::response::status_response(
                        ::embedded_web_server::utils::response::HttpStatus::BadRequest
                    )}
            };

            #method_related_block

            #( #deserialized_args )*

            let fn_result = (|| #fn_block )();
            ::embedded_web_server::utils::response::format_response(fn_result)
        }

        #[::embedded_web_server::ctor::ctor]
        fn #register_fn_name() {
            ::embedded_web_server::utils::request::route::register_route(
                ::embedded_web_server::utils::request::route::Method::#method_as_tokens,
                #path,
                #fn_name
            );
        }
    };

    fn_expanded
}

/// Converts a [`Method`] enum variant into its corresponding [`TokenStream`] identifier,
/// used when emitting `Method::GET`, `Method::POST`, etc. into generated code.
fn method_tokens(http_method: &Method) -> TokenStream {
    match http_method {
        Method::GET => quote! {GET},
        Method::POST => quote! {POST},
        Method::PUT => quote! {PUT},
        Method::PATCH => quote! {PATCH},
        Method::DELETE => quote! {DELETE},
    }
}

/// Generates a `Vec` of [`TokenStream`] blocks that deserialize each function argument
/// from the `map_with_params` HashMap into its declared Rust type.
///
/// Deserialization strategy per type:
/// - **Numeric types** (`u8`–`f64`): parsed via `.parse()`, returns 404 on failure.
/// - **`bool`**: matched against `"true"`, `"1"`, `"false"`, `"0"`, returns 404 otherwise.
/// - **`String`**: converted directly via `.to_string()`.
/// - **All other types**: deserialized via `serde_json::from_str`. If the value is not
///   already a JSON object (`{...}`), it is wrapped in quotes first to allow
///   deserialization of string-backed enums and newtypes. Returns 404 on failure.
///
/// # Arguments
/// * `fn_args` - Slice of `(Ident, Type)` pairs representing the function parameters.
///
/// # Returns
/// A `Vec<TokenStream>` where each element is a `let` binding that deserializes
/// one parameter from `map_with_params`.
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
                let param_val_orig = match map_with_params.get(&#arg_str[..]) {
                    Some(param_val) => param_val.as_str(),
                    None => {
                        return ::embedded_web_server::utils::response::status_response(
                            ::embedded_web_server::utils::response::HttpStatus::BadRequest
                        )}
                };
                let #arg_name: #arg_type = match param_val_orig.parse() {
                    Ok(val) => val,
                    Err(_) => {
                        return ::embedded_web_server::utils::response::status_response(
                            ::embedded_web_server::utils::response::HttpStatus::NotFound
                        )}
                };
            });
        } else if ty_str == "bool" {
            deserialized.push(quote! {
                let param_val_orig = match map_with_params.get(&#arg_str[..]) {
                    Some(param_val) => param_val.as_str(),
                    None => {
                        return ::embedded_web_server::utils::response::status_response(
                            ::embedded_web_server::utils::response::HttpStatus::BadRequest
                        )}
                };
                let #arg_name: #arg_type = match param_val_orig {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => {
                        return ::embedded_web_server::utils::response::status_response(
                            ::embedded_web_server::utils::response::HttpStatus::NotFound
                        )}
                };
            });
        } else if ty_str == "String" {
            deserialized.push(quote! {
                let param_val_orig = match map_with_params.get(&#arg_str[..]) {
                    Some(param_val) => param_val.as_str(),
                    None => {
                        return ::embedded_web_server::utils::response::status_response(
                            ::embedded_web_server::utils::response::HttpStatus::BadRequest
                        )}
                };
                let #arg_name: #arg_type = param_val_orig.to_string();
            });
        } else {
            deserialized.push(quote! {
                let param_val_orig = match map_with_params.get(&#arg_str[..]) {
                    Some(param_val) => param_val.as_str(),
                    None => {
                        return ::embedded_web_server::utils::response::status_response(
                            ::embedded_web_server::utils::response::HttpStatus::BadRequest
                        )}
                };
                let param_val: &str;
                let formatted;

                if !(param_val_orig.starts_with("{") && param_val_orig.ends_with("}")) {
                    formatted = format!("\"{}\"", param_val_orig);
                    param_val = &formatted;
                } else {
                    param_val = param_val_orig;
                }

                let #arg_name: #arg_type = match ::embedded_web_server::serde_json::from_str(param_val) {
                    Ok(val) => val,
                    Err(_) => {
                        return ::embedded_web_server::utils::response::status_response(
                            ::embedded_web_server::utils::response::HttpStatus::NotFound
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
