extern crate proc_macro;

use proc_macro2::TokenStream;
use syn::{parse_macro_input, AttributeArgs, ItemFn, NestedMeta, Meta, FnArg, PatType, Pat};
use quote::{quote, format_ident};
use std::vec::Vec;

fn generate_deserialization_block(fn_args: &Vec<(syn::Ident, syn::Type)>) -> Vec<TokenStream> {
    let mut deserialized = vec![];

    for (arg_name, arg_type) in fn_args {
        let arg_str = arg_name.to_string();

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

            let #arg_name: #arg_type = serde_json::from_str(param_val)
                .expect(&format!("Failed to deserialize argument {}", #arg_str));
        });
    }

    deserialized
}

#[proc_macro_attribute]
pub fn get(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let fn_vis = &input_fn.vis;

    let mut path = None;
    let mut fn_args: Vec<(syn::Ident, syn::Type)> = vec![];

    for arg in args.iter() {
        if let NestedMeta::Meta(meta) = arg {
            match meta {
                Meta::NameValue(nv) => {
                    if nv.path.get_ident().unwrap() == "path" {
                        path = Some(nv.lit.clone())
                    }
                }
                _ => {}
            }
        }
    }

    let path= match path.unwrap() {
        syn::Lit::Str(lit_str) => lit_str.value(), // extract the inner string value
        _ => panic!("Expected a string literal for path"),
    };

    for arg in &input_fn.sig.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Pat::Ident(pat_ident) = &**pat {
                fn_args.push((pat_ident.ident.clone(), (**ty).clone()));
            }
        }
    }

    let register_fn_name = format_ident!("register_route_{}", fn_name);

    let deserialized_args = generate_deserialization_block(&fn_args);

    let expanded = quote! {
        use utils::request::route::{register_route, extract_path_from_request, Method};
        use utils::response::http_response::format_response;
        use utils::request::query::extract_params;
        use serde_json;

        #fn_vis fn #fn_name(request: &str) -> String {

            let map_with_params = extract_params(&extract_path_from_request(request).unwrap());

            #( #deserialized_args )*

            let fn_result = (|| #fn_block )();
            format_response(fn_result)
        }
        
        #[ctor::ctor]
        fn #register_fn_name() {
            register_route(Method::GET, #path, #fn_name);
        }   
    };

    expanded.into()
}