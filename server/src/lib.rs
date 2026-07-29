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
use utils::request::route::{
    extract_method_from_request, extract_path_from_request, get_route_function, path_exists,
};
use utils::response::{HttpStatus, status_response};

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
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Total bytes accepted for one request, headers plus body. Exceeding it is
    /// answered with `413 Payload Too Large`.
    pub max_request_bytes: usize,

    /// Budget for delivering a whole request, measured from the moment the
    /// connection is handed over. It is a single deadline shared by the header and
    /// body reads, so it cannot be extended by pacing the bytes.
    pub request_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_request_bytes: 1024 * 1024,
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// What reading a request produced.
enum ReadOutcome {
    /// A complete, well-formed request.
    Request(String),

    /// The request was refused, and the peer should be told why.
    Reject(HttpStatus),

    /// The peer went away or the transport failed. There is nobody left to answer.
    Abandon,
}

/// Reads one request from `stream`, answers it, and closes the stream.
///
/// A connection serves exactly one request. On any exit path the stream is shut
/// down explicitly, which is what emits the TLS `close_notify` when the stream is a
/// TLS one — dropping it would skip that, because shutdown is an async write and
/// `Drop` is synchronous.
pub async fn handle_connection<S>(stream: &mut S, limits: Limits)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // Taken before the first read so that the budget covers the request as a whole
    // rather than resetting on every chunk.
    let deadline = Instant::now() + limits.request_timeout;

    let request = match read_request(stream, limits, deadline).await {
        ReadOutcome::Request(request) => request,
        ReadOutcome::Reject(status) => {
            respond(stream, &status_response(status)).await;
            return;
        }
        ReadOutcome::Abandon => return,
    };

    respond(stream, &dispatch(&request)).await;
}

/// Resolves a raw request to a response, without touching the transport.
///
/// Separate from [`handle_connection`] so the routing decision — handler, `405` or
/// `404` — can be tested against a plain string.
pub fn dispatch(request: &str) -> String {
    let path = match extract_path_from_request(request) {
        Ok(path) => path,
        Err(_) => return status_response(HttpStatus::BadRequest),
    };

    // The request target carries the query string; route patterns never do.
    let path_without_query = match path.split_once('?') {
        Some((path_only, _)) => path_only,
        None => path.as_str(),
    };

    let route_function = match extract_method_from_request(request) {
        Ok(method) => get_route_function(path_without_query, method),
        Err(_) => None,
    };

    if let Some(route_function) = route_function {
        route_function(request)
    } else if path_exists(path_without_query) {
        // Served by some other verb, so this is the wrong method rather than a
        // missing resource.
        status_response(HttpStatus::MethodNotAllowed)
    } else {
        status_response(HttpStatus::NotFound)
    }
}

/// Reads until a complete request has arrived, or until something says stop.
///
/// A single read is not a request: TLS record boundaries and TCP segmentation split
/// the byte stream at arbitrary points. The header block is read to completion
/// first, then exactly as many body bytes as `Content-Length` declares.
async fn read_request<S>(stream: &mut S, limits: Limits, deadline: Instant) -> ReadOutcome
where
    S: AsyncRead + Unpin,
{
    let mut data: Vec<u8> = Vec::new();
    let mut buffer = [0u8; READ_CHUNK_BYTES];

    let (header_len, content_length) = loop {
        let n = match read_chunk(stream, &mut buffer, deadline).await {
            Ok(0) => return ReadOutcome::Abandon,
            Ok(n) => n,
            Err(outcome) => return outcome,
        };

        data.extend_from_slice(&buffer[..n]);

        if data.len() > limits.max_request_bytes {
            return ReadOutcome::Reject(HttpStatus::PayloadTooLarge);
        }

        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut parsed_request = httparse::Request::new(&mut headers);

        // `parsed_request` borrows `data`, so only Copy values may leave this match —
        // the next iteration needs `data` mutable again.
        match parsed_request.parse(&data) {
            Ok(httparse::Status::Complete(header_len)) => {
                // Absent, unparseable or duplicated Content-Length is treated as no
                // body. Chunked transfer encoding is not supported, so such a request
                // is dispatched with an empty body rather than rejected.
                let content_length = parsed_request
                    .headers
                    .iter()
                    .find(|header| header.name.eq_ignore_ascii_case("content-length"))
                    .and_then(|header| std::str::from_utf8(header.value).ok())
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);

                break (header_len, content_length);
            }
            Ok(httparse::Status::Partial) => continue,
            Err(_) => return ReadOutcome::Reject(HttpStatus::BadRequest),
        }
    };

    // Checked, because Content-Length is attacker-controlled and a value near
    // usize::MAX would otherwise wrap into a small total.
    let total_len = match header_len.checked_add(content_length) {
        Some(total_len) if total_len <= limits.max_request_bytes => total_len,
        _ => return ReadOutcome::Reject(HttpStatus::PayloadTooLarge),
    };

    while data.len() < total_len {
        let n = match read_chunk(stream, &mut buffer, deadline).await {
            // EOF before the declared body arrived: the client hung up mid-request.
            Ok(0) => return ReadOutcome::Abandon,
            Ok(n) => n,
            Err(outcome) => return outcome,
        };

        data.extend_from_slice(&buffer[..n]);
    }

    // Anything past Content-Length belongs to a pipelined request, and this server
    // answers one request per connection.
    data.truncate(total_len);

    // The request is complete, so a decoding failure is genuinely invalid UTF-8
    // rather than a multi-byte character split across two reads.
    match String::from_utf8(data) {
        Ok(request) => ReadOutcome::Request(request),
        Err(_) => ReadOutcome::Reject(HttpStatus::BadRequest),
    }
}

/// One deadline-bounded read. `Err` carries the outcome the caller should return.
async fn read_chunk<S>(
    stream: &mut S,
    buffer: &mut [u8],
    deadline: Instant,
) -> Result<usize, ReadOutcome>
where
    S: AsyncRead + Unpin,
{
    match tokio::time::timeout_at(deadline, stream.read(buffer)).await {
        Ok(Ok(n)) => Ok(n),
        Ok(Err(_)) => Err(ReadOutcome::Abandon),
        Err(_) => Err(ReadOutcome::Reject(HttpStatus::RequestTimeout)),
    }
}

/// Writes a response and closes the stream cleanly.
///
/// A failed write means the peer is already gone, so there is nothing left to shut
/// down — and nothing worth panicking about.
async fn respond<S>(stream: &mut S, message: &str)
where
    S: AsyncWrite + Unpin,
{
    if stream.write_all(message.as_bytes()).await.is_ok() {
        let _ = stream.shutdown().await;
    }
}

#[cfg(test)]
mod tests;
