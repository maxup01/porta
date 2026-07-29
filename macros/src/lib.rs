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
/// - Requests are read in 4 KB chunks until the header block parses and the
///   `Content-Length` body has arrived. A request whose total size exceeds 1 MiB is
///   answered with `413 Payload Too Large`; a malformed header block or a body that
///   is not valid UTF-8 is answered with `400 Bad Request`.
/// - At most 512 connections are served at once. Further clients wait in the kernel
///   accept backlog until a slot frees. Peak memory is roughly
///   `512 * 3 * 1 MiB` — the request buffer, the copied body and the deserialized
///   value are live together — so the two limits are one decision, not two.
/// - Bodies must be valid UTF-8, so binary payloads have to be encoded (base64 in
///   JSON, say) rather than posted raw. There is no `multipart/form-data` support.
/// - `Transfer-Encoding: chunked` is not supported. A chunked request is dispatched
///   with an empty body rather than being rejected.
/// - A connection has 10 seconds to complete the TLS handshake and 30 seconds to
///   deliver a whole request. The request budget is a single deadline shared by the
///   header and body reads, so it cannot be extended by pacing the bytes; expiry is
///   answered with `408 Request Timeout`. Neither limit is configurable yet.
/// - Each connection handles exactly one request (no keep-alive or pipelining).
/// - Neither a failed TLS handshake nor a failed write panics. Both mean the peer is
///   unreachable, so the task drops that one connection and returns; the accept loop
///   keeps running.
/// - Every response is followed by a TLS `close_notify` via an explicit `shutdown`,
///   which flushes rustls' buffered records and marks the stream as ended on purpose
///   rather than truncated.
/// - Failing to bind the address at startup is fatal. Errors from `accept` are not:
///   they are retried after a short pause. Only the first failure in a run is logged,
///   with a second line on recovery reporting how many followed it — a sustained
///   failure such as file-descriptor exhaustion would otherwise emit the same message
///   a hundred times a second for as long as it lasted.
#[proc_macro_attribute]
pub fn http_server(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let input_fn = parse_macro_input!(item as ItemFn);
    let sig = &input_fn.sig;

    if sig.ident != "main" {
        panic!("The http_server macro can only be applied to the main function.");
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

            // Ceiling on connections served at once. Peak memory is roughly
            // MAX_CONNECTIONS * 3 * MAX_REQUEST_BYTES — the request buffer, the copied
            // body and the deserialized value are live together — so the two limits are
            // one decision rather than two.
            const MAX_CONNECTIONS: usize = 512;

            let connection_limit = ::std::sync::Arc::new(
                ::embedded_web_server::tokio::sync::Semaphore::new(MAX_CONNECTIONS)
            );

            let mut suppressed_accept_errors: u64 = 0;

            loop {
                // Taken before `accept`, not after. Accepting first spends the descriptor
                // regardless, which bounds memory but still walks into `EMFILE`; waiting
                // here leaves excess clients queued in the kernel backlog instead.
                //
                // On the accept-error path below this is dropped by `continue`, which
                // returns the slot.
                let permit = connection_limit
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("connection limit semaphore closed");

                let (socket, _) = match listener.accept().await {
                    Ok(connection) => {
                        if suppressed_accept_errors > 0 {
                            eprintln!(
                                "Accepting connections again after {} further failures",
                                suppressed_accept_errors
                            );

                            suppressed_accept_errors = 0;
                        }

                        connection
                    }
                    Err(error) => {
                        if suppressed_accept_errors == 0 {
                            eprintln!("Failed to accept connection: {}", error);
                        }

                        suppressed_accept_errors += 1;

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
                    // Bound to a name, not to `_`: `let _ = permit` would drop it here
                    // and release the slot immediately. Held like this it lives to the
                    // end of the task, so every exit path below returns it.
                    let _permit = permit;

                    // A peer that opens a socket and never sends a ClientHello would
                    // otherwise hold this task forever, which is the cheapest way to
                    // exhaust the process: it costs the client one socket and no data.
                    const HANDSHAKE_TIMEOUT: ::std::time::Duration =
                        ::std::time::Duration::from_secs(10);

                    let mut tls_stream = match ::embedded_web_server::tokio::time::timeout(
                        HANDSHAKE_TIMEOUT,
                        acceptor.accept(socket)
                    ).await {
                        Ok(Ok(tls_stream)) => tls_stream,
                        // A failed or abandoned handshake leaves no encrypted channel to
                        // answer over, so the connection is dropped without a response.
                        // This is a routine event on a public network — port scans, plain
                        // HTTP sent to the TLS port, version mismatches — and must not
                        // panic.
                        _ => return,
                    };

                    // Reading the request, enforcing the size ceiling and the deadline,
                    // routing it and writing the answer all live in `server`, as
                    // ordinary code this workspace can call and test. Emitting them
                    // here would put them in the caller's crate, where no test of ours
                    // can reach them.
                    ::embedded_web_server::server::handle_connection(
                        &mut tls_stream,
                        ::embedded_web_server::server::Limits::default()
                    ).await;
                });
            }
        }
    };

    expanded.into()
}
