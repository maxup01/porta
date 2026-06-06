# embedded-web-server

**Macro-driven, async embedded web server in Rust.**

`embedded-web-server` is a minimalist, async HTTP server designed to be embedded directly into your application. You define routes as plain functions, annotate them with attribute macros, and turn `main` into a running TLS server with a single attribute — no router setup, no boilerplate.

## Features

- **Attribute-macro routing** — `#[get]`, `#[post]`, `#[patch]`, `#[delete]` register handlers at compile time.
- **Zero-boilerplate startup** — `#[http_server]` rewrites `main` into a full async TLS server.
- **Automatic parameter binding** — path params, query params, and JSON bodies are parsed and deserialized straight into your function arguments.
- **Typed responses** — return an `HttpResponse<T>` with a strongly-typed `HttpStatus`; serialization and HTTP formatting are handled for you.
- **TLS by default** — every connection is served over `rustls` with an in-process self-signed certificate.
- **Built on Tokio** — connections are accepted and handled concurrently.

## Quick start

Add the crate to your `Cargo.toml`, then:

```rust
use embedded_web_server::*;
use embedded_web_server::response::{HttpResponse, HttpStatus};
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

## Routing macros

Each macro takes a required `path` argument and registers the annotated function as a handler for that method and path. Paths support named segments with `{name}` syntax.

| Macro | Method | Example |
|-------|--------|---------|
| `#[get(path = "...")]`    | GET    | `#[get(path = "/users/{id}")]` |
| `#[post(path = "...")]`   | POST   | `#[post(path = "/users")]` |
| `#[patch(path = "...")]`  | PATCH  | `#[patch(path = "/users/{id}")]` |
| `#[delete(path = "...")]` | DELETE | `#[delete(path = "/users/{id}")]` |

### Parameter binding

Function arguments are filled automatically by name:

- **Path parameters** (`{id}`) are matched from the URL.
- **Query parameters** (`?key=value`) are merged in for `GET` and `DELETE`.
- **Request body** is bound to the single non-path argument for `POST` and `PATCH`.

Arguments are deserialized into their declared Rust type:

- Numeric types (`u8`–`u64`, `i8`–`i64`, `usize`, `isize`, `f32`, `f64`) are parsed directly.
- `bool` accepts `true`/`1` and `false`/`0`.
- `String` is taken verbatim.
- Any other type is deserialized via `serde_json` (so it must derive `Deserialize`).

If a value fails to parse or deserialize, the handler returns `404 Not Found`.

## The `#[http_server]` macro

Applied to `main`, this macro generates a complete async server. At startup it:

1. Generates a self-signed TLS certificate in-process with [`rcgen`].
2. Builds a `rustls` server config and a `tokio_rustls` TLS acceptor.
3. Binds a `TcpListener` to the given `ip:port`.
4. Accepts connections concurrently, performs the TLS handshake, parses each request, and dispatches it to the matching registered handler — returning `404 Not Found` when no route matches.

```rust
#[http_server(ip = "127.0.0.1", port = 8443)]
async fn main() {}
```

Both `ip` (string) and `port` (integer) are required. The macro must be applied to a function named `main`.

## Security & limitations

This server is intended to run as an **internal service behind a reverse proxy** (e.g. nginx, Caddy, Traefik, or a cloud load balancer). The proxy is expected to terminate public TLS with a CA-issued certificate and handle concerns like authentication, rate limiting, request buffering, and keep-alive — the embedded server itself stays minimal and trusts that it sits on a private network.

The server is **TLS-only**. Encryption is always on, but the certificate is **self-signed and regenerated on every startup**, with Subject Alternative Names set to `"embedded-http-server-rs"` and the provided `ip`. This means:

- Connections are encrypted, but the cert is **not browser-trusted** — this is by design. The self-signed cert secures the hop between the reverse proxy and this server on a trusted internal network; the proxy presents a CA-issued certificate to the outside world. The server is not meant to be exposed directly to the public internet.
- Requests are read into a fixed **4 KB buffer** — larger request bodies are truncated.
- Each connection handles **exactly one request** (no keep-alive or pipelining).
- TLS handshake and write failures cause the spawned connection task to panic.

## Project layout

```
.
├── src/            # crate re-exports
├── macros/         # proc-macro crate: #[get], #[post], #[http_server], etc.
├── utils/          # request parsing, routing table, response formatting
├── error/          # shared error types
└── tests/          # integration tests
```

## License

See [LICENSE](./LICENSE).
