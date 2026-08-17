extern crate proc_macro;

mod macro_utils;

use macro_utils::*;
use quote::{format_ident, quote, quote_spanned};
use syn::{AttributeArgs, ItemFn, ItemStruct, parse_macro_input};
use utils::request::route::Method;

/// Registers the annotated function as a handler for HTTP `GET` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/hello"` or `"/users/:id"`)
///
/// The `path` argument is required. Omitting it is a compile error at the attribute.
///
/// # See also
///
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`require_route_path`] — extracts the `path` value, or reports its absence
#[proc_macro_attribute]
pub fn get(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = match require_route_path(&args, Method::GET) {
        Ok(path) => path,
        Err(error) => return error.to_compile_error().into(),
    };

    let expanded = generate_route_handler_tokens(&path, Method::GET, &input_fn);

    expanded.into()
}

/// Registers the annotated function as a handler for HTTP `DELETE` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/users/:id"`)
///
/// The `path` argument is required. Omitting it is a compile error at the attribute.
///
/// # See also
///
/// - [`get`] — the equivalent macro for `GET` requests
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`require_route_path`] — extracts the `path` value, or reports its absence
#[proc_macro_attribute]
pub fn delete(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = match require_route_path(&args, Method::DELETE) {
        Ok(path) => path,
        Err(error) => return error.to_compile_error().into(),
    };

    let expanded = generate_route_handler_tokens(&path, Method::DELETE, &input_fn);

    expanded.into()
}

/// Registers the annotated function as a handler for HTTP `POST` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/users"`)
///
/// The `path` argument is required. Omitting it is a compile error at the attribute.
///
/// # See also
///
/// - [`get`] — the equivalent macro for `GET` requests
/// - [`delete`] — the equivalent macro for `DELETE` requests
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`require_route_path`] — extracts the `path` value, or reports its absence
#[proc_macro_attribute]
pub fn post(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = match require_route_path(&args, Method::POST) {
        Ok(path) => path,
        Err(error) => return error.to_compile_error().into(),
    };

    let expanded = generate_route_handler_tokens(&path, Method::POST, &input_fn);

    expanded.into()
}

/// Registers the annotated function as a handler for HTTP `PUT` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/users/{id}"`)
///
/// The `path` argument is required. Omitting it is a compile error at the attribute.
///
/// # Body binding
///
/// Like [`post`] and [`patch`], the request body is bound to the single argument that
/// is not a path parameter. `PUT` replaces a resource outright where `PATCH` amends
/// one; the difference is semantic, and this crate binds both identically.
///
/// # See also
///
/// - [`patch`] — the equivalent macro for partial updates
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`require_route_path`] — extracts the `path` value, or reports its absence
#[proc_macro_attribute]
pub fn put(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = match require_route_path(&args, Method::PUT) {
        Ok(path) => path,
        Err(error) => return error.to_compile_error().into(),
    };

    let expanded = generate_route_handler_tokens(&path, Method::PUT, &input_fn);

    expanded.into()
}

/// Registers the annotated function as a handler for HTTP `PATCH` requests at the given path.
///
/// # Arguments
///
/// - `path` — the route path to match, as a string literal (e.g. `"/users/:id"`)
///
/// The `path` argument is required. Omitting it is a compile error at the attribute.
///
/// # See also
///
/// - [`get`] — the equivalent macro for `GET` requests
/// - [`post`] — the equivalent macro for `POST` requests
/// - [`delete`] — the equivalent macro for `DELETE` requests
/// - [`generate_route_handler_tokens`] — generates the registration code for the resolved handler
/// - [`require_route_path`] — extracts the `path` value, or reports its absence
#[proc_macro_attribute]
pub fn patch(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let path = match require_route_path(&args, Method::PATCH) {
        Ok(path) => path,
        Err(error) => return error.to_compile_error().into(),
    };

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
/// - `allow_origins` — optional list of origins permitted to call this server from
///   a browser, as an array of string literals (e.g.
///   `["http://localhost:1420"]`), or `["*"]` for any origin
///
/// `ip` and `port` are required; omitting either is a compile error, as is an
/// unrecognised argument name.
///
/// # Constraints
///
/// This macro **must** be applied to a function named `main`. Applying it to any
/// other function name causes a compile-time panic.
///
/// # CORS
///
/// Without `allow_origins` the server emits no cross-origin headers at all, which
/// is correct for a backend behind a reverse proxy and for any non-browser client:
/// nothing enforces CORS but a browser, and a request with no `Origin` header is
/// answered byte for byte as it was before this argument existed.
///
/// With it, two things change. `OPTIONS` — which is not a routable [`Method`] and
/// used to fall through to `404` — is answered by the server itself, as a
/// preflight when the request carries `Origin` and `Access-Control-Request-Method`
/// and the origin is listed, and otherwise as a plain `204` naming what the path
/// serves in an `Allow` header. And every response to a request from a listed
/// origin, handler-generated or not, carries `Access-Control-Allow-Origin`.
///
/// Both halves are necessary. A browser making any request that is not [simple] —
/// which includes every `fetch` with a JSON body — sends the preflight first and
/// will not send the real request until it is answered; then it checks the real
/// response's headers before letting the calling script read it.
///
/// Origins are matched in full and compared case-insensitively: scheme, host and
/// port, with no trailing slash. `http://localhost:1420` and
/// `http://127.0.0.1:1420` are different origins, and a page served from one is
/// not served from the other.
///
/// ```ignore
/// #[http_server(
///     ip = "127.0.0.1",
///     port = 8443,
///     allow_origins = ["http://localhost:1420", "tauri://localhost"]
/// )]
/// async fn main() {}
/// ```
///
/// Credentials are not enabled, and requested headers are echoed back on a
/// preflight. See [`utils::cors::CorsConfig`] for what the policy can express
/// beyond what this attribute currently exposes.
///
/// [simple]: https://developer.mozilla.org/docs/Glossary/CORS-safelisted_request_header
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
///   deliver a whole request, measured from that request's first byte. The request
///   budget is a single deadline shared by the header and body reads, so it cannot
///   be extended by pacing the bytes; expiry is answered with `408 Request Timeout`.
///   None of these limits is configurable yet.
/// - Connections are persistent, as HTTP/1.1 requires. One connection serves up to
///   100 requests and may sit idle for 15 seconds between them, so a client that
///   makes several requests pays one TLS handshake rather than one each. Pipelined
///   requests are answered in order. The connection ends when the client asks it to,
///   when it goes quiet, when either bound is reached, or when a request is refused
///   — and the last response carries `Connection: close` so the client learns of it
///   from the answer rather than from a failed write.
/// - Persistence has a cost the connection ceiling has to absorb: a slot is held for
///   as long as the connection lives, not just while a request is in flight, so the
///   idle timeout is what keeps quiet peers from crowding out live ones.
/// - Neither a failed TLS handshake nor a failed write panics. Both mean the peer is
///   unreachable, so the task drops that one connection and returns; the accept loop
///   keeps running.
/// - A closed connection is followed by a TLS `close_notify` via an explicit
///   `shutdown`, which flushes rustls' buffered records and marks the stream as ended
///   on purpose rather than truncated.
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
    let args = parse_macro_input!(attr as HttpServerArgs);
    let input_fn = parse_macro_input!(item as ItemFn);
    let sig = &input_fn.sig;

    if sig.ident != "main" {
        panic!("The http_server macro can only be applied to the main function.");
    }

    let ip_str = args.ip;
    let port = args.port;
    let allow_origins = args.allow_origins;

    let expanded = quote! {


        #[derive(Default)]
        pub struct AppContext {}

        static APP_CONTEXT: ::std::sync::LazyLock<AppContext> = ::std::sync::LazyLock::new(|| AppContext::default());

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

            // Built once and shared: the policy is the same for every connection,
            // and an `Arc` keeps a per-connection clone to a refcount bump rather
            // than a copy of the origin list. An empty list yields a disabled
            // policy, which adds no header to anything.
            let cors = ::std::sync::Arc::new(
                ::embedded_web_server::utils::cors::CorsConfig::new(
                    &[#(#allow_origins),*],
                    false,
                    ::std::option::Option::None,
                )
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
                let cors = ::std::sync::Arc::clone(&cors);

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
                        ::embedded_web_server::server::Limits::default(),
                        &cors
                    ).await;
                });
            }
        }
    };

    expanded.into()
}

/// Makes the annotated struct a process-wide singleton that any handler can ask for.
///
/// Every argument the routing macros bind comes out of the request. A database handle,
/// a cache or a counter does not — it belongs to the process and outlives any one
/// request. This attribute is the seam for those: it generates an accessor on
/// `AppContext` handing out one shared instance, and the routing macros bind that
/// instance to any parameter marked `#[component]`.
///
/// # Arguments
///
/// - `name` — the accessor generated on `AppContext`, as a string literal
///   (e.g. `name = "tickets"` produces `AppContext::tickets()`)
///
/// `name` is required. It is not derived from the type, because `TicketHandler` →
/// `ticket_handler` has no answer a caller would predict for `HTTPHandler` or
/// `TicketDb`. A value that could not be an identifier, or that is a reserved word,
/// is a compile error at the literal rather than a panic — see [`ComponentArgs`].
///
/// # Usage
///
/// The attribute is one half; the parameter marker is the other.
///
/// ```ignore
/// #[component(name = "tickets")]
/// #[derive(Default)]
/// struct TicketStore {
///     rows: std::sync::Mutex<Vec<Ticket>>,
/// }
///
/// #[get(path = "/tickets/{id}")]
/// fn get_ticket(id: u64, #[component] tickets: &TicketStore) -> HttpResponse<String> {
///     ...
/// }
/// ```
///
/// `id` is still bound from the path. `tickets` is removed from the request parameters
/// before any of that happens, so it is never looked for in the URL or the body — see
/// [`split_component_args`]. The two may appear in either order.
///
/// The parameter *name* is what selects the component, not its type: the expansion is
/// `let tickets = AppContext::tickets();`, because a proc macro sees tokens and cannot
/// work out which type an identifier refers to. The type you write is checked against
/// what the accessor returns, so a mismatched pair fails to compile.
///
/// # Constraints
///
/// - **[`Default`] is required.** `Default::default()` is what constructs the shared
///   instance; nothing calls a component into being, so the only value a singleton can
///   have is one a no-argument function produces. Setup work belongs there. It runs
///   once, on whichever request arrives first, and a panic inside it poisons the
///   component for every later access.
/// - **The type must be [`Sync`].** One instance is shared by every connection, and
///   connections run concurrently.
/// - **The parameter must be `&T`.** The accessor returns `&'static T`; `&mut T` and
///   by-value are both rejected with a message saying so. Put a `Mutex`, `RwLock` or an
///   atomic on the field that needs writing. There is deliberately no switch that wraps
///   the whole struct in a lock — per-field locking keeps the granularity visible, and
///   a handler asking for two components cannot deadlock on an ordering chosen for it.
/// - **The type must not be generic.** A `static` needs one concrete type and nothing
///   here says which instantiation was meant — see [`reject_generic_component`].
///
/// # Placement
///
/// `AppContext` is generated by [`http_server`], so components live in the same crate
/// as `main`. Handlers never name it; the generated code does.
///
/// # See also
///
/// - [`http_server`] — generates the `AppContext` these accessors are implemented on
/// - [`ComponentArgs`] — parses and validates `name`
/// - [`split_component_args`] — takes component parameters out of a handler's signature
#[proc_macro_attribute]
pub fn component(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(attr as ComponentArgs);
    let input_struct = parse_macro_input!(item as ItemStruct);

    if let Err(error) = reject_generic_component(&input_struct) {
        let error = error.to_compile_error();

        return quote! {
            #error
            #input_struct
        }
        .into();
    }

    let component_type = &input_struct.ident;
    let accessor = format_ident!("{}", args.name);
    let vis = &input_struct.vis;

    let component_struct_span = input_struct.ident.span();

    let app_context_access_function_block = quote_spanned! {component_struct_span =>
        impl crate::AppContext {
            #vis fn #accessor() -> &'static #component_type {
                static INSTANCE: ::std::sync::LazyLock<#component_type> = ::std::sync::LazyLock::new(
                    <#component_type as ::core::default::Default>::default
                );

                &*INSTANCE
            }
        }
    };

    quote! {
        #input_struct

        #app_context_access_function_block

        const _: () = {
            fn assert_sync<T: ::core::marker::Sync>() {}
            let _ = assert_sync::<#component_type>;
        };
    }
    .into()
}
