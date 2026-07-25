extern crate proc_macro;

mod macro_utils;

use macro_utils::*;
use quote::quote;
use syn::{AttributeArgs, ItemFn, Lit, Meta, NestedMeta, parse_macro_input};
use utils::request::route::Method;

/// Registers the annotated function as a handler for HTTP `GET` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/hello"` or `"/users/:id"`)
///
/// The `path` argument is required. Omitting it causes a compile-time panic.
///
/// # See also
///
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`get_route_path_attribute_value`] — extracts the `path` value from the macro arguments
#[proc_macro_attribute]
pub fn get(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = get_route_path_attribute_value(&args).expect("Path for route handler not specified");

    let expanded = generate_route_handler_tokens(&path, Method::GET, &input_fn);

    expanded.into()
}

/// Registers the annotated function as a handler for HTTP `DELETE` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/users/:id"`)
///
/// The `path` argument is required. Omitting it causes a compile-time panic.
///
/// # See also
///
/// - [`get`] — the equivalent macro for `GET` requests
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`get_route_path_attribute_value`] — extracts the `path` value from the macro arguments
#[proc_macro_attribute]
pub fn delete(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = get_route_path_attribute_value(&args).expect("Path for route handler not specified");

    let expanded = generate_route_handler_tokens(&path, Method::DELETE, &input_fn);

    expanded.into()
}

/// Registers the annotated function as a handler for HTTP `POST` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/users"`)
///
/// The `path` argument is required. Omitting it causes a compile-time panic.
///
/// # See also
///
/// - [`get`] — the equivalent macro for `GET` requests
/// - [`delete`] — the equivalent macro for `DELETE` requests
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`get_route_path_attribute_value`] — extracts the `path` value from the macro arguments
#[proc_macro_attribute]
pub fn post(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = get_route_path_attribute_value(&args).expect("Path for route handler not specified");

    let expanded = generate_route_handler_tokens(&path, Method::POST, &input_fn);

    expanded.into()
}

/// Registers the annotated function as a handler for HTTP `PATCH` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/users/:id"`)
///
/// The `path` argument is required. Omitting it causes a compile-time panic.
///
/// # See also
///
/// - [`get`] — the equivalent macro for `GET` requests
/// - [`post`] — the equivalent macro for `POST` requests
/// - [`delete`] — the equivalent macro for `DELETE` requests
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`get_route_path_attribute_value`] — extracts the `path` value from the macro arguments
#[proc_macro_attribute]
pub fn patch(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = get_route_path_attribute_value(&args).expect("Path for route handler not specified");

    let expanded = generate_route_handler_tokens(&path, Method::PATCH, &input_fn);

    expanded.into()
}

/// An attribute macro that transforms `main` into a self-contained TLS HTTP server.
///
/// At compile time, this macro injects all boilerplate needed to:
/// - Generate a self-signed TLS certificate (via [`rcgen`])
/// - Bind a [`tokio::net::TcpListener`] to the given address
/// - Accept connections concurrently and perform TLS handshakes
/// - Parse incoming HTTP requests and dispatch them to registered route handlers
/// - Return a `404 Not Found` response when no route matches
///
/// # Arguments
///
/// - `ip` — IP address to bind to, as a string literal (e.g. `"127.0.0.1"` or `"0.0.0.0"`)
/// - `port` — TCP port to listen on, as an integer literal (e.g. `8443`)
///
/// Both arguments are required. Missing either one causes a compile-time panic.
///
/// # Constraints
///
/// This macro **must** be applied to a function named `main`. Applying it to any
/// other function name causes a compile-time panic.
///
/// # TLS
///
/// A self-signed certificate is generated in-process on every startup using [`rcgen`].
/// The Subject Alternative Names are set to `"embedded-http-server-rs"` and the
/// provided `ip` value. No client authentication is required.
///
/// > Self-signed certificates are not browser-trusted. This is suitable for
/// > development or internal tooling. For production, replace with a CA-issued cert.
///
/// # Routing
///
/// The macro dispatches requests using helpers from `utils::request::route`:
///
/// - [`utils::request::route::extract_path_from_request`] — parses the request path
/// - [`utils::request::route::extract_method_from_request`] — parses the HTTP method
/// - [`utils::request::route::get_matching_route_path`] — resolves a registered route pattern
/// - [`utils::request::route::get_route_function`] — returns the handler for `(route, method)`
///
/// Route handlers must have the signature:
///
/// They receive the full raw HTTP request string and must return a complete HTTP
/// response string (status line + headers + body).
///
/// # Limitations
///
/// - Requests are read into a fixed 4 KB buffer. Large bodies will be truncated.
/// - Each connection handles exactly one request (no keep-alive or pipelining).
/// - TLS handshake failures and write errors cause the spawned task to panic.
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
            if let Lit::Str(lit_str) = &nv.lit
                && nv.path.is_ident("ip")
            {
                ip_lit = Some(lit_str.value());
            } else if let Lit::Int(lit_int) = &nv.lit
                && nv.path.is_ident("port")
            {
                port_lit = Some(
                    lit_int
                        .base10_parse::<u16>()
                        .expect("Given invalid port number"),
                )
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

                    // The request target includes the query string; route patterns never do,
                    // so match against the path only.
                    let path_without_query = match path.split_once('?') {
                        Some((path_only, _)) => path_only,
                        None => path.as_str(),
                    };

                    let route_path = utils::request::route::get_matching_route_path(path_without_query);

                    let request_method =
                        utils::request::route::extract_method_from_request(&request);

                    let response = if let Some(route_path) = route_path
                        && let Ok(method) = request_method
                        && let Ok(Some(route_function)) = utils::request::route::get_route_function(&route_path, method)
                    {
                        route_function(&request)
                    } else {
                        format!(
                            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                            "Not Found".len(),
                            "Not Found"
                        )
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
