# embedded-web-server

**Macro-driven, async embedded web server in Rust.**

`embedded-web-server` is a minimalist, async HTTP server designed to be embedded directly into your application. You define routes as plain functions, annotate them with attribute macros, and turn `main` into a running TLS server with a single attribute — no router setup, no boilerplate.

## Features

- **Attribute-macro routing** — `#[get]`, `#[post]`, `#[put]`, `#[patch]`, `#[delete]` register handlers at compile time.
- **Zero-boilerplate startup** — `#[http_server]` rewrites `main` into a full async TLS server.
- **Automatic parameter binding** — path params, query params, and JSON bodies are parsed and deserialized straight into your function arguments.
- **Typed responses** — return an `HttpResponse<T>` with a strongly-typed `HttpStatus`; serialization and HTTP formatting are handled for you.
- **TLS by default** — every connection is served over `rustls` with an in-process self-signed certificate.
- **Bounded by construction** — request size, request duration and connection count all have ceilings, and exceeding one produces a proper HTTP status rather than unbounded growth.
- **Built on Tokio** — connections are accepted and handled concurrently.

## Quick start

Add the crate to your `Cargo.toml`, then:

```rust
use embedded_web_server::*;
use serde::{Deserialize, Serialize};

#[get(path = "/")]
fn index() -> HttpResponse<String> {
    HttpResponse::new("Hello, world!".to_string(), HttpStatus::Ok)
}

#[get(path = "/hello/{name}")]
fn hello(name: String) -> HttpResponse<String> {
    HttpResponse::new(format!("Hello, {}!", name), HttpStatus::Ok)
}

#[derive(Serialize, Deserialize)]
struct RandomStruct {
    pub num: u64,
    pub name: String,
}

#[post(path = "/something/{id}")]
fn something(id: u64, body: RandomStruct) -> HttpResponse<String> {
    HttpResponse::new(
        format!("Received id: {} num: {} name: {}", id, body.num, body.name),
        HttpStatus::Ok,
    )
}

#[http_server(ip = "127.0.0.1", port = 8443)]
async fn main() {}
```

Run it:

```bash
cargo run
```

The server listens on `https://127.0.0.1:8443`. Because the certificate is self-signed, clients must skip verification — e.g. `curl -k`:

```bash
curl -k https://127.0.0.1:8443/
curl -k https://127.0.0.1:8443/hello/world
curl -k -X POST https://127.0.0.1:8443/something/42 \
     -d '{"num": 7, "name": "abc"}'
```

## Public API

`use embedded_web_server::*;` brings in everything you need: the six attribute macros, plus `HttpResponse` and `HttpStatus`.

That is the entire surface. The crate also re-exports `tokio`, `rustls`, `rcgen`, `tokio-rustls`, `ctor`, `serde_json` and the internal `utils` and `server` crates, but all of them are `#[doc(hidden)]` — they exist only so that macro-generated code has somewhere to resolve. They are not covered by this crate's semantic versioning, and naming them by hand means opting out of that guarantee.

## Routing macros

Each macro takes a required `path` argument and registers the annotated function as a handler for that method and path. Paths support named segments with `{name}` syntax.

| Macro | Method | Example |
|-------|--------|---------|
| `#[get(path = "...")]`    | GET    | `#[get(path = "/users/{id}")]` |
| `#[post(path = "...")]`   | POST   | `#[post(path = "/users")]` |
| `#[put(path = "...")]`    | PUT    | `#[put(path = "/users/{id}")]` |
| `#[patch(path = "...")]`  | PATCH  | `#[patch(path = "/users/{id}")]` |
| `#[delete(path = "...")]` | DELETE | `#[delete(path = "/users/{id}")]` |

`HEAD` and `OPTIONS` are not supported.

### Parameter binding

Function arguments are filled automatically by name:

- **Path parameters** (`{id}`) are matched from the URL.
- **Query parameters** (`?key=value`) are merged in for `GET` and `DELETE`.
- **Request body** is bound to the single non-path argument for `POST`, `PUT` and `PATCH`.

Query parameters are merged *after* path parameters, so when both supply a name, the query string wins: `GET /users/7?id=9` binds `id = 9`.

Arguments are deserialized into their declared Rust type:

- Numeric types (`u8`–`u64`, `i8`–`i64`, `usize`, `isize`, `f32`, `f64`) are parsed directly.
- `bool` accepts `true`/`1` and `false`/`0`.
- `String` is taken verbatim.
- Any other type is deserialized via `serde_json` (so it must derive `Deserialize`). Values that are not already JSON objects are wrapped in quotes first, so string-backed enums and newtypes work — but JSON arrays as a top-level body do not.

A parameter that is **missing** returns `400 Bad Request`. A parameter that is **present but unparseable** returns `404 Not Found`.

Values are taken verbatim: percent-encoding is not decoded, so `/hello/John%20Doe` binds the literal `John%20Doe`.

### Route resolution

Each method owns its own routing table, so a route registered for one verb can never resolve a request made with another.

When several patterns match, the most specific wins — the one with the fewest `{param}` segments. `/users/me` beats `/users/{id}`. Two patterns of equal specificity that differ only in parameter name are a routing ambiguity on your part, and resolve arbitrarily.

The dispatcher answers in this order:

| Situation | Response |
|-----------|----------|
| A handler matches the path and method | the handler's response |
| The path is served, but by another method | `405 Method Not Allowed` |
| No method serves the path | `404 Not Found` |
| The request line could not be parsed | `400 Bad Request` |

The `405` does not carry an `Allow` header.

## Responses

`format_response` turns a handler's `HttpResponse<T>` into the wire format:

```text
HTTP/1.1 <code> <reason>
Content-Type: application/json
Content-Length: <byte_length>
Date: <rfc-date>
Connection: close

<json_body>
```

`Connection: close` is sent on every response because a connection serves exactly one request.

A `204 No Content` response is the exception: the body is discarded and both content headers are omitted, as RFC 9110 requires.

## The `#[http_server]` macro

Applied to `main`, this macro generates a complete async server. At startup it:

1. Generates a self-signed TLS certificate in-process with [`rcgen`].
2. Builds a `rustls` server config and a `tokio_rustls` TLS acceptor.
3. Binds a `TcpListener` to the given `ip:port`.
4. Accepts connections concurrently, performs the TLS handshake, and hands each stream to `server::handle_connection`, which reads the request, dispatches it and writes the answer.

```rust
#[http_server(ip = "127.0.0.1", port = 8443)]
async fn main() {}
```

Both `ip` (string) and `port` (integer) are required. The macro must be applied to a function named `main`.

## Limits

| Limit | Value | Exceeded |
|-------|-------|----------|
| Request size (headers + body) | 1 MiB | `413 Payload Too Large` |
| Request duration | 30 s | `408 Request Timeout` |
| TLS handshake duration | 10 s | connection dropped |
| Concurrent connections | 512 | client waits in the accept backlog |
| Headers per request | 32 | `400 Bad Request` |

The request deadline is a single budget shared by the header and body reads, so it cannot be extended by pacing bytes slowly.

Peak memory is roughly `connections × 3 × request size` — the request buffer, the copied body and the deserialized value are all live at once — so the size and connection limits are one decision rather than two.

These are currently compile-time constants in `server::Limits`; the macro passes `Limits::default()` and does not yet expose them as attribute arguments.

## Security & limitations

This server is intended to run as an **internal service behind a reverse proxy** (e.g. nginx, Caddy, Traefik, or a cloud load balancer). The proxy is expected to terminate public TLS with a CA-issued certificate and handle concerns like authentication, rate limiting, request buffering, and keep-alive — the embedded server itself stays minimal and trusts that it sits on a private network.

The server is **TLS-only**. Encryption is always on, but the certificate is **self-signed and regenerated on every startup**, with Subject Alternative Names set to `"embedded-http-server-rs"` and the provided `ip`. Connections are encrypted, but the cert is **not browser-trusted** — this is by design. The self-signed cert secures the hop between the reverse proxy and this server on a trusted internal network; the proxy presents a CA-issued certificate to the outside world. The server is not meant to be exposed directly to the public internet.

Other constraints worth knowing:

- **Request bodies must be valid UTF-8.** Binary payloads are rejected with `400 Bad Request`, so files have to be encoded — base64 inside JSON, for instance. There is no `multipart/form-data` support and no streaming; the whole request is buffered in memory.
- **`Transfer-Encoding: chunked` is not supported.** A chunked request is dispatched with an empty body rather than rejected.
- **Each connection handles exactly one request** — no keep-alive, no pipelining. Bytes arriving after `Content-Length` is satisfied are discarded.
- **Handlers are synchronous** and cannot return `Result`. Blocking work inside one occupies a runtime worker.
- **A panicking handler** drops its connection without a response.
- Every response is followed by an explicit TLS `close_notify`, so peers can distinguish a normal end of stream from a truncated one.
- Failed TLS handshakes and failed writes do not panic; the connection is dropped and the accept loop continues. Failing to bind the address at startup is fatal.
- Sustained `accept` failure — file-descriptor exhaustion, say — is logged once at onset and once on recovery with a count, rather than on every retry.

## Project layout

```
.
├── src/            # crate re-exports and public API
├── macros/         # proc-macro crate: #[get], #[post], #[http_server], etc.
├── server/         # connection handling: read loop, limits, dispatch
├── utils/          # request parsing, routing table, response formatting
├── error/          # shared error types
└── tests/          # integration tests
```

`server` exists as its own crate rather than as code inside `#[http_server]`'s expansion so that the read loop, the limits and the dispatch decision are ordinary functions this workspace can test. It is generic over `AsyncRead + AsyncWrite`, so tests drive it over an in-memory pipe while the macro passes a TLS stream.

## License

See [LICENSE](./LICENSE).
