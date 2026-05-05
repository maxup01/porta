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
pub fn http_server(
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
        use std::{net::SocketAddr, io::BufReader, sync::Arc};
        use tokio::{
            net::TcpStream,
            io::{AsyncReadExt, AsyncWriteExt}
        };
        use tokio_rustls::TlsAcceptor;
        use rustls::{
            pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
            ServerConfig, RootCertStore
        };
        use rcgen::generate_simple_self_signed; 

        #[tokio::main]
        #sig {
            let subject_alt_names = vec!["embedded-http-server-rs".to_string(), #ip_str.to_string()];
            let cert = generate_simple_self_signed(subject_alt_names).unwrap();

            let cert_der = CertificateDer::from(cert.cert.der().to_vec());
            let key_der = PrivateKeyDer::Pkcs8(
                PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der())
            );

            let server_config = ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![cert_der], key_der)
                .expect("Failed to initialize server config");

            let acceptor = TlsAcceptor::from(Arc::new(server_config));

            let addr = format!("{}:{}", #ip_str, #port);
            let listener = tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind address");

            loop {
                let (socket, _) = listener.accept().await.expect("Failed to accept connection");
                let acceptor = acceptor.clone();

                tokio::spawn(async move {
                    let mut tls_stream = acceptor
                        .accept(socket)
                        .await
                        .expect("TLS handshake failed");

                    let mut buffer = [0u8; 4096];
                    let n = match tls_stream.read(&mut buffer).await {
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

                    tls_stream
                        .write_all(response.as_bytes())
                        .await
                        .expect("Failed to write");
                });
            }
        }
    };

    expanded.into()
}
