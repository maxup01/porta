use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
async fn send_chunks(mut client: tokio::io::DuplexStream, chunks: Vec<Vec<u8>>) -> String {
    for chunk in chunks {
        client.write_all(&chunk).await.expect("write failed");
        // Yields so the server side actually observes a short read rather than
        // finding everything already buffered.
        tokio::task::yield_now().await;
    }

    let mut response = Vec::new();
    let _ = client.read_to_end(&mut response).await;

    String::from_utf8_lossy(&response).to_string()
}

// ── Assembly across reads ────────────────────────────────────────────────────

#[tokio::test]
async fn request_split_mid_header_is_reassembled() {
    // The split falls inside a header name, the case a single `read()` got wrong.
    let response = exchange(Limits::default(), |client| {
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
    let response = exchange(Limits::default(), |client| {
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
        ..Limits::default()
    };

    let response = exchange(limits, |client| {
        let mut request = b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 4096\r\n\r\n".to_vec();
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
        ..Limits::default()
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
    let response = exchange(Limits::default(), |client| {
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
    let response = exchange(Limits::default(), |client| {
        let mut request = b"POST /nothing-registered HTTP/1.1\r\nContent-Length: 4\r\n\r\n".to_vec();
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

// ── Deadline ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_request_that_never_completes_is_answered_with_408() {
    let limits = Limits {
        request_timeout: std::time::Duration::from_millis(50),
        ..Limits::default()
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
        ..Limits::default()
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
    let response = exchange(Limits::default(), |mut client| async move {
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
async fn generated_errors_carry_connection_close() {
    let response = exchange(Limits::default(), |client| {
        send_chunks(client, vec![b"GET /nothing-registered HTTP/1.1\r\n\r\n".to_vec()])
    })
    .await;

    assert!(response.contains("Connection: close\r\n"), "{response}");
    assert!(response.contains("Content-Type: text/plain\r\n"), "{response}");
}

// ── dispatch, without a transport ────────────────────────────────────────────

/// Registers a route so the dispatch tests below have something to resolve.
///
/// The route tables are process-global, so every path here is prefixed to keep
/// these tests from colliding with each other or with `utils`'.
fn register(method: Method, path: &str) {
    fn handler(_: &str) -> String {
        utils::response::status_response(HttpStatus::Ok)
    }

    utils::request::route::register_route(method, path, handler);
}

const ORIGIN: &str = "http://localhost:1420";

fn header_value<'a>(response: &'a str, name: &str) -> Option<&'a str> {
    utils::request::header::get_header(
        // `get_header` skips the first line, and a response's first line is its
        // status line — the same shape as a request line, for this purpose.
        response,
        name,
    )
}

#[test]
fn dispatch_answers_an_unroutable_path_with_404() {
    let response = dispatch(
        "GET /nothing-registered HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    );

    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"), "{response}");
}

#[test]
fn dispatch_answers_a_malformed_request_line_with_400() {
    let response = dispatch("GARBAGE", &CorsConfig::disabled());

    assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"), "{response}");
}

#[test]
fn dispatch_answers_an_unsupported_verb_with_404_when_nothing_serves_the_path() {
    // CONNECT does not parse as a Method, so no handler resolves; with no route
    // registered for the path either, that is a 404 rather than a 405.
    let response = dispatch(
        "CONNECT /nothing-registered HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    );

    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"), "{response}");
}

// ── 405 and Allow ────────────────────────────────────────────────────────────

#[test]
fn a_405_names_the_methods_that_are_served() {
    register(Method::GET, "/dispatch-allow/items");
    register(Method::DELETE, "/dispatch-allow/items");

    let response = dispatch(
        "POST /dispatch-allow/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    );

    assert!(
        response.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"),
        "{response}"
    );
    assert_eq!(header_value(&response, "Allow"), Some("GET, DELETE, OPTIONS"));
}

// ── OPTIONS ──────────────────────────────────────────────────────────────────

#[test]
fn a_plain_options_request_is_answered_with_204_and_allow() {
    // The bug this closes: OPTIONS is not a `Method`, so this used to fall through
    // the routing table and answer 404 for a path that plainly exists.
    register(Method::GET, "/dispatch-options/items");
    register(Method::POST, "/dispatch-options/items");

    let response = dispatch(
        "OPTIONS /dispatch-options/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    );

    assert!(
        response.starts_with("HTTP/1.1 204 No Content\r\n"),
        "{response}"
    );
    assert_eq!(header_value(&response, "Allow"), Some("GET, POST, OPTIONS"));
}

#[test]
fn an_options_request_for_an_unserved_path_is_still_a_404() {
    let response = dispatch(
        "OPTIONS /dispatch-options/nothing-here HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    );

    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"), "{response}");
}

#[test]
fn an_options_request_ignores_the_query_string() {
    register(Method::GET, "/dispatch-options-query/items");

    let response = dispatch(
        "OPTIONS /dispatch-options-query/items?id=5 HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    );

    assert!(
        response.starts_with("HTTP/1.1 204 No Content\r\n"),
        "{response}"
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

#[test]
fn a_preflight_from_an_allowed_origin_is_granted() {
    register(Method::POST, "/dispatch-preflight/users");

    let cors = CorsConfig::new(&[ORIGIN], false, None);
    let response = dispatch(&preflight("/dispatch-preflight/users", "POST", ORIGIN), &cors);

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

#[test]
fn a_preflight_from_an_unlisted_origin_gets_no_cors_headers() {
    register(Method::POST, "/dispatch-preflight-denied/users");

    let cors = CorsConfig::new(&[ORIGIN], false, None);
    let response = dispatch(
        &preflight("/dispatch-preflight-denied/users", "POST", "http://evil.test"),
        &cors,
    );

    // Still a truthful answer about the resource; just no permission attached.
    assert!(
        response.starts_with("HTTP/1.1 204 No Content\r\n"),
        "{response}"
    );
    assert!(header_value(&response, "Access-Control-Allow-Origin").is_none());
    assert_eq!(header_value(&response, "Allow"), Some("POST, OPTIONS"));
}

#[test]
fn a_preflight_is_not_answered_when_no_policy_is_configured() {
    register(Method::POST, "/dispatch-preflight-off/users");

    let response = dispatch(
        &preflight("/dispatch-preflight-off/users", "POST", ORIGIN),
        &CorsConfig::disabled(),
    );

    assert!(header_value(&response, "Access-Control-Allow-Origin").is_none());
}

// ── CORS headers on real responses ───────────────────────────────────────────

#[test]
fn a_handler_response_carries_the_cors_headers() {
    // The second leg: a granted preflight is worthless if the request it
    // authorised comes back without the headers.
    register(Method::GET, "/dispatch-cors/items");

    let cors = CorsConfig::new(&[ORIGIN], false, None);
    let response = dispatch(
        &format!("GET /dispatch-cors/items HTTP/1.1\r\nOrigin: {}\r\n\r\n", ORIGIN),
        &cors,
    );

    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert_eq!(
        header_value(&response, "Access-Control-Allow-Origin"),
        Some(ORIGIN)
    );
    assert_eq!(header_value(&response, "Vary"), Some("Origin"));
}

#[test]
fn a_generated_error_carries_the_cors_headers_too() {
    // A 404 a script cannot read is a "Load failed" in the console rather than a
    // 404, which is the whole reason CORS is hard to debug.
    let cors = CorsConfig::new(&["*"], false, None);
    let response = dispatch(
        &format!("GET /dispatch-cors/nowhere HTTP/1.1\r\nOrigin: {}\r\n\r\n", ORIGIN),
        &cors,
    );

    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"), "{response}");
    assert_eq!(header_value(&response, "Access-Control-Allow-Origin"), Some("*"));
}

#[test]
fn a_request_without_an_origin_is_answered_exactly_as_before() {
    register(Method::GET, "/dispatch-cors-plain/items");

    let cors = CorsConfig::new(&["*"], false, None);
    let with_policy = dispatch("GET /dispatch-cors-plain/items HTTP/1.1\r\n\r\n", &cors);
    let without_policy = dispatch(
        "GET /dispatch-cors-plain/items HTTP/1.1\r\n\r\n",
        &CorsConfig::disabled(),
    );

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
        Limits::default(),
        CorsConfig::new(&[ORIGIN], false, None),
        |client| {
            send_chunks(
                client,
                vec![
                    format!("GET /wire-cors/items HTTP/1.1\r\nOrigin: {}\r\n\r\n", ORIGIN)
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
