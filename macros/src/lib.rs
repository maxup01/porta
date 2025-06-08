extern crate proc_macro;

use proc_macro2::TokenStream;
use syn::{parse_macro_input, AttributeArgs, ItemFn, NestedMeta, Meta,
     FnArg, PatType, Pat, Lit};
use quote::{quote, format_ident};
use std::vec::Vec;

#[proc_macro]
pub fn inject_common_imports(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let expanded: TokenStream = quote! {
        use utils::request::route::{register_route, extract_path_from_request, Method};
        use utils::response::http_response::format_response;
        use utils::request::query::extract_params;
        use utils::request::path_param::extract_path_params;
        use serde_json;
    };

    expanded.into()
}

fn generate_deserialization_block_for_params(fn_args: &Vec<(syn::Ident, syn::Type)>) -> Vec<TokenStream> {
    let mut deserialized = vec![];

    for (arg_name, arg_type) in fn_args {
        let arg_str = arg_name.to_string();
        let ty_str = quote!(#arg_type).to_string();

        if ty_str == "u8" || ty_str == "u16" || ty_str == "u32" || ty_str == "u64" || ty_str == "usize"
            || ty_str == "i8" || ty_str == "i16" || ty_str == "i32" || ty_str == "i64" || ty_str == "isize"
            || ty_str == "f32" || ty_str == "f64"
        {
            deserialized.push(quote! {
                let param_val_orig = map_with_params.get(&#arg_str[..]).unwrap().as_str();
                let #arg_name: #arg_type = param_val_orig.parse()
                    .expect(&format!("Failed to parse argument {} as {}", #arg_str, #ty_str));
            });
        }
        else if ty_str == "bool" {
            deserialized.push(quote! {
                let param_val_orig = map_with_params.get(&#arg_str[..]).unwrap().as_str();
                let #arg_name: #arg_type = match param_val_orig {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => panic!("Failed to parse argument {} as bool", #arg_str),
                };
            });
        }
        else if ty_str == "String" {
            deserialized.push(quote! {
                let param_val_orig = map_with_params.get(&#arg_str[..]).unwrap().as_str();
                let #arg_name: #arg_type = param_val_orig.to_string();
            });
        }
        else {
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

                let #arg_name: #arg_type = ::serde_json::from_str(param_val)
                    .expect(&format!("Failed to deserialize argument {}", #arg_str));
            });
        }
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
        syn::Lit::Str(lit_str) => lit_str.value(),
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

    let deserialized_args = generate_deserialization_block_for_params(&fn_args);

    let expanded = quote! {

        #fn_vis fn #fn_name(request: &str) -> String {

            let path_from_request = &utils::request::route::extract_path_from_request(request).unwrap();
            let path_params = ::utils::request::path_param::extract_path_params(#path, path_from_request);
            let mut map_with_params = ::utils::request::query::extract_params(path_from_request);
            map_with_params.extend(path_params);

            #( #deserialized_args )*

            let fn_result = (|| #fn_block )();
            ::utils::response::http_response::format_response(fn_result)
        }
        
        #[ctor::ctor]
        fn #register_fn_name() {
            ::utils::request::route::register_route(::utils::request::route::Method::GET, #path, #fn_name);
        }   
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn delete(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
        syn::Lit::Str(lit_str) => lit_str.value(),
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

    let deserialized_args = generate_deserialization_block_for_params(&fn_args);

    let expanded = quote! {

        #fn_vis fn #fn_name(request: &str) -> String {

            let path_from_request = &::utils::request::route::extract_path_from_request(request).unwrap();
            let path_params = ::utils::request::path_param::extract_path_params(
                #path, path_from_request.as_str());
            let mut map_with_params = 
                ::utils::request::query::extract_params(path_from_request.as_str());
            map_with_params.extend(path_params);

            #( #deserialized_args )*

            let fn_result = (|| #fn_block )();
            ::utils::response::http_response::format_response(fn_result)
        }
        
        #[ctor::ctor]
        fn #register_fn_name() {
            ::utils::request::route::register_route(::utils::request::route::Method::DELETE, #path, #fn_name);
        }   
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn post(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
        syn::Lit::Str(lit_str) => lit_str.value(),
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

    let deserialized_args = generate_deserialization_block_for_params(&fn_args);
    let mut not_path_param: String = String::new();
    let path_params = ::utils::request::path_param::extract_path_param_names_from_path(&path);

    for (arg_name, _) in &fn_args {
        if !path_params.contains(&arg_name.to_string()) {
            not_path_param = arg_name.to_string();
            break;
        }
    }

    let expanded = quote! {

        #fn_vis fn #fn_name(request: &str) -> String {

            let path_from_request = ::utils::request::route::extract_path_from_request(request).unwrap();
            let mut map_with_params = ::utils::request::path_param::extract_path_params(#path, path_from_request.as_str());
            map_with_params.insert(#not_path_param.to_string(),
                ::utils::request::request_body::extract_request_body(request).unwrap().to_string());

            #( #deserialized_args )*

            let fn_result = (|| #fn_block )();
            ::utils::response::http_response::format_response(fn_result)
        }
        
        #[ctor::ctor]
        fn #register_fn_name() {
            ::utils::request::route::register_route(::utils::request::route::Method::POST, #path, #fn_name);
        }   
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn patch(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
        syn::Lit::Str(lit_str) => lit_str.value(),
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

    let deserialized_args = generate_deserialization_block_for_params(&fn_args);
    let mut not_path_param: String = String::new();
    let path_params = ::utils::request::path_param::extract_path_param_names_from_path(&path);

    for (arg_name, _) in &fn_args {
        if !path_params.contains(&arg_name.to_string()) {
            not_path_param = arg_name.to_string();
            break;
        }
    }

    let expanded = quote! {

        #fn_vis fn #fn_name(request: &str) -> String {

            let path_from_request = ::utils::request::route::extract_path_from_request(request).unwrap();
            let mut map_with_params = ::utils::request::path_param::extract_path_params(#path, path_from_request.as_str());
            map_with_params.insert(#not_path_param.to_string(),
                ::utils::request::request_body::extract_request_body(request).unwrap().to_string());

            #( #deserialized_args )*

            let fn_result = (|| #fn_block )();
            ::utils::response::http_response::format_response(fn_result)
        }
        
        #[ctor::ctor]
        fn #register_fn_name() {
            ::utils::request::route::register_route(::utils::request::route::Method::PATCH, #path, #fn_name);
        }   
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn unsecure_http_server(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let input_fn = parse_macro_input!(item as ItemFn);
    let sig = &input_fn.sig;

    if sig.ident != "main" {
        panic!("The unsecure_http_server macro can only be applied to the main function.");
    }

    let mut ip_lit = None;
    let mut port_lit = None;

    for arg in args.iter() {
        if let NestedMeta::Meta(meta) = arg {
            match meta {
                Meta::NameValue(nv) => {
                    if nv.path.get_ident().unwrap() == "ip" {
                        if let Lit::Str(lit_str) = &nv.lit {
                            ip_lit = Some(lit_str.value());
                        }
                    }
                    else if nv.path.get_ident().unwrap() == "port" {
                        if let Lit::Int(lit_int) = &nv.lit {
                            port_lit = Some(lit_int.base10_parse::<u16>().unwrap());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let ip_str = ip_lit.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = port_lit.unwrap_or(8080);

    let expanded = quote! {
        use std::net::SocketAddr;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        #[tokio::main]
        #sig {
            let addr = format!("{}:{}", #ip_str, #port);
            let listener = ::tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind address");

            loop {
                let (mut socket, _) = listener.accept().await.expect("Failed to accept connection");
                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    let n = match socket.read(&mut buffer).await {
                        Ok(n) if n == 0 => return,
                        Ok(n) => n,
                        Err(_) => return,
                    };

                    let request = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let path = ::utils::request::route::extract_path_from_request(&request).unwrap_or_default();

                    let route_path = ::utils::request::route::get_matching_route_path(&path);

                    let mut response = format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                        "Not Found".len(),
                        "Not Found"
                    ); 

                    if let Some(route_path) = route_path {
                        match ::utils::request::route::get_route_function(
                            &route_path, ::utils::request::route::extract_method_from_request(&request)
                            .unwrap_or(::utils::request::route::Method::GET)) {
                            Some(route_function) => {
                                response = route_function(&request);
                            }
                            None => {}
                        }
                    }

                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        }
    };

    expanded.into()
}

