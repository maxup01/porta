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
/// - [`utils::request::route::extract_path_from_request`] — parses the request target
/// - [`utils::request::route::extract_method_from_request`] — parses the HTTP method
/// - [`utils::request::route::get_route_function`] — resolves the handler for `(path, method)`
/// - [`utils::request::route::path_exists`] — on a miss, decides `405` versus `404`
///
/// The query string is stripped before matching, since route patterns never carry one.
/// A path served by some other method yields `405 Method Not Allowed`; a path served by
/// no method at all yields `404 Not Found`.
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
///   This drops that one connection; the accept loop keeps running.
/// - Failing to bind the address at startup is fatal. Errors from `accept` are not:
///   they are logged to stderr and retried after a short pause.
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
        // Every path below is absolute and routed through `embedded_web_server`, because
        // this code is expanded into the *caller's* crate. The caller depends only on
        // `embedded_web_server`, not on tokio/rustls/rcgen/ctor/serde_json directly, so
        // any bare path here would fail to resolve downstream. See `embedded_web_server`'s
        // crate-root re-exports.
        #[::embedded_web_server::tokio::main(crate = "::embedded_web_server::tokio")]
        #sig {
            // Trait methods can't be called through an absolute path, so these two are
            // imported at function scope rather than into the caller's module.
            use ::embedded_web_server::tokio::io::{AsyncReadExt, AsyncWriteExt};

            let subject_alt_names = vec!["embedded-http-server-rs".to_string(), #ip_str.to_string()];
            let cert = ::embedded_web_server::rcgen::generate_simple_self_signed(subject_alt_names)
                .expect("Failed to generate self-signed certificate");

            let cert_der = ::embedded_web_server::rustls::pki_types::CertificateDer::from(
                cert.cert.der().to_vec()
            );
            let key_der = ::embedded_web_server::rustls::pki_types::PrivateKeyDer::Pkcs8(
                ::embedded_web_server::rustls::pki_types::PrivatePkcs8KeyDer::from(
                    cert.key_pair.serialize_der()
                )
            );

            let server_config = ::embedded_web_server::rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![cert_der], key_der)
                .expect("Failed to initialize server config");

            let acceptor = ::embedded_web_server::tokio_rustls::TlsAcceptor::from(
                ::std::sync::Arc::new(server_config)
            );

            let addr = format!("{}:{}", #ip_str, #port);
            let listener = ::embedded_web_server::tokio::net::TcpListener::bind(&addr)
                .await
                .expect("Failed to bind address");

            loop {
                let (socket, _) = match listener.accept().await {
                    Ok(connection) => connection,
                    Err(error) => {
                        eprintln!("Failed to accept connection: {}", error);

                        // On descriptor exhaustion the refused connection stays in the
                        // backlog, so the socket reports readable again immediately and
                        // an unpaused retry would spin a core until an fd frees.
                        ::embedded_web_server::tokio::time::sleep(
                            ::std::time::Duration::from_millis(10)
                        ).await;

                        continue;
                    }
                };

                let acceptor = acceptor.clone();

                ::embedded_web_server::tokio::spawn(async move {
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

                    let response = match ::embedded_web_server::utils::request::route::extract_path_from_request(&request) {
                        Err(_) => format!(
                            "HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                            "Bad Request".len(),
                            "Bad Request"
                        ),
                        Ok(path) => {
                            // The request target includes the query string; route patterns
                            // never do, so match against the path only.
                            let path_without_query = match path.split_once('?') {
                                Some((path_only, _)) => path_only,
                                None => path.as_str(),
                            };

                            let route_function = match ::embedded_web_server::utils::request::route::extract_method_from_request(
                                &request
                            ) {
                                Ok(method) => {
                                    ::embedded_web_server::utils::request::route::get_route_function(
                                        path_without_query,
                                        method
                                    )
                                    .ok()
                                    .flatten()
                                },
                                Err(_) => None,
                            };

                            if let Some(route_function) = route_function {
                                route_function(&request)
                            } else if ::embedded_web_server::utils::request::route::path_exists(path_without_query) {
                                format!(
                                    "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                                    "Method Not Allowed".len(),
                                    "Method Not Allowed"
                                )
                            } else {
                                format!(
                                    "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                                    "Not Found".len(),
                                    "Not Found"
                                )
                            }
                        }
                    };

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
