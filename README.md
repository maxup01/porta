# porta

**Macro-driven, async web server in Rust that you embed in your own application.**

`porta` is a minimalist, async HTTP server designed to be embedded directly into your application. You define routes as plain functions, annotate them with attribute macros, and turn `main` into a running TLS server with a single attribute — no router setup, no boilerplate.

## Features

- **Attribute-macro routing** — `#[get]`, `#[post]`, `#[put]`, `#[patch]`, `#[delete]` register handlers at compile time.
- **Zero-boilerplate startup** — `#[http_server]` rewrites `main` into a full async TLS server.
- **Automatic parameter binding** — path params, query params, and JSON bodies are parsed and deserialized straight into your function arguments.
- **Typed responses** — return an `HttpResponse<T>` with a strongly-typed `HttpStatus`; serialization and HTTP formatting are handled for you.
- **Shared state without plumbing** — `#[component]` makes a struct a process-wide singleton that any handler can ask for by adding a parameter.
- **CORS when you want it** — name the origins in `#[http_server]` and preflights are answered and headers attached; name none and nothing changes.
- **TLS by default** — every connection is served over `rustls` with an in-process self-signed certificate.
- **Bounded by construction** — request size, request duration and connection count all have ceilings, and exceeding one produces a proper HTTP status rather than unbounded growth.
- **Built on Tokio** — connections are accepted and handled concurrently.

## Quick start

Add the crate to your `Cargo.toml`, then:

```rust
use porta::*;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[get(path = "/")]
async fn index() -> HttpResponse<String> {
    HttpResponse::new("Hello, world!".to_string(), HttpStatus::Ok)
}

#[get(path = "/hello/{name}")]
async fn hello(name: String) -> HttpResponse<String> {
    HttpResponse::new(format!("Hello, {}!", name), HttpStatus::Ok)
}

#[derive(Serialize, Deserialize)]
struct RandomStruct {
    pub num: u64,
    pub name: String,
}

#[post(path = "/something/{id}")]
async fn something(id: u64, body: RandomStruct) -> HttpResponse<String> {
    HttpResponse::new(
        format!("Received id: {} num: {} name: {}", id, body.num, body.name),
        HttpStatus::Ok,
    )
}

// One instance for the whole process, reachable from any handler as
// `AppContext::counter()`. The parameter below is what asks for it.
//
// `Default` is required: `Counter::default()` is what builds that instance,
// once, the first time a handler asks.
#[component(name = "counter")]
#[derive(Default)]
struct Counter {
    hits: Mutex<u64>,
}

#[get(path = "/hits")]
async fn hits(#[component] counter: &Counter) -> HttpResponse<u64> {
    let mut hits = counter.hits.lock().unwrap();
    *hits += 1;

    HttpResponse::new(*hits, HttpStatus::Ok)
}

#[http_server(
    ip = "127.0.0.1",
    port = 8443,
    allow_origins = ["http://localhost:1420"]
)]
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
curl -k https://127.0.0.1:8443/hits   # 1, then 2, then 3 …
```

`allow_origins` is what lets a browser on `http://localhost:1420` read those answers; drop it and the server behaves identically for everything that is not a browser. See [CORS](#cors).

## Public API

`use porta::*;` brings in everything you need: the seven attribute macros, plus `HttpResponse` and `HttpStatus`.

That is the entire surface. The crate also re-exports `tokio`, `rustls`, `rcgen`, `tokio-rustls`, `ctor`, `serde_json` and the internal `utils` and `server` crates, but all of them are `#[doc(hidden)]` — they exist only so that macro-generated code has somewhere to resolve. They are not covered by this crate's semantic versioning, and naming them by hand means opting out of that guarantee.

## Routing macros

Each macro takes a required `path` argument and registers the annotated function as a handler for that method and path. Paths support named segments with `{name}` syntax.

| Macro                     | Method | Example                           |
| ------------------------- | ------ | --------------------------------- |
| `#[get(path = "...")]`    | GET    | `#[get(path = "/users/{id}")]`    |
| `#[post(path = "...")]`   | POST   | `#[post(path = "/users")]`        |
| `#[put(path = "...")]`    | PUT    | `#[put(path = "/users/{id}")]`    |
| `#[patch(path = "...")]`  | PATCH  | `#[patch(path = "/users/{id}")]`  |
| `#[delete(path = "...")]` | DELETE | `#[delete(path = "/users/{id}")]` |

`HEAD` is not supported. `OPTIONS` has no macro because it is not routed: the server answers it itself, from the route table — see [CORS](#cors).

### Parameter binding

Function arguments are filled automatically by name:

- **Path parameters** (`{id}`) are matched from the URL.
- **Query parameters** (`?key=value`) are merged in for `GET` and `DELETE`.
- **Request body** is bound to the single non-path argument for `POST`, `PUT` and `PATCH`.

An argument marked `#[component]` is exempt from all of the above — it comes from the process, not the request, and is removed before any of these rules are applied. See [Shared state](#shared-state).

Query parameters are merged _after_ path parameters, so when both supply a name, the query string wins: `GET /users/7?id=9` binds `id = 9`.

Arguments are deserialized into their declared Rust type:

- Numeric types (`u8`–`u64`, `i8`–`i64`, `usize`, `isize`, `f32`, `f64`) are parsed directly.
- `bool` accepts `true`/`1` and `false`/`0`.
- `String` is taken verbatim.
- Any other type is deserialized via `serde_json` (so it must derive `Deserialize`). Values that are not already JSON objects are wrapped in quotes first, so string-backed enums and newtypes work — but JSON arrays as a top-level body do not.

A parameter that is **missing** returns `400 Bad Request`.

Values are taken verbatim: percent-encoding is not decoded, so `/hello/John%20Doe` binds the literal `John%20Doe`.

### Route resolution

Each method owns its own routing table, so a route registered for one verb can never resolve a request made with another.

When several patterns match, the most specific wins — the one with the fewest `{param}` segments. `/users/me` beats `/users/{id}`. Two patterns of equal specificity that differ only in parameter name are a routing ambiguity on your part, and resolve arbitrarily.

The dispatcher answers in this order:

| Situation                                       | Response                           |
| ----------------------------------------------- | ---------------------------------- |
| A handler matches the path and method           | the handler's response             |
| The request is `OPTIONS` and the path is served | `204 No Content` + `Allow`         |
| The path is served, but by another method       | `405 Method Not Allowed` + `Allow` |
| No method serves the path                       | `404 Not Found`                    |
| The request line could not be parsed            | `400 Bad Request`                  |

`Allow` lists every method that serves the path, plus `OPTIONS`.

## Responses

`format_response` turns a handler's `HttpResponse<T>` into the wire format:

```text
HTTP/1.1 <code> <reason>
Content-Type: application/json
Content-Length: <byte_length>
Date: <rfc-date>

<json_body>
```

No `Connection` header appears, because HTTP/1.1 connections are persistent unless a response says otherwise. `Connection: close` is added only to the last response on a connection — see [Connections](#connections).

A `204 No Content` response is the exception: the body is discarded and both content headers are omitted, as RFC 9110 requires.

## Connections

A connection serves as many requests as the client wants to send, so a client making several requests pays one TLS handshake rather than one per request. Requests pipelined without waiting for the previous answer are answered in order.

The connection ends when any of these happens, and the response before it carries `Connection: close`:

| Cause                                  | Bound                                                             |
| -------------------------------------- | ----------------------------------------------------------------- |
| The client asked to close              | `Connection: close`, or HTTP/1.0 without `Connection: keep-alive` |
| The client went quiet between requests | 15 seconds                                                        |
| The connection served its budget       | 100 requests                                                      |
| A request was refused                  | `400`, `408`, `413`                                               |

A refused request ends the connection because nothing after it can be framed: an oversized body is still arriving, and a request that did not parse has no length to skip past. A `404` or `405` is not a refusal — the request was well-formed, so the connection stays open.

An idle timeout produces no response at all. The client is very likely closing the connection at the same moment, and there is no fault to report.

Persistence is not free: a connection holds one of the server's 512 slots for as long as it lives, not just while a request is in flight. The idle timeout is what stops quiet clients from crowding out live ones.

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

| Argument        | Type                     | Required | Meaning                                              |
| --------------- | ------------------------ | -------- | ---------------------------------------------------- |
| `ip`            | string literal           | yes      | address to bind                                      |
| `port`          | integer literal          | yes      | TCP port to listen on                                |
| `allow_origins` | array of string literals | no       | origins permitted to call this server from a browser |

The macro must be applied to a function named `main`. A missing required argument, an unknown argument name and a repeated one are all compile errors pointing at the offending token.

## CORS

Browsers refuse to hand a page the response to a cross-origin request unless the server says they may. Nothing but the browser enforces this, so it changes nothing for `curl`, for a reverse proxy, or for a load test — a request without an `Origin` header is answered exactly as it was before this existed.

By default no cross-origin headers are emitted at all. Naming origins turns them on:

```rust
#[http_server(
    ip = "127.0.0.1",
    port = 8443,
    allow_origins = ["http://localhost:1420", "tauri://localhost"]
)]
async fn main() {}
```

Use `["*"]` to permit any origin. Origins are matched in full — scheme, host and port, no trailing slash — and compared case-insensitively; `http://localhost:1420` and `http://127.0.0.1:1420` are different origins.

Two things then happen, and a browser needs both:

**The preflight.** Before any request that is not [simple] — which includes every `fetch` carrying `Content-Type: application/json` — the browser sends an `OPTIONS` request of its own and will not send the real one until it is answered. A preflight from a listed origin gets:

```text
HTTP/1.1 204 No Content
Access-Control-Allow-Origin: http://localhost:1420
Vary: Origin
Access-Control-Allow-Methods: POST
Access-Control-Allow-Headers: content-type
Access-Control-Max-Age: 600
```

`Access-Control-Allow-Methods` lists what the path actually serves, so a preflight for `DELETE` against a `GET`-only route comes back listing `GET` and the browser blocks the request itself. `Access-Control-Allow-Headers` echoes what was asked for. The result is cacheable for ten minutes, so a browser pays for the preflight once per origin, method and header set rather than once per request.

**The response.** Every answer to a request from a listed origin carries `Access-Control-Allow-Origin`, whether it came from a handler or from the server — a `404` a script cannot read shows up in the console as an opaque load failure rather than as a `404`.

An origin that is not listed gets a truthful answer with no CORS headers on it, which is how a browser is told no. `Vary: Origin` accompanies any echoed origin so a shared cache cannot serve one origin's permission slip to another.

Credentialed requests (cookies, `Authorization`) are not enabled through this attribute. `utils::cors::CorsConfig` supports them along with an explicit allow-header list; only the origin list is currently reachable from `#[http_server]`.

[simple]: https://developer.mozilla.org/docs/Glossary/CORS-safelisted_request_header

## Shared state

Every handler argument described so far comes out of the request. A database handle, a cache or a counter does not — it belongs to the process and outlives any one request. `#[component]` is how a handler asks for one.

It is two halves. On a struct, it declares the component and names the accessor:

```rust
#[component(name = "tickets")]
struct TicketStore {
    rows: Mutex<Vec<Ticket>>,
}

impl Default for TicketStore {
    fn default() -> Self {
        TicketStore { rows: Mutex::new(Vec::new()) }
    }
}
```

**The struct must implement `Default`, and `Default::default()` is what builds the shared instance.** There is no other constructor and no way to pass one in: nothing calls a component into being, so the only place a value can come from is a function that takes no arguments. `Default` is that function. Write it by hand as above when the starting state matters, or `#[derive(Default)]` when every field's own default will do.

That is also where any setup belongs — reading an environment variable, opening a file, connecting to a database. It runs once, on first access, on whichever request happens to arrive first. A panic inside it poisons the component, and every later access panics too.

On a parameter, it asks for that component:

```rust
#[get(path = "/tickets/{id}")]
async fn get_ticket(id: u64, #[component] tickets: &TicketStore) -> HttpResponse<String> {
    let rows = tickets.rows.lock().unwrap();
    ...
}
```

`id` is still bound from the path; `tickets` is not looked for in the request at all. The two can appear in either order, and a component declared ahead of a body argument does not take the body.

**The parameter name is the accessor name.** `#[component(name = "tickets")]` generates `AppContext::tickets()`, and the parameter must be called `tickets` to receive it — that is the only link between the two, because a macro sees tokens and cannot work out which type a name refers to. The type you write is checked against what the accessor returns, so a mismatched pair is a compile error rather than a surprise at runtime.

**A component is always taken as an immutable reference.** The accessor hands back `&'static TicketStore`, so the parameter must be written `&TicketStore` — `&mut TicketStore` is rejected with a message saying so, and taking it by value is rejected too. There is no version of this that lends you the component exclusively: one instance is shared by every handler on every connection, and those run at the same time, so a `&mut` to it could not be handed out to two requests at once. That is the same reason the struct must be `Send + Sync`: the instance lives in a `static`, and the connections reaching it are spread across Tokio's worker threads.

This is not a restriction on what you can change, only on where the changing happens. Put a `Mutex`, `RwLock` or an atomic on the field you need to write, and mutate through the shared reference:

```rust
fn add(&self, ticket: Ticket) {
    self.rows.lock().unwrap().push(ticket);
}
```

| Rule                                | Why                                                                                                             |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `name` is required                  | `TicketStore` → `tickets` has no answer a caller would predict for `HTTPHandler` or `TicketDb`                  |
| `name` must be a plain identifier   | it becomes a method name, so `"ticket-handler"`, `"1st"` and keywords like `"type"` are rejected at the literal |
| The struct must implement `Default` | `Default::default()` is what constructs the shared instance                                                     |
| The struct must be `Send + Sync`    | one instance is shared by every connection, and connections run concurrently on Tokio's worker threads          |
| The parameter must be `&T`          | not `&mut T`, not `T` — every handler holds the same instance                                                   |
| The struct must not be generic      | there would be no single type to instantiate                                                                    |

Instances are created on first access, once, and live for the rest of the process. Nothing is constructed for a component no handler ever asks for.

Synchronization is deliberately yours to place. There is no `sync` switch that wraps the whole struct in a lock: locking a field at a time keeps the granularity where you can see it, and a handler that asks for two components cannot deadlock on an ordering the macro chose for it.

`AppContext` — the type the accessors are implemented on — is generated by `#[http_server]`, so components live in the same crate as your `main`. Handlers do not name it; the generated code does.

**The accessor is exactly as visible as the struct.** `#[component]` copies the struct's visibility onto the method it generates, so a `pub struct` gets `pub fn tickets()` and a private struct gets a private one. A component declared at the crate root is therefore reachable from every handler in the crate, which is the common case and needs no thought. A component declared inside a module is not:

```rust
mod store {
    #[component(name = "tickets")]
    #[derive(Default)]
    struct TicketStore { … }          // private → so is `AppContext::tickets()`
}

// error: `tickets` is private
#[get(path = "/tickets")]
async fn list(#[component] tickets: &TicketStore) -> HttpResponse<String> { … }
```

Write `pub(crate) struct TicketStore` and both the type and its accessor reach the whole crate. Inheriting the visibility rather than always emitting `pub` is what keeps the generated method from being more public than the type it hands out.

## Limits

| Limit                         | Value | Exceeded                                 |
| ----------------------------- | ----- | ---------------------------------------- |
| Request size (headers + body) | 1 MiB | `413 Payload Too Large`                  |
| Request duration              | 30 s  | `408 Request Timeout`                    |
| Idle time between requests    | 15 s  | connection closed, no response           |
| Requests per connection       | 100   | connection closed after the 100th answer |
| TLS handshake duration        | 10 s  | connection dropped                       |
| Concurrent connections        | 512   | client waits in the accept backlog       |
| Headers per request           | 32    | `400 Bad Request`                        |

The request deadline is a single budget shared by the header and body reads, so it cannot be extended by pacing bytes slowly. It starts at the request's first byte, so time a connection spends idle is not charged to the request that follows.

The request-size limit bounds _buffered_ bytes rather than strictly one request: a client that pipelines spends the same budget on everything it has sent that has not been answered yet. For a client that waits for each response, the two are the same number.

Peak memory is roughly `connections × 3 × request size` — the request buffer, the copied body and the deserialized value are all live at once — so the size and connection limits are one decision rather than two.

These are currently compile-time constants in `server::Limits`; the macro passes `Limits::default()` and does not yet expose them as attribute arguments.

## Security & limitations

This server is intended to run as an **internal service behind a reverse proxy** (e.g. nginx, Caddy, Traefik, or a cloud load balancer). The proxy is expected to terminate public TLS with a CA-issued certificate and handle concerns like authentication, rate limiting and request buffering — the embedded server itself stays minimal and trusts that it sits on a private network.

The server is **TLS-only**. Encryption is always on, but the certificate is **self-signed and regenerated on every startup**, with Subject Alternative Names set to `"embedded-http-server-rs"` and the provided `ip`. Connections are encrypted, but the cert is **not browser-trusted** — this is by design. The self-signed cert secures the hop between the reverse proxy and this server on a trusted internal network; the proxy presents a CA-issued certificate to the outside world. The server is not meant to be exposed directly to the public internet.

Other constraints worth knowing:

- **Request bodies must be valid UTF-8.** Binary payloads are rejected with `400 Bad Request`, so files have to be encoded — base64 inside JSON, for instance. There is no `multipart/form-data` support and no streaming; the whole request is buffered in memory.
- **`Transfer-Encoding: chunked` is not supported.** A chunked request is dispatched with an empty body rather than rejected.
- **A panicking handler** drops its connection without a response, and takes any requests already pipelined behind it with it.
- **Handlers cannot return `Result`.** Blocking work inside one occupies a runtime worker, and on a persistent connection it also stalls every later request on that same connection.
- A closed connection is followed by an explicit TLS `close_notify`, so peers can distinguish a normal end of stream from a truncated one.
- Failed TLS handshakes and failed writes do not panic; the connection is dropped and the accept loop continues. Failing to bind the address at startup is fatal.
- Sustained `accept` failure — file-descriptor exhaustion, say — is logged once at onset and once on recovery with a count, rather than on every retry.

## TODO

Known gaps, in rough order of how much they cost a caller. Each one is a deliberate
absence rather than a surprise — this is the list of what "minimal" currently leaves out.

### Protocol

- [ ] **Decode `Transfer-Encoding: chunked`** instead of refusing it with `501`. Any request framed this way is rejected today; `httparse::parse_chunk_size` is the piece that would make reading one possible.
- [x] **Reject conflicting `Content-Length` headers.** RFC 9112 §6.3 requires it; two of them currently resolve to the first value, which is a framing decision a client should not get to make twice.
- [ ] **Answer `HEAD`.** RFC 9110 §9.3.2 requires it wherever `GET` is served, and it comes back `405` today. It would run the `GET` handler and drop the body, the way `OPTIONS` is already answered outside the route tables.
- [ ] **Percent-decode path and query values**, and treat `+` as a space in query strings. `/hello/John%20Doe` binds the literal `John%20Doe` right now, so every handler would otherwise have to decode for itself.
- [ ] **Treat method tokens as case-sensitive.** RFC 9110 §9.1 says they are; `Method::from_str` uppercases first while `dispatch`'s `OPTIONS` check compares verbatim, so the two disagree about `options`.

### Newer protocol versions

HTTP/2 and HTTP/3 do not have `Transfer-Encoding` at all. Both replace the text framing
above with a **binary framing layer** — bodies travel in `DATA` frames and headers are
compressed (HPACK for HTTP/2, QPACK for HTTP/3), so chunked encoding has nothing left to
do and is forbidden outright.

- [ ] **Advertise `http/1.1` through ALPN.** No protocol is offered during the handshake today, so agreement on HTTP/1.1 is left implicit.
- [ ] **HTTP/2** behind ALPN `h2`: binary framing, HPACK, and stream multiplexing, which replaces the one-request-at-a-time read loop with concurrent streams on a single connection.
- [ ] **HTTP/3** over QUIC afterwards. It is a larger step than h2 — a UDP transport with TLS folded into it, so `rustls` stays but `TcpListener` and `tokio_rustls` do not.

### Behaviour

- [ ] **One status for one failure.** An unparseable parameter answers `404` for numeric and `bool` types but `400` for anything deserialized through `serde_json` — the Rust type decides, which is not something a caller can reason about.
- [ ] **Catch a handler panic** and answer `500`, rather than dropping the connection with no response. A panic while holding a component's `Mutex` also poisons it for the rest of the process, which needs an answer of its own.
- [ ] **Let handlers return `Result`**, so an error path does not have to be spelled as a successful `HttpResponse` carrying an error status.
- [ ] **Accept binary bodies.** The request is handled as a `String` end to end, so uploads have to be base64'd inside JSON; lifting that is also what `multipart/form-data` and streaming would need.

### Configuration

- [ ] **Expose `Limits` through `#[http_server]`.** All seven are compile-time constants, so sizing them to a workload means editing this crate.
- [ ] **Accept a supplied certificate and key**, instead of only the self-signed pair regenerated at every startup.
- [ ] **Reach the rest of `CorsConfig` from the attribute** — credentialed requests and an explicit allow-header list are implemented but only `allow_origins` is wired up.

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
