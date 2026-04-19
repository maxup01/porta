extern crate proc_macro;

mod macro_utils;

use macro_utils::*;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Lit, Meta, NestedMeta};
use utils::request::route::Method;

#[proc_macro_attribute]
pub fn get(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = get_route_path_attribute_value(&args)
        .expect("Path for route handler not specified"); 

    let expanded = generate_route_handler_tokens(&path, Method::GET, &input_fn); 

    expanded.into()
}

#[proc_macro_attribute]
pub fn delete(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = get_route_path_attribute_value(&args)
        .expect("Path for route handler not specified"); 

    let expanded = generate_route_handler_tokens(&path, Method::DELETE, &input_fn); 

    expanded.into()
}

#[proc_macro_attribute]
pub fn post(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = get_route_path_attribute_value(&args)
        .expect("Path for route handler not specified"); 

    let expanded = generate_route_handler_tokens(&path, Method::POST, &input_fn);
    
    expanded.into()
}

#[proc_macro_attribute]
pub fn patch(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = get_route_path_attribute_value(&args)
        .expect("Path for route handler not specified"); 

    let expanded = generate_route_handler_tokens(&path, Method::PATCH, &input_fn);
    
    expanded.into()
}

#[proc_macro_attribute]
pub fn unsecure_http_server(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let input_fn = parse_macro_input!(item as ItemFn);
    let sig = &input_fn.sig;

    if sig.ident != "main" {
        panic!("The unsecure_http_server macro can only be applied to the main function.");
    }

    let mut ip_lit = None;
    let mut port_lit = None;

    for arg in args.iter() {
        if let NestedMeta::Meta(Meta::NameValue(nv)) = arg {  
            if let Lit::Str(lit_str) = &nv.lit && nv.path.is_ident("ip") {
                ip_lit = Some(lit_str.value()); 
            } else if let Lit::Int(lit_int) = &nv.lit && nv.path.is_ident("port") { 
                port_lit = Some(lit_int.base10_parse::<u16>().expect("Given invalid port number"))
            }
        }
    }

    let ip_str = ip_lit.expect("Ip is not specified");
    let port = port_lit.expect("Port is not specified");

    let expanded = quote! {
        use std::net::SocketAddr;
        use tokio::io::{AsyncReadExt, AsyncWriteExt}; 

        #[tokio::main]
        #sig {
            let addr = format!("{}:{}", #ip_str, #port);
            let listener = tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind address");

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
                    let path = utils::request::route::extract_path_from_request(&request).unwrap_or_default(); 

                    let mut response = format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                        "Not Found".len(),
                        "Not Found"
                    );

                    let route_path = utils::request::route::get_matching_route_path(&path);

                    let request_method = 
                        utils::request::route::extract_method_from_request(&request);

                    if let Some(route_path) = route_path 
                        && let Ok(method) = request_method
                        && let Ok(Some(route_function)) = utils::request::route::get_route_function(&route_path, method)
                    { 
                        response = route_function(&request);
                    }

                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        }
    };

    expanded.into()
}
