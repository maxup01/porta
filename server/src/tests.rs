use super::*;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The production defaults, with the idle timeout cut to something a test suite
/// can wait for.
///
/// Connections persist, so the server does not close one until the client stops
/// using it. A test whose client writes a request and then only reads would
/// otherwise sit out the full fifteen seconds before the server gave up on it.
/// Every test that means to hold a connection open does so deliberately, and this
/// bounds the ones that do not.
fn limits() -> Limits {
    Limits {
        idle_timeout: std::time::Duration::from_millis(100),
        ..Limits::default()
    }
}

/// Drives `handle_connection` over an in-memory pipe.
///
/// `client` is handed to the caller's closure, which plays the peer: it writes
/// request bytes and then reads until the server shuts the stream down. The
/// returned string is the raw response.
async fn exchange<F, Fut>(limits: Limits, play_client: F) -> String
where
    F: FnOnce(tokio::io::DuplexStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = String> + Send + 'static,
{
    exchange_with_cors(limits, CorsConfig::disabled(), play_client).await
}

/// [`exchange`], with a CORS policy other than the default of none.
async fn exchange_with_cors<F, Fut>(limits: Limits, cors: CorsConfig, play_client: F) -> String
where
    F: FnOnce(tokio::io::DuplexStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = String> + Send + 'static,
{
    exchange_returning_with_cors(limits, cors, play_client).await
}

/// [`exchange`] for a client that returns something other than one response.
///
/// A connection that serves several requests produces several responses, and the
/// tests that check reuse need all of them rather than one concatenated string.
async fn exchange_returning<F, Fut, T>(limits: Limits, play_client: F) -> T
where
    F: FnOnce(tokio::io::DuplexStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    exchange_returning_with_cors(limits, CorsConfig::disabled(), play_client).await
}

/// The one that actually runs the server. The three above are its spellings.
async fn exchange_returning_with_cors<F, Fut, T>(
    limits: Limits,
    cors: CorsConfig,
    play_client: F,
) -> T
where
    F: FnOnce(tokio::io::DuplexStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (client, mut server) = tokio::io::duplex(READ_CHUNK_BYTES * 2);

    let client_side = tokio::spawn(play_client(client));

    handle_connection(&mut server, limits, &cors).await;

    // Dropped before joining the client. On the paths where the server answers
    // nothing — a peer that hung up mid-request — no shutdown is sent, so the
    // client's `read_to_end` would wait for an EOF that only this drop produces.
    drop(server);

    client_side.await.expect("client task panicked")
}

/// Writes `chunks` with a pause between each, then reads the whole response.
///
/// The write half is closed once the last chunk is out. On a persistent connection
/// that is how a client says it has nothing further to send: without it the server
/// would correctly wait for another request, and `read_to_end` would block until
/// the idle timeout rather than returning as soon as the answer arrives. Tests that
/// exercise reuse keep the write half open on purpose and are written by hand.
async fn send_chunks(mut client: tokio::io::DuplexStream, chunks: Vec<Vec<u8>>) -> String {
    for chunk in chunks {
        client.write_all(&chunk).await.expect("write failed");
        // Yields so the server side actually observes a short read rather than
        // finding everything already buffered.
        tokio::task::yield_now().await;
    }

    let _ = client.shutdown().await;

    let mut response = Vec::new();
    let _ = client.read_to_end(&mut response).await;

    String::from_utf8_lossy(&response).to_string()
}

/// Reads exactly one response off `client`, leaving the connection open.
///
/// `read_to_end` cannot be used on a connection that is going to be reused: it
/// waits for an EOF the server has no reason to send. Responses here are framed by
/// `Content-Length`, or by the end of the header block when there is none — the
/// same rules a real client uses to find where one response stops and the next
/// begins.
///
/// `buffered` belongs to the client rather than to one response, for the same
/// reason the server keeps one: answers to pipelined requests arrive together, and
/// a reader that dropped whatever came after the response it was asked for would
/// lose the next one. Reusing it across calls is what makes a second call work.
async fn read_one_response(client: &mut tokio::io::DuplexStream, buffered: &mut Vec<u8>) -> String {
    let mut chunk = [0u8; 1024];

    loop {
        let header_end = buffered
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|start| start + 4);

        if let Some(header_end) = header_end {
            let headers = String::from_utf8_lossy(&buffered[..header_end]).to_string();

            let content_length = utils::request::header::get_header(&headers, "content-length")
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);

            if buffered.len() >= header_end + content_length {
                let response: Vec<u8> = buffered.drain(..header_end + content_length).collect();

                return String::from_utf8_lossy(&response).to_string();
            }
        }

        match client.read(&mut chunk).await {
            // The connection ended before a whole response arrived. Returning what
            // there is lets the assertion name it instead of hanging.
            Ok(0) | Err(_) => return String::from_utf8_lossy(buffered).to_string(),
            Ok(n) => buffered.extend_from_slice(&chunk[..n]),
        }
    }
}

// ── Assembly across reads ────────────────────────────────────────────────────

#[tokio::test]
async fn request_split_mid_header_is_reassembled() {
    // The split falls inside a header name, the case a single `read()` got wrong.
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"GET /nothing-registered HTTP/1.1\r\nHo".to_vec(),
                b"st: example\r\n\r\n".to_vec(),
            ],
        )
    })
    .await;

    // 404 rather than 400 is the point: the request parsed, it just matched no
    // route. A 400 would mean the two halves were never joined.
    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "request was not reassembled: {response}"
    );
}

#[tokio::test]
async fn body_split_from_headers_is_awaited() {
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 9\r\n\r\n".to_vec(),
                b"first".to_vec(),
                b"last".to_vec(),
            ],
        )
    })
    .await;

    // Reaching dispatch at all proves all nine body bytes were collected first.
    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "body was not fully read: {response}"
    );
}

// ── Size ceiling ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn request_over_the_ceiling_is_rejected_with_413() {
    let limits = Limits {
        max_request_bytes: 512,
        ..limits()
    };

    let response = exchange(limits, |client| {
        let mut request =
            b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 4096\r\n\r\n".to_vec();
        request.extend(std::iter::repeat(b'x').take(4096));

        send_chunks(client, vec![request])
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 413 Payload Too Large\r\n"),
        "expected 413: {response}"
    );
}

#[tokio::test]
async fn declared_content_length_over_the_ceiling_is_rejected_before_reading_it() {
    // The body is never sent. The rejection has to come from the declared length
    // alone, otherwise the server would sit waiting for bytes that never arrive.
    let limits = Limits {
        max_request_bytes: 512,
        ..limits()
    };

    let response = exchange(limits, |client| {
        send_chunks(
            client,
            vec![b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 100000\r\n\r\n".to_vec()],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 413 Payload Too Large\r\n"),
        "expected 413 from the header alone: {response}"
    );
}

// ── Malformed input ──────────────────────────────────────────────────────────

#[tokio::test]
async fn unparseable_header_block_is_rejected_with_400() {
    let response = exchange(limits(), |client| {
        send_chunks(client, vec![b"GET /x HTTP/9.9\r\n\r\n".to_vec()])
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
}

#[tokio::test]
async fn body_that_is_not_utf8_is_rejected_with_400() {
    let response = exchange(limits(), |client| {
        let mut request =
            b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 4\r\n\r\n".to_vec();
        // A lone continuation byte sequence: valid bytes, invalid UTF-8.
        request.extend([0xFF, 0xFE, 0xFD, 0xFC]);

        send_chunks(client, vec![request])
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
}

// ── Content-Length framing ───────────────────────────────────────────────────

#[tokio::test]
async fn an_absent_content_length_means_no_body() {
    // The ordinary case, pinned because the framing checks below refuse everything
    // they cannot read as a single decimal — silence is not one of those.
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![b"POST /nothing-registered HTTP/1.1\r\nHost: x\r\n\r\n".to_vec()],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "a request with no body should still reach dispatch: {response}"
    );
}

#[tokio::test]
async fn conflicting_content_length_headers_are_rejected_with_400() {
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 5\r\nContent-Length: 6\r\n\r\nhello"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
    assert!(response.contains("Connection: close\r\n"), "{response}");
}

#[tokio::test]
async fn agreeing_content_length_headers_are_rejected_too() {
    // Stricter than RFC 9112 §6.3, which only requires refusing values that differ.
    // Accepting a matching pair means trusting them to match, and this server has
    // nothing to gain from a length declared twice.
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 5\r\nContent-Length: 5\r\n\r\nhello"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
}

#[tokio::test]
async fn a_second_content_length_cannot_smuggle_a_request() {
    // What the refusal is for. Reading the first declaration — zero — and ignoring
    // the second left the bytes after the header block in the buffer, where the next
    // pass answered them as a pipelined request. A proxy that framed the message by
    // the other declaration forwarded one request; this server ran two.
    register(Method::GET, "/cl-smuggling/target");

    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 0\r\nContent-Length: 44\r\n\r\n\
                  GET /cl-smuggling/target HTTP/1.1\r\nHost: x\r\n\r\n"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
    assert!(
        !response.contains("200 OK"),
        "the smuggled request was answered: {response}"
    );
}

#[tokio::test]
async fn a_content_length_that_is_not_a_number_is_rejected_with_400() {
    // Previously read as no body, which framed the request one byte short of
    // wherever it actually ended.
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![b"POST /nothing-registered HTTP/1.1\r\nContent-Length: abc\r\n\r\nhello".to_vec()],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
}

#[tokio::test]
async fn a_comma_joined_content_length_is_rejected_with_400() {
    // One header, two values, because something upstream folded a duplicated pair
    // into a list. It is the same ambiguity as two headers and gets the same answer.
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 5, 5\r\n\r\nhello".to_vec(),
            ],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
}

#[tokio::test]
async fn a_signed_content_length_is_rejected_with_400() {
    // `1*DIGIT` admits no sign, though `"+5".parse::<usize>()` would take it.
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![b"POST /nothing-registered HTTP/1.1\r\nContent-Length: +5\r\n\r\nhello".to_vec()],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
}

#[tokio::test]
async fn a_content_length_too_large_for_usize_is_rejected_with_400() {
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 999999999999999999999999\r\n\r\n"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400: {response}"
    );
}

// ── Framing this server cannot honour ────────────────────────────────────────

#[tokio::test]
async fn a_chunked_request_is_rejected_with_501() {
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n\
                  2\r\nhi\r\n0\r\n\r\n"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 501 Not Implemented\r\n"),
        "expected 501: {response}"
    );
}

#[tokio::test]
async fn a_chunked_request_ends_the_connection() {
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n\
                  0\r\n\r\n"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(response.contains("Connection: close\r\n"), "{response}");
}

#[tokio::test]
async fn transfer_encoding_is_recognised_whatever_its_case() {
    // Header names are case-insensitive on the wire and httparse does not normalise
    // them, so a lowercase spelling must not slip past the check.
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\ntransfer-encoding: Chunked\r\n\r\n\
                  0\r\n\r\n"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 501 Not Implemented\r\n"),
        "a lowercase Transfer-Encoding was not recognised: {response}"
    );
}

#[tokio::test]
async fn a_chunked_body_is_not_executed_as_a_smuggled_request() {
    // The reason this is a 501 rather than an empty body. Framing the request by
    // `Content-Length` — absent here, so zero — would leave the chunk data in the
    // buffer, where the next pass through the read loop parses it as a second,
    // pipelined request. A proxy that understands chunked sees one request; this
    // server would run two, the second one never written by any client the proxy
    // vetted.
    register(Method::GET, "/smuggling/target");

    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"POST /nothing-registered HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n\
                  GET /smuggling/target HTTP/1.1\r\nHost: x\r\n\r\n"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 501 Not Implemented\r\n"),
        "expected 501: {response}"
    );
    assert!(
        !response.contains("200 OK"),
        "the smuggled request was answered: {response}"
    );
}

// ── Deadline ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_request_that_never_completes_is_answered_with_408() {
    let limits = Limits {
        request_timeout: std::time::Duration::from_millis(50),
        ..limits()
    };

    let response = exchange(limits, |mut client| async move {
        // A header block that is never terminated. httparse keeps saying Partial,
        // so without a deadline this read would wait forever.
        client
            .write_all(b"GET /x HTTP/1.1\r\nHost: example")
            .await
            .expect("write failed");

        let mut response = Vec::new();
        let _ = client.read_to_end(&mut response).await;

        String::from_utf8_lossy(&response).to_string()
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
        "expected 408: {response}"
    );
}

#[tokio::test]
async fn the_deadline_is_not_reset_by_trickled_bytes() {
    // The whole point of a shared deadline: a client sending one byte at a time,
    // faster than any per-read timeout would fire, must still be cut off.
    let limits = Limits {
        request_timeout: std::time::Duration::from_millis(100),
        ..limits()
    };

    let response = exchange(limits, |mut client| async move {
        let dribble = tokio::spawn(async move {
            for _ in 0..200 {
                if client.write_all(b"X").await.is_err() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }

            let mut response = Vec::new();
            let _ = client.read_to_end(&mut response).await;

            String::from_utf8_lossy(&response).to_string()
        });

        dribble.await.unwrap_or_default()
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
        "a trickling client outlived its budget: {response}"
    );
}

// ── Peer disappearing ────────────────────────────────────────────────────────

#[tokio::test]
async fn eof_before_a_complete_request_is_answered_with_silence() {
    let response = exchange(limits(), |mut client| async move {
        client
            .write_all(b"GET /x HTTP/1.1\r\n")
            .await
            .expect("write failed");
        client.shutdown().await.expect("shutdown failed");

        let mut response = Vec::new();
        let _ = client.read_to_end(&mut response).await;

        String::from_utf8_lossy(&response).to_string()
    })
    .await;

    assert!(
        response.is_empty(),
        "a peer that hung up mid-request should not be written to: {response}"
    );
}

// ── Response framing ─────────────────────────────────────────────────────────

#[tokio::test]
async fn a_generated_error_does_not_end_a_healthy_connection() {
    // A 404 is an answer, not a fault. The request was framed correctly, so the
    // connection is still synchronised and the client may keep using it.
    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![b"GET /nothing-registered HTTP/1.1\r\n\r\n".to_vec()],
        )
    })
    .await;

    assert!(!response.contains("Connection: close"), "{response}");
    assert!(
        response.contains("Content-Type: text/plain\r\n"),
        "{response}"
    );
}

#[tokio::test]
async fn a_refused_request_ends_the_connection() {
    // The opposite case. A body that overran the ceiling is still arriving, so the
    // next bytes on the wire are not a request line and nothing after this can be
    // framed. Saying `close` is the only honest answer.
    let refusing = Limits {
        max_request_bytes: 512,
        ..limits()
    };

    let response = exchange(refusing, |client| {
        send_chunks(
            client,
            vec![b"POST /x HTTP/1.1\r\nContent-Length: 100000\r\n\r\n".to_vec()],
        )
    })
    .await;

    assert!(
        response.starts_with("HTTP/1.1 413 Payload Too Large\r\n"),
        "{response}"
    );
    assert!(response.contains("Connection: close\r\n"), "{response}");
}

// ── Connection reuse ─────────────────────────────────────────────────────────

#[tokio::test]
async fn a_second_request_is_served_on_the_same_connection() {
    // The point of the whole exercise: one TLS handshake, many requests. Before
    // this, the server shut the stream down after answering and the second request
    // was written into a closed socket.
    register(Method::GET, "/keep-alive/first");
    register(Method::GET, "/keep-alive/second");

    let (first, second) = exchange_returning(limits(), |mut client| async move {
        let mut buffered = Vec::new();

        client
            .write_all(b"GET /keep-alive/first HTTP/1.1\r\nHost: x\r\n\r\n")
            .await
            .expect("write failed");

        let first = read_one_response(&mut client, &mut buffered).await;

        client
            .write_all(b"GET /keep-alive/second HTTP/1.1\r\nHost: x\r\n\r\n")
            .await
            .expect("the connection was closed after one request");

        let second = read_one_response(&mut client, &mut buffered).await;

        (first, second)
    })
    .await;

    assert!(first.starts_with("HTTP/1.1 200 OK\r\n"), "{first}");
    assert!(second.starts_with("HTTP/1.1 200 OK\r\n"), "{second}");
    assert!(!first.contains("Connection: close"), "{first}");
}

#[tokio::test]
async fn pipelined_requests_are_both_answered() {
    // Both requests arrive in one read. The second one lives in the bytes past the
    // first request's Content-Length — which the old reader discarded, losing a
    // request the client believed it had sent.
    register(Method::POST, "/keep-alive-pipeline/items");

    let (first, second) = exchange_returning(limits(), |mut client| async move {
        client
            .write_all(
                b"POST /keep-alive-pipeline/items HTTP/1.1\r\nContent-Length: 2\r\n\r\nhi\
                  POST /keep-alive-pipeline/items HTTP/1.1\r\nContent-Length: 2\r\n\r\nho",
            )
            .await
            .expect("write failed");

        let mut buffered = Vec::new();

        let first = read_one_response(&mut client, &mut buffered).await;
        let second = read_one_response(&mut client, &mut buffered).await;

        (first, second)
    })
    .await;

    assert!(first.starts_with("HTTP/1.1 200 OK\r\n"), "{first}");
    assert!(
        second.starts_with("HTTP/1.1 200 OK\r\n"),
        "the pipelined second request was dropped: {second}"
    );
}

#[tokio::test]
async fn a_client_asking_to_close_is_obeyed() {
    register(Method::GET, "/keep-alive-close/items");

    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![b"GET /keep-alive-close/items HTTP/1.1\r\nConnection: close\r\n\r\n".to_vec()],
        )
    })
    .await;

    assert!(response.contains("Connection: close\r\n"), "{response}");
}

#[tokio::test]
async fn http_1_0_closes_unless_it_asks_otherwise() {
    // The default inverted between versions, and both are still on the wire. An
    // HTTP/1.0 client that is not told the connection ends will hang waiting for
    // an EOF that marks the end of a body it already has.
    register(Method::GET, "/keep-alive-http10/items");

    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![b"GET /keep-alive-http10/items HTTP/1.0\r\n\r\n".to_vec()],
        )
    })
    .await;

    assert!(response.contains("Connection: close\r\n"), "{response}");
}

#[tokio::test]
async fn http_1_0_keeps_alive_when_it_asks() {
    register(Method::GET, "/keep-alive-http10-on/items");

    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![
                b"GET /keep-alive-http10-on/items HTTP/1.0\r\nConnection: keep-alive\r\n\r\n"
                    .to_vec(),
            ],
        )
    })
    .await;

    assert!(!response.contains("Connection: close"), "{response}");
}

#[tokio::test]
async fn the_request_budget_closes_the_connection() {
    register(Method::GET, "/keep-alive-budget/items");

    let budgeted = Limits {
        max_requests_per_connection: 1,
        ..limits()
    };

    let response = exchange(budgeted, |client| {
        send_chunks(
            client,
            vec![b"GET /keep-alive-budget/items HTTP/1.1\r\n\r\n".to_vec()],
        )
    })
    .await;

    assert!(
        response.contains("Connection: close\r\n"),
        "the last response the budget allows must say so: {response}"
    );
}

#[tokio::test]
async fn an_idle_connection_is_closed_without_a_second_response() {
    // The cost of persistence, bounded. A client that stops talking must not hold
    // a connection slot forever, and a connection the server times out while it is
    // between requests has no fault to report — closing quietly is the answer.
    register(Method::GET, "/keep-alive-idle/items");

    let (first, after) = exchange_returning(limits(), |mut client| async move {
        client
            .write_all(b"GET /keep-alive-idle/items HTTP/1.1\r\n\r\n")
            .await
            .expect("write failed");

        let mut buffered = Vec::new();

        let first = read_one_response(&mut client, &mut buffered).await;

        // Never sends a second request. The server's idle timeout is what ends it.
        let mut rest = Vec::new();
        let _ = client.read_to_end(&mut rest).await;

        (first, String::from_utf8_lossy(&rest).to_string())
    })
    .await;

    assert!(first.starts_with("HTTP/1.1 200 OK\r\n"), "{first}");
    assert!(
        after.is_empty(),
        "an idle timeout is not a request to answer: {after}"
    );
}

// ── dispatch, without a transport ────────────────────────────────────────────

/// Registers a route so the dispatch tests below have something to resolve.
///
/// The route tables are process-global, so every path here is prefixed to keep
/// these tests from colliding with each other or with `utils`'.
fn register(method: Method, path: &str) {
    fn handler(_: &str) -> Pin<Box<dyn Future<Output = String> + Send + '_>> {
        Box::pin(async { utils::response::status_response(HttpStatus::Ok) })
    }

    utils::request::route::register_route(method, path, handler);
}

const ORIGIN: &str = "http://localhost:1420";

fn header_value<'a>(response: &'a str, name: &str) -> Option<&'a str> {
    utils::request::header::get_header(
        // `get_header` skips the first line, and a response's first line is its
        // status line — the same shape as a request line, for this purpose.
        response, name,
    )
}

#[tokio::test]
async fn dispatch_answers_an_unroutable_path_with_404() {
    let response = dispatch(
        "GET /nothing-registered HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
}

#[tokio::test]
async fn dispatch_answers_a_malformed_request_line_with_400() {
    let response = dispatch("GARBAGE", &CorsConfig::disabled()).await;

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
}

#[tokio::test]
async fn dispatch_answers_an_unsupported_verb_with_404_when_nothing_serves_the_path() {
    // CONNECT does not parse as a Method, so no handler resolves; with no route
    // registered for the path either, that is a 404 rather than a 405.
    let response = dispatch(
        "CONNECT /nothing-registered HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
}

// ── 405 and Allow ────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_405_names_the_methods_that_are_served() {
    register(Method::GET, "/dispatch-allow/items");
    register(Method::DELETE, "/dispatch-allow/items");

    let response = dispatch(
        "POST /dispatch-allow/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"),
        "{response}"
    );
    // HEAD rides along with GET: the server answers it for anything GET serves, so
    // `Allow` has to name it whether or not a handler was registered for it.
    assert_eq!(
        header_value(&response, "Allow"),
        Some("GET, HEAD, DELETE, OPTIONS")
    );
}

// ── OPTIONS ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_plain_options_request_is_answered_with_204_and_allow() {
    // The bug this closes: OPTIONS is not a `Method`, so this used to fall through
    // the routing table and answer 404 for a path that plainly exists.
    register(Method::GET, "/dispatch-options/items");
    register(Method::POST, "/dispatch-options/items");

    let response = dispatch(
        "OPTIONS /dispatch-options/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 204 No Content\r\n"),
        "{response}"
    );
    assert_eq!(
        header_value(&response, "Allow"),
        Some("GET, HEAD, POST, OPTIONS")
    );
}

#[tokio::test]
async fn an_options_request_for_an_unserved_path_is_still_a_404() {
    let response = dispatch(
        "OPTIONS /dispatch-options/nothing-here HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
}

#[tokio::test]
async fn an_options_request_ignores_the_query_string() {
    register(Method::GET, "/dispatch-options-query/items");

    let response = dispatch(
        "OPTIONS /dispatch-options-query/items?id=5 HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 204 No Content\r\n"),
        "{response}"
    );
}

// ── HEAD ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_head_request_is_answered_by_the_get_handler() {
    // No handler is registered for HEAD, and none can be: it has no `Method` and no
    // table. RFC 9110 §9.1 requires it wherever GET is served, so it comes from the
    // GET route or not at all.
    register(Method::GET, "/dispatch-head/items");

    let response = dispatch(
        "HEAD /dispatch-head/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
}

#[tokio::test]
async fn a_head_response_has_no_body() {
    register(Method::GET, "/dispatch-head-empty/items");

    let response = dispatch(
        "HEAD /dispatch-head-empty/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.ends_with("\r\n\r\n"),
        "a HEAD answer carried content: {response}"
    );
}

#[tokio::test]
async fn a_head_response_reports_the_length_a_get_would_have_sent() {
    // What a cache or a client checking a size is actually asking for. Recomputing
    // the length from the absent body would answer 0 and make HEAD useless.
    register(Method::GET, "/dispatch-head-length/items");

    let from_get = dispatch(
        "GET /dispatch-head-length/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    let from_head = dispatch(
        "HEAD /dispatch-head-length/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert_eq!(
        header_value(&from_head, "Content-Length"),
        header_value(&from_get, "Content-Length"),
        "HEAD disagreed with GET about the body's length"
    );
    assert_eq!(
        header_value(&from_head, "Content-Type"),
        header_value(&from_get, "Content-Type"),
    );
}

#[tokio::test]
async fn a_head_request_for_an_unserved_path_is_a_404_without_a_body() {
    let response = dispatch(
        "HEAD /nothing-registered-for-head HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
    // A generated status loses its body too: the rule belongs to the method, not to
    // whichever part of the server wrote the answer.
    assert!(response.ends_with("\r\n\r\n"), "{response}");
}

#[tokio::test]
async fn a_head_request_against_a_route_without_get_is_a_405() {
    // HEAD mirrors GET and nothing else, so a POST-only path does not serve it — and
    // the `Allow` it comes back with names no HEAD either.
    register(Method::POST, "/dispatch-head-405/items");

    let response = dispatch(
        "HEAD /dispatch-head-405/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"),
        "{response}"
    );
    assert_eq!(header_value(&response, "Allow"), Some("POST, OPTIONS"));
    assert!(response.ends_with("\r\n\r\n"), "{response}");
}

#[tokio::test]
async fn a_head_response_reaches_the_wire_without_a_body() {
    register(Method::GET, "/wire-head/items");

    let response = exchange(limits(), |client| {
        send_chunks(
            client,
            vec![b"HEAD /wire-head/items HTTP/1.1\r\nHost: x\r\n\r\n".to_vec()],
        )
    })
    .await;

    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(
        response.contains("Content-Length: 2\r\n"),
        "the length GET would report was lost: {response}"
    );
    assert!(
        !response.contains("\r\n\r\nOK"),
        "the body was written after all: {response}"
    );
}

// ── Preflight ────────────────────────────────────────────────────────────────

/// The exact exchange a browser performs before `fetch`ing with a JSON body.
fn preflight(path: &str, method: &str, origin: &str) -> String {
    format!(
        "OPTIONS {} HTTP/1.1\r\nHost: localhost\r\nOrigin: {}\r\n\
         Access-Control-Request-Method: {}\r\n\
         Access-Control-Request-Headers: content-type\r\n\r\n",
        path, origin, method
    )
}

#[tokio::test]
async fn a_preflight_from_an_allowed_origin_is_granted() {
    register(Method::POST, "/dispatch-preflight/users");

    let cors = CorsConfig::new(&[ORIGIN], false, None);
    let response = dispatch(
        &preflight("/dispatch-preflight/users", "POST", ORIGIN),
        &cors,
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 204 No Content\r\n"),
        "{response}"
    );
    assert_eq!(
        header_value(&response, "Access-Control-Allow-Origin"),
        Some(ORIGIN)
    );
    assert_eq!(
        header_value(&response, "Access-Control-Allow-Methods"),
        Some("POST")
    );
    assert_eq!(
        header_value(&response, "Access-Control-Allow-Headers"),
        Some("content-type")
    );
}

#[tokio::test]
async fn a_preflight_from_an_unlisted_origin_gets_no_cors_headers() {
    register(Method::POST, "/dispatch-preflight-denied/users");

    let cors = CorsConfig::new(&[ORIGIN], false, None);
    let response = dispatch(
        &preflight(
            "/dispatch-preflight-denied/users",
            "POST",
            "http://evil.test",
        ),
        &cors,
    )
    .await;

    // Still a truthful answer about the resource; just no permission attached.
    assert!(
        response.starts_with("HTTP/1.1 204 No Content\r\n"),
        "{response}"
    );
    assert!(header_value(&response, "Access-Control-Allow-Origin").is_none());
    assert_eq!(header_value(&response, "Allow"), Some("POST, OPTIONS"));
}

#[tokio::test]
async fn a_preflight_is_not_answered_when_no_policy_is_configured() {
    register(Method::POST, "/dispatch-preflight-off/users");

    let response = dispatch(
        &preflight("/dispatch-preflight-off/users", "POST", ORIGIN),
        &CorsConfig::disabled(),
    )
    .await;

    assert!(header_value(&response, "Access-Control-Allow-Origin").is_none());
}

// ── CORS headers on real responses ───────────────────────────────────────────

#[tokio::test]
async fn a_handler_response_carries_the_cors_headers() {
    // The second leg: a granted preflight is worthless if the request it
    // authorised comes back without the headers.
    register(Method::GET, "/dispatch-cors/items");

    let cors = CorsConfig::new(&[ORIGIN], false, None);
    let response = dispatch(
        &format!(
            "GET /dispatch-cors/items HTTP/1.1\r\nOrigin: {}\r\n\r\n",
            ORIGIN
        ),
        &cors,
    )
    .await;

    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert_eq!(
        header_value(&response, "Access-Control-Allow-Origin"),
        Some(ORIGIN)
    );
    assert_eq!(header_value(&response, "Vary"), Some("Origin"));
}

#[tokio::test]
async fn a_generated_error_carries_the_cors_headers_too() {
    // A 404 a script cannot read is a "Load failed" in the console rather than a
    // 404, which is the whole reason CORS is hard to debug.
    let cors = CorsConfig::new(&["*"], false, None);
    let response = dispatch(
        &format!(
            "GET /dispatch-cors/nowhere HTTP/1.1\r\nOrigin: {}\r\n\r\n",
            ORIGIN
        ),
        &cors,
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
    assert_eq!(
        header_value(&response, "Access-Control-Allow-Origin"),
        Some("*")
    );
}

#[tokio::test]
async fn a_request_without_an_origin_is_answered_exactly_as_before() {
    register(Method::GET, "/dispatch-cors-plain/items");

    let cors = CorsConfig::new(&["*"], false, None);
    let with_policy = dispatch("GET /dispatch-cors-plain/items HTTP/1.1\r\n\r\n", &cors).await;
    let without_policy = dispatch(
        "GET /dispatch-cors-plain/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    )
    .await;

    assert!(!with_policy.contains("Access-Control-"), "{with_policy}");
    // The Date header differs by the second at worst; compare the framing instead.
    assert_eq!(
        with_policy.split("Date:").next(),
        without_policy.split("Date:").next()
    );
}

#[tokio::test]
async fn cors_headers_reach_the_wire() {
    // Everything above tests `dispatch` against a string. This one goes through
    // the read loop and the socket, which is what the browser actually meets.
    register(Method::GET, "/wire-cors/items");

    let response = exchange_with_cors(
        limits(),
        CorsConfig::new(&[ORIGIN], false, None),
        |client| {
            send_chunks(
                client,
                vec![
                    format!(
                        "GET /wire-cors/items HTTP/1.1\r\nOrigin: {}\r\n\r\n",
                        ORIGIN
                    )
                    .into_bytes(),
                ],
            )
        },
    )
    .await;

    assert!(
        response.contains(&format!("Access-Control-Allow-Origin: {}\r\n", ORIGIN)),
        "{response}"
    );
}
