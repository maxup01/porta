//! Connection handling: reading a request off a stream and answering it.
//!
//! This lives in a crate of its own rather than inside `#[http_server]`'s `quote!`
//! block. Code emitted by a macro exists only in the caller's crate, so nothing in
//! this workspace can call it, and nothing can test it — the read loop, the size
//! ceiling and the request deadline were previously unreachable for that reason.
//!
//! Everything here is generic over [`AsyncRead`] + [`AsyncWrite`], so tests drive it
//! over an in-memory [`tokio::io::duplex`] pipe while the macro passes a TLS stream.
//! The transport is the only difference between the two.

use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::Instant;
use utils::cors::{CorsConfig, is_preflight};
use utils::request::route::{
    Method, extract_method_from_request, extract_method_token_from_request,
    extract_path_from_request, get_route_function, methods_for_path,
};
use utils::response::{HttpStatus, status_response, with_headers};

/// Size of a single read from the stream. Requests are assembled from as many of
/// these as they need.
const READ_CHUNK_BYTES: usize = 4096;

/// Upper bound on how many headers a request may carry. Exceeding it is a parse
/// failure, answered with `400`.
const MAX_HEADERS: usize = 32;

/// Per-connection limits.
///
/// `max_request_bytes` bounds one connection; the number of connections is bounded
/// by the caller. Peak memory is roughly the product of the two multiplied again by
/// three, since the request buffer, the copied body and the deserialized value are
/// all live at once.
///
/// The remaining three exist because connections persist. A connection that serves
/// many requests holds a caller-issued slot for as long as it lives, so how long it
/// may sit idle and how many requests it may serve are limits in their own right —
/// without them, a client that opens connections and never speaks again costs one
/// socket each and takes the server's capacity with it.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Total bytes buffered for one connection, headers plus body. Exceeding it is
    /// answered with `413 Payload Too Large`.
    ///
    /// It is a ceiling on buffered bytes rather than strictly on one request: a
    /// client that pipelines spends the same budget on whatever it has sent that
    /// has not been answered yet. For a client that waits for each response — every
    /// browser, and every HTTP library by default — the two are the same number.
    pub max_request_bytes: usize,

    /// Budget for delivering a whole request, measured from the arrival of its
    /// first byte. It is a single deadline shared by the header and body reads, so
    /// it cannot be extended by pacing the bytes.
    ///
    /// It starts at the first byte rather than at the read, so time a connection
    /// spends idle between requests is not charged to the request that follows.
    pub request_timeout: Duration,

    /// How long a connection may sit with no request in progress before the server
    /// closes it.
    ///
    /// This is the cost of persistence: the slot stays taken while the client
    /// thinks. Too short and every client pays a fresh handshake anyway; too long
    /// and idle peers crowd out live ones.
    pub idle_timeout: Duration,

    /// How many requests one connection may serve before the server closes it.
    ///
    /// A backstop rather than a tuning knob. It caps the damage from any per-
    /// connection state that grows, and guarantees a long-lived client eventually
    /// re-resolves DNS and re-checks the certificate instead of pinning one socket
    /// indefinitely.
    pub max_requests_per_connection: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_request_bytes: 1024 * 1024,
            request_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(15),
            max_requests_per_connection: 100,
        }
    }
}

/// What reading a request produced.
enum ReadOutcome {
    /// A complete, well-formed request.
    Request {
        /// The request itself, headers and body, without any bytes belonging to
        /// whatever the client sent after it.
        request: String,

        /// Whether the client is willing to reuse the connection. Persistence
        /// needs both ends to agree, and this is the client's half of it.
        client_persists: bool,
    },

    /// The request was refused, and the peer should be told why.
    Reject(HttpStatus),

    /// The peer went away, went quiet, or the transport failed. There is nobody
    /// left to answer.
    Abandon,
}

/// Why one read stopped short.
enum ReadError {
    /// The deadline passed. What that means depends on whether a request was
    /// already in progress, so the caller decides rather than this.
    Timeout,

    /// The transport failed. Nothing more will arrive and nothing can be sent.
    Closed,
}

/// Serves requests from `stream` until neither side wants another, then closes it.
///
/// The connection is persistent, as HTTP/1.1 requires unless something says
/// otherwise. It ends when the client hangs up, when it asks to close, when it goes
/// quiet for `limits.idle_timeout`, when a request is refused, or when
/// `limits.max_requests_per_connection` is reached — and the last response the
/// server sends before any of those carries `Connection: close`, so the client
/// learns the connection is finished from the response rather than from a failed
/// write on its next request.
///
/// On every exit path the stream is shut down explicitly, which is what emits the
/// TLS `close_notify` when the stream is a TLS one — dropping it would skip that,
/// because shutdown is an async write and `Drop` is synchronous.
///
/// `cors` is the policy the server was configured with. Pass
/// [`CorsConfig::disabled`] to answer exactly as this server did before CORS
/// existed: no cross-origin headers on anything.
pub async fn handle_connection<S>(stream: &mut S, limits: Limits, cors: &CorsConfig)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // Outlives a single request on purpose. A read returns whatever bytes have
    // arrived, which for a pipelining client is the tail of one request and the
    // head of the next; that tail stays here and is the first thing parsed on the
    // following pass.
    let mut buffered: Vec<u8> = Vec::new();

    for request_number in 1..=limits.max_requests_per_connection {
        let (request, client_persists) = match read_request(stream, limits, &mut buffered).await {
            ReadOutcome::Request {
                request,
                client_persists,
            } => (request, client_persists),

            // A refused request leaves the stream at an unknown offset — a body
            // that was too large is still arriving, and a request that did not
            // parse has no length to skip. Nothing after it can be framed, so the
            // answer is also the goodbye.
            ReadOutcome::Reject(status) => {
                respond(stream, &closing(status_response(status))).await;
                break;
            }

            ReadOutcome::Abandon => break,
        };

        let response = dispatch(&request, cors).await;

        // Either side may end it: the client by saying so, the server by having
        // spent the budget.
        let last = !client_persists || request_number == limits.max_requests_per_connection;

        let written = if last {
            respond(stream, &closing(response)).await
        } else {
            respond(stream, &response).await
        };

        // The write failed, so the peer is gone and there is nothing to shut down.
        if !written {
            return;
        }

        if last {
            break;
        }
    }

    let _ = stream.shutdown().await;
}

/// Marks `response` as the last one on its connection.
///
/// HTTP/1.1 treats a connection as persistent unless a response says otherwise, so
/// this header is the only thing that distinguishes a server that is about to close
/// from one that is waiting for more. Sending it is what lets the client reuse the
/// socket for every response that does *not* carry it.
fn closing(response: String) -> String {
    with_headers(&response, &[("Connection", "close".to_string())])
}

/// Resolves a raw request to a response, without touching the transport.
///
/// Separate from [`handle_connection`] so the routing decision — handler, `405` or
/// `404` — can be tested against a plain string.
///
/// Every answer leaves through here, which is what makes this the place to attach
/// the CORS headers: a handler's response and a status the server generated itself
/// need the same ones, and a browser rejects the second as readily as the first
/// when they are missing.
pub async fn dispatch(request: &str, cors: &CorsConfig) -> String {
    let path = match extract_path_from_request(request) {
        Ok(path) => path,
        Err(_) => return status_response(HttpStatus::BadRequest),
    };

    // The request target carries the query string; route patterns never do.
    let path_without_query = match path.split_once('?') {
        Some((path_only, _)) => path_only,
        None => path.as_str(),
    };

    // Answered here rather than routed. `OPTIONS` is not a `Method`, so no handler
    // can be registered for it, and a browser sends one on its own initiative
    // before any request carrying a JSON body — including from a page this server
    // is the entire backend for. Falling through to the routing below would answer
    // it `404`, and the request the browser was asking permission for would never
    // be sent.
    if let Ok(method_token) = extract_method_token_from_request(request)
        && method_token == "OPTIONS"
    {
        return options_response(request, path_without_query, cors);
    }

    let route_function = match extract_method_from_request(request) {
        Ok(method) => get_route_function(path_without_query, method),
        Err(_) => None,
    };

    let response = match route_function {
        Some(route_function) => route_function(request).await,
        None => {
            let allowed_methods = methods_for_path(path_without_query);

            if allowed_methods.is_empty() {
                status_response(HttpStatus::NotFound)
            } else {
                // Served by some other verb, so this is the wrong method rather
                // than a missing resource. RFC 9110 §15.5.6 requires the `Allow`
                // header naming what is served instead.
                with_headers(
                    &status_response(HttpStatus::MethodNotAllowed),
                    &[("Allow", allow_header_value(&allowed_methods))],
                )
            }
        }
    };

    with_headers(&response, &cors.response_headers(request))
}

/// Answers an `OPTIONS` request: a CORS preflight if it is one and the policy
/// permits it, otherwise a plain statement of what the path serves.
///
/// A path nothing serves is a `404` either way. Answering a preflight for a route
/// that does not exist would grant permission for a request that could only fail,
/// and would make a typo in a URL look like a CORS problem.
fn options_response(request: &str, path: &str, cors: &CorsConfig) -> String {
    let allowed_methods = methods_for_path(path);

    if allowed_methods.is_empty() {
        return status_response(HttpStatus::NotFound);
    }

    if is_preflight(request)
        && let Some(preflight_headers) = cors.preflight_headers(request, &allowed_methods)
    {
        return with_headers(&status_response(HttpStatus::NoContent), &preflight_headers);
    }

    // Either not a preflight, or from an origin the policy does not cover. Both get
    // the same honest answer — here is what this path serves — and in the second
    // case the absent CORS headers are what tell the browser no.
    let mut headers = vec![("Allow", allow_header_value(&allowed_methods))];
    headers.extend(cors.response_headers(request));

    with_headers(&status_response(HttpStatus::NoContent), &headers)
}

/// Formats an `Allow` header value from the methods a path serves.
///
/// `OPTIONS` is appended because the server does answer it for every path that
/// exists, whatever its route table says, and `Allow` is defined as the set the
/// resource supports rather than the set someone registered.
fn allow_header_value(allowed_methods: &[Method]) -> String {
    allowed_methods
        .iter()
        .map(Method::as_str)
        .chain(std::iter::once("OPTIONS"))
        .collect::<Vec<&str>>()
        .join(", ")
}

/// Reads one request out of `buffered`, refilling it from `stream` as needed.
///
/// A single read is not a request: TLS record boundaries and TCP segmentation split
/// the byte stream at arbitrary points. The header block is read to completion
/// first, then exactly as many body bytes as `Content-Length` declares.
///
/// `buffered` is owned by the connection rather than by one request, and on return
/// it holds exactly the bytes that arrived after the request being returned. That
/// is what makes pipelining safe: a client is entitled to send its next request
/// without waiting, and those bytes land in the same read as the tail of this one.
/// Discarding them would silently lose a request the client believes it sent.
///
/// Because it is parsed before anything is read, a request already sitting in
/// `buffered` is answered without touching the stream at all.
///
/// # Deadlines
///
/// Two, and the distinction matters. Waiting for a request to *begin* is bounded by
/// `idle_timeout` and ends the connection silently — an idle persistent connection
/// is not a fault, and the client is very likely closing it at the same moment.
/// Waiting for a request already in progress to *finish* is bounded by
/// `request_timeout` and is answered `408`, because half a request is a fault.
async fn read_request<S>(stream: &mut S, limits: Limits, buffered: &mut Vec<u8>) -> ReadOutcome
where
    S: AsyncRead + Unpin,
{
    let mut chunk = [0u8; READ_CHUNK_BYTES];

    let idle_deadline = Instant::now() + limits.idle_timeout;

    // `None` until the first byte of this request is in hand. Bytes carried over
    // from the previous read mean it has already begun.
    let mut request_deadline =
        (!buffered.is_empty()).then(|| Instant::now() + limits.request_timeout);

    let (header_len, content_length, client_persists) = loop {
        // Scoped because the parse borrows `buffered`, which the read below needs
        // mutably. Only Copy values leave.
        let parsed = {
            let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
            let mut parsed_request = httparse::Request::new(&mut headers);

            let parse_request_result = parsed_request.parse(buffered);

            if parsed_request
                .headers
                .iter()
                .any(|header| header.name.eq_ignore_ascii_case("transfer-encoding"))
            {
                return ReadOutcome::Reject(HttpStatus::NotImplemented);
            }

            match parse_request_result {
                Ok(httparse::Status::Complete(header_len)) => {
                    // Chunked transfer encoding is not supported, so such a request
                    // is dispatched with an empty body rather than rejected.
                    let Some(content_length) = declared_body_length(&parsed_request) else {
                        return ReadOutcome::Reject(HttpStatus::BadRequest);
                    };

                    let client_persists = client_persists(
                        parsed_request.version,
                        header_value(&parsed_request, "connection"),
                    );

                    Some((header_len, content_length, client_persists))
                }
                Ok(httparse::Status::Partial) => None,
                Err(_) => return ReadOutcome::Reject(HttpStatus::BadRequest),
            }
        };

        if let Some(parsed) = parsed {
            break parsed;
        }

        let deadline = request_deadline.unwrap_or(idle_deadline);

        match read_chunk(stream, &mut chunk, deadline).await {
            // EOF. Between requests this is the client closing a connection it is
            // done with; mid-request it hung up early. Neither is answerable.
            Ok(0) => return ReadOutcome::Abandon,
            Ok(n) => {
                request_deadline.get_or_insert_with(|| Instant::now() + limits.request_timeout);
                buffered.extend_from_slice(&chunk[..n]);
            }
            Err(ReadError::Closed) => return ReadOutcome::Abandon,
            Err(ReadError::Timeout) => {
                return match request_deadline {
                    Some(_) => ReadOutcome::Reject(HttpStatus::RequestTimeout),
                    None => ReadOutcome::Abandon,
                };
            }
        }

        if buffered.len() > limits.max_request_bytes {
            return ReadOutcome::Reject(HttpStatus::PayloadTooLarge);
        }
    };

    // Checked, because Content-Length is attacker-controlled and a value near
    // usize::MAX would otherwise wrap into a small total.
    let total_len = match header_len.checked_add(content_length) {
        Some(total_len) if total_len <= limits.max_request_bytes => total_len,
        _ => return ReadOutcome::Reject(HttpStatus::PayloadTooLarge),
    };

    while buffered.len() < total_len {
        let deadline = request_deadline.unwrap_or(idle_deadline);

        match read_chunk(stream, &mut chunk, deadline).await {
            // EOF before the declared body arrived: the client hung up mid-request.
            Ok(0) => return ReadOutcome::Abandon,
            Ok(n) => buffered.extend_from_slice(&chunk[..n]),
            Err(ReadError::Closed) => return ReadOutcome::Abandon,
            Err(ReadError::Timeout) => return ReadOutcome::Reject(HttpStatus::RequestTimeout),
        }
    }

    // Splits the request off and leaves the rest — the head of whatever the client
    // pipelined — in place for the next call.
    let request: Vec<u8> = buffered.drain(..total_len).collect();

    // The request is complete, so a decoding failure is genuinely invalid UTF-8
    // rather than a multi-byte character split across two reads.
    match String::from_utf8(request) {
        Ok(request) => ReadOutcome::Request {
            request,
            client_persists,
        },
        Err(_) => ReadOutcome::Reject(HttpStatus::BadRequest),
    }
}

/// First value of `name` in a parsed request, as text, or `None` if it is absent or
/// not UTF-8.
fn header_value<'a>(request: &httparse::Request<'_, 'a>, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .and_then(|header| std::str::from_utf8(header.value).ok())
}

/// How many body bytes the request declares, or `None` when the declaration cannot
/// be trusted.
///
/// Absent means no body, which is the ordinary case for a `GET`. Anything else has
/// to be one unambiguous decimal, because this number is what decides where the
/// request ends: RFC 9112 §6.3 makes a duplicated or unparseable `Content-Length` an
/// unrecoverable framing error, and the alternative to refusing is guessing. A guess
/// that reads short leaves the remainder in `buffered`, where the next pass through
/// the read loop answers it as a request of its own — one no client sent.
///
/// Duplicates are refused even when they agree. A proxy combining two identical
/// headers is legal and harmless, but telling that apart from a smuggling attempt
/// means trusting the pair to match, and a client that wants a body read has no
/// reason to declare its length twice.
fn declared_body_length(request: &httparse::Request<'_, '_>) -> Option<usize> {
    let mut declarations = request
        .headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("content-length"));

    let declaration = match declarations.next() {
        Some(declaration) => declaration,
        None => return Some(0),
    };

    if declarations.next().is_some() {
        return None;
    }

    let value = std::str::from_utf8(declaration.value).ok()?.trim();

    // `1*DIGIT` and nothing else (RFC 9112 §6.2). `parse` alone is too generous: it
    // accepts `+5`, and a value a proxy joined into `5, 5` has to be refused rather
    // than read as either number.
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    // Still fallible with every byte a digit: a value longer than `usize` can hold
    // overflows, and treating that as no body is what the old `unwrap_or(0)` did.
    value.parse().ok()
}

/// Whether the client is willing to reuse this connection.
///
/// The default flipped between HTTP versions, and both are still on the wire, so
/// the version decides what silence means: HTTP/1.1 persists unless it says `close`
/// (RFC 9112 §9.3), HTTP/1.0 closes unless it says `keep-alive`. An unrecognised
/// version is treated as closing, since guessing wrong the other way desynchronises
/// a connection rather than merely costing a handshake.
///
/// `connection` is a comma-separated token list, so a bare `contains` would match
/// `close` inside a longer token. It is split before comparison.
fn client_persists(version: Option<u8>, connection: Option<&str>) -> bool {
    let has_token = |wanted: &str| {
        connection.is_some_and(|value| {
            value
                .split(',')
                .any(|token| token.trim().eq_ignore_ascii_case(wanted))
        })
    };

    match version {
        _ if has_token("close") => false,
        Some(1) => true,
        Some(0) => has_token("keep-alive"),
        _ => false,
    }
}

/// One deadline-bounded read. `Ok(0)` is EOF; `Err` says why nothing was read.
async fn read_chunk<S>(
    stream: &mut S,
    buffer: &mut [u8],
    deadline: Instant,
) -> Result<usize, ReadError>
where
    S: AsyncRead + Unpin,
{
    match tokio::time::timeout_at(deadline, stream.read(buffer)).await {
        Ok(Ok(n)) => Ok(n),
        Ok(Err(_)) => Err(ReadError::Closed),
        Err(_) => Err(ReadError::Timeout),
    }
}

/// Writes a response, returning whether it reached the peer.
///
/// The stream is left open: on a persistent connection the next request arrives
/// through it, and shutting down here is what made the previous one-request-per-
/// connection server unable to serve a second. [`handle_connection`] shuts down
/// once, when the loop is over.
///
/// A failed write means the peer is already gone — nothing worth panicking about,
/// but the caller should stop rather than write again.
async fn respond<S>(stream: &mut S, message: &str) -> bool
where
    S: AsyncWrite + Unpin,
{
    stream.write_all(message.as_bytes()).await.is_ok()
}

#[cfg(test)]
mod tests;
