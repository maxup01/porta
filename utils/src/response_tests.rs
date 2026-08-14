use super::*;

// ── Shared fixtures ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct SimpleBody {
    message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct NumericBody {
    value: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct EmptyBody {}

fn simple_body() -> SimpleBody {
    SimpleBody {
        message: "hello".to_string(),
    }
}

// ── HttpStatus ───────────────────────────────────────────────────────────────

#[test]
fn status_numeric_values_are_correct() {
    assert_eq!(HttpStatus::Ok as u32, 200);
    assert_eq!(HttpStatus::Created as u32, 201);
    assert_eq!(HttpStatus::Accepted as u32, 202);
    assert_eq!(HttpStatus::NoContent as u32, 204);
    assert_eq!(HttpStatus::MovedPermanently as u32, 301);
    assert_eq!(HttpStatus::BadRequest as u32, 400);
    assert_eq!(HttpStatus::Unauthorized as u32, 401);
    assert_eq!(HttpStatus::Forbidden as u32, 403);
    assert_eq!(HttpStatus::NotFound as u32, 404);
    assert_eq!(HttpStatus::MethodNotAllowed as u32, 405);
    assert_eq!(HttpStatus::RequestTimeout as u32, 408);
    assert_eq!(HttpStatus::Conflict as u32, 409);
    assert_eq!(HttpStatus::PayloadTooLarge as u32, 413);
    assert_eq!(HttpStatus::UnprocessableEntity as u32, 422);
    assert_eq!(HttpStatus::TooManyRequests as u32, 429);
    assert_eq!(HttpStatus::InternalServerError as u32, 500);
    assert_eq!(HttpStatus::ServiceUnavailable as u32, 503);
    assert_eq!(HttpStatus::GatewayTimeout as u32, 504);
}

#[test]
fn status_display_reason_phrases_are_correct() {
    let cases = [
        (HttpStatus::Ok, "OK"),
        (HttpStatus::Created, "Created"),
        (HttpStatus::Accepted, "Accepted"),
        (HttpStatus::NoContent, "No Content"),
        (HttpStatus::MovedPermanently, "Moved Permanently"),
        (HttpStatus::BadRequest, "Bad Request"),
        (HttpStatus::Unauthorized, "Unauthorized"),
        (HttpStatus::Forbidden, "Forbidden"),
        (HttpStatus::NotFound, "Not Found"),
        (HttpStatus::MethodNotAllowed, "Method Not Allowed"),
        (HttpStatus::RequestTimeout, "Request Timeout"),
        (HttpStatus::Conflict, "Conflict"),
        (HttpStatus::PayloadTooLarge, "Payload Too Large"),
        (HttpStatus::UnprocessableEntity, "Unprocessable Entity"),
        (HttpStatus::TooManyRequests, "Too Many Requests"),
        (HttpStatus::InternalServerError, "Internal Server Error"),
        (HttpStatus::ServiceUnavailable, "Service Unavailable"),
        (HttpStatus::GatewayTimeout, "Gateway Timeout"),
    ];

    for (status, expected) in cases {
        assert_eq!(
            status.to_string(),
            expected,
            "wrong reason phrase for {expected}"
        );
    }
}

#[test]
fn status_equality_holds() {
    assert_eq!(HttpStatus::Ok, HttpStatus::Ok);
    assert_ne!(HttpStatus::Ok, HttpStatus::Created);
}

#[test]
fn status_is_copy() {
    let a = HttpStatus::NotFound;
    let b = a; // would move if not Copy
    assert_eq!(a, b);
}

#[test]
fn status_can_be_used_as_hash_map_key() {
    use std::collections::HashMap;
    let mut map: HashMap<HttpStatus, &str> = HashMap::new();
    map.insert(HttpStatus::Ok, "success");
    map.insert(HttpStatus::NotFound, "missing");
    assert_eq!(map[&HttpStatus::Ok], "success");
    assert_eq!(map[&HttpStatus::NotFound], "missing");
}

// ── HttpResponse::new / accessors ────────────────────────────────────────────

#[test]
fn new_stores_body_and_status() {
    let resp = HttpResponse::new(simple_body(), HttpStatus::Ok);
    assert_eq!(resp.status(), HttpStatus::Ok);
}

#[test]
fn body_returns_original_value() {
    let resp = HttpResponse::new(NumericBody { value: 42 }, HttpStatus::Created);
    assert_eq!(resp.body(), NumericBody { value: 42 });
}

#[test]
fn status_does_not_consume_response() {
    let resp = HttpResponse::new(simple_body(), HttpStatus::Accepted);
    let _ = resp.status();
    let _ = resp.status(); // callable twice — body still owned
}

#[test]
fn body_consumes_response() {
    let resp = HttpResponse::new(simple_body(), HttpStatus::Ok);
    let body = resp.body();
    assert_eq!(body.message, "hello");
    // resp is moved; this just confirms body() returns T
}

#[test]
fn new_accepts_primitive_body() {
    let resp = HttpResponse::new(99u32, HttpStatus::Ok);
    assert_eq!(resp.body(), 99u32);
}

#[test]
fn new_accepts_string_body() {
    let resp = HttpResponse::new("bare string".to_string(), HttpStatus::Ok);
    assert_eq!(resp.body(), "bare string");
}

#[test]
fn new_accepts_empty_struct_body() {
    let resp = HttpResponse::new(EmptyBody {}, HttpStatus::NoContent);
    assert_eq!(resp.status(), HttpStatus::NoContent);
}

#[test]
fn new_accepts_vec_body() {
    let resp = HttpResponse::new(vec![1u32, 2, 3], HttpStatus::Ok);
    assert_eq!(resp.body(), vec![1, 2, 3]);
}

// ── format_response ───────────────────────────────────────────────────────────

fn formatted(body: impl Serialize + for<'de> Deserialize<'de>, status: HttpStatus) -> String {
    format_response(HttpResponse::new(body, status))
}

#[test]
fn format_starts_with_http_version() {
    let raw = formatted(simple_body(), HttpStatus::Ok);
    assert!(raw.starts_with("HTTP/1.1 "), "missing HTTP/1.1 prefix");
}

#[test]
fn format_status_line_contains_code_and_reason() {
    let raw = formatted(simple_body(), HttpStatus::NotFound);
    assert!(
        raw.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "unexpected status line: {raw}"
    );
}

#[test]
fn format_all_status_codes_appear_in_status_line() {
    let statuses = [
        HttpStatus::Ok,
        HttpStatus::Created,
        HttpStatus::Accepted,
        HttpStatus::NoContent,
        HttpStatus::MovedPermanently,
        HttpStatus::BadRequest,
        HttpStatus::Unauthorized,
        HttpStatus::Forbidden,
        HttpStatus::NotFound,
        HttpStatus::MethodNotAllowed,
        HttpStatus::RequestTimeout,
        HttpStatus::Conflict,
        HttpStatus::PayloadTooLarge,
        HttpStatus::UnprocessableEntity,
        HttpStatus::TooManyRequests,
        HttpStatus::InternalServerError,
        HttpStatus::ServiceUnavailable,
        HttpStatus::GatewayTimeout,
    ];

    for status in statuses {
        let raw = formatted(EmptyBody {}, status);
        let expected_line = format!("HTTP/1.1 {} {}\r\n", status as u32, status);
        assert!(
            raw.starts_with(&expected_line),
            "bad status line for {status}"
        );
    }
}

#[test]
fn format_contains_content_type_header() {
    let raw = formatted(simple_body(), HttpStatus::Ok);
    assert!(
        raw.contains("Content-Type: application/json\r\n"),
        "missing Content-Type header"
    );
}

#[test]
fn format_content_length_matches_body_byte_length() {
    let body = SimpleBody {
        message: "hi".to_string(),
    };
    let raw = formatted(body, HttpStatus::Ok);

    let json_body = raw.split("\r\n\r\n").nth(1).expect("missing body section");
    let expected_len = json_body.len();

    let length_header = format!("Content-Length: {}\r\n", expected_len);
    assert!(raw.contains(&length_header), "Content-Length mismatch");
}

#[test]
fn format_contains_date_header() {
    let raw = formatted(simple_body(), HttpStatus::Ok);
    assert!(raw.contains("Date: "), "missing Date header");
    assert!(raw.contains(" GMT\r\n"), "Date header not in GMT");
}

#[test]
fn format_headers_and_body_separated_by_double_crlf() {
    let raw = formatted(simple_body(), HttpStatus::Ok);
    assert!(raw.contains("\r\n\r\n"), "missing header/body separator");
}

#[test]
fn format_body_is_valid_json() {
    let raw = formatted(simple_body(), HttpStatus::Ok);
    let json_part = raw.split("\r\n\r\n").nth(1).expect("missing body");
    serde_json::from_str::<serde_json::Value>(json_part).expect("body is not valid JSON");
}

#[test]
fn format_body_roundtrips_to_original_value() {
    let original = SimpleBody {
        message: "roundtrip".to_string(),
    };
    let raw = formatted(
        SimpleBody {
            message: "roundtrip".to_string(),
        },
        HttpStatus::Ok,
    );
    let json_part = raw.split("\r\n\r\n").nth(1).expect("missing body");
    let recovered: SimpleBody = serde_json::from_str(json_part).expect("deserialize failed");
    assert_eq!(recovered, original);
}

#[test]
fn format_body_is_last_segment() {
    let body = NumericBody { value: 7 };
    let raw = formatted(body, HttpStatus::Ok);
    assert!(
        raw.ends_with("{\"value\":7}"),
        "JSON body should be the final segment"
    );
}

#[test]
fn format_empty_struct_produces_empty_json_object() {
    // Deliberately not 204: that status suppresses the body entirely, which would
    // make this assert the framing rule rather than the serialization of `{}`.
    let raw = formatted(EmptyBody {}, HttpStatus::Ok);
    let json_part = raw.split("\r\n\r\n").nth(1).expect("missing body");
    assert_eq!(json_part, "{}");
}

#[test]
fn format_vec_body_serializes_as_json_array() {
    let raw = formatted(vec![1u32, 2, 3], HttpStatus::Ok);
    let json_part = raw.split("\r\n\r\n").nth(1).expect("missing body");
    assert_eq!(json_part, "[1,2,3]");
}

// ── status_response ──────────────────────────────────────────────────────────

/// Every status the enum can express, so a new variant that breaks the
/// invariants below fails a test rather than reaching the wire.
const ALL_STATUSES: [HttpStatus; 18] = [
    HttpStatus::Ok,
    HttpStatus::Created,
    HttpStatus::Accepted,
    HttpStatus::NoContent,
    HttpStatus::MovedPermanently,
    HttpStatus::BadRequest,
    HttpStatus::Unauthorized,
    HttpStatus::Forbidden,
    HttpStatus::NotFound,
    HttpStatus::MethodNotAllowed,
    HttpStatus::RequestTimeout,
    HttpStatus::Conflict,
    HttpStatus::PayloadTooLarge,
    HttpStatus::UnprocessableEntity,
    HttpStatus::TooManyRequests,
    HttpStatus::InternalServerError,
    HttpStatus::ServiceUnavailable,
    HttpStatus::GatewayTimeout,
];

/// The four statuses the server generates on its own, without a handler.
/// These are the responses previously hand-rolled inside `quote!` blocks and
/// therefore unreachable from any test.
const SERVER_GENERATED_STATUSES: [HttpStatus; 5] = [
    HttpStatus::BadRequest,
    HttpStatus::NotFound,
    HttpStatus::MethodNotAllowed,
    HttpStatus::RequestTimeout,
    HttpStatus::PayloadTooLarge,
];

fn split_message(raw: &str) -> (&str, &str) {
    let separator = raw.find("\r\n\r\n").expect("missing header/body separator");
    (&raw[..separator], &raw[separator + 4..])
}

#[test]
fn status_response_status_line_matches_code_and_reason() {
    for status in ALL_STATUSES {
        let raw = status_response(status);
        let expected_line = format!("HTTP/1.1 {} {}\r\n", status as u32, status);

        assert!(
            raw.starts_with(&expected_line),
            "bad status line for {status}"
        );
    }
}

/// Every status except `204`, which is defined to carry no content and so has
/// neither a body nor the headers describing one.
fn body_bearing_statuses() -> impl Iterator<Item = HttpStatus> {
    ALL_STATUSES
        .into_iter()
        .filter(|status| *status != HttpStatus::NoContent)
}

#[test]
fn status_response_body_is_the_reason_phrase() {
    for status in body_bearing_statuses() {
        let raw = status_response(status);
        let (_, body) = split_message(&raw);

        assert_eq!(body, status.to_string(), "wrong body for {status}");
    }
}

#[test]
fn status_response_content_length_matches_body_byte_length() {
    for status in body_bearing_statuses() {
        let raw = status_response(status);
        let (headers, body) = split_message(&raw);
        let expected = format!("Content-Length: {}\r\n", body.len());

        assert!(
            headers.contains(&expected),
            "Content-Length does not match body length for {status}"
        );
    }
}

#[test]
fn status_response_uses_text_plain() {
    let raw = status_response(HttpStatus::BadRequest);

    assert!(
        raw.contains("Content-Type: text/plain\r\n"),
        "missing or wrong Content-Type: {raw}"
    );
}

#[test]
fn status_response_contains_date_header() {
    let raw = status_response(HttpStatus::NotFound);

    assert!(raw.contains("Date: "), "missing Date header");
    assert!(raw.contains(" GMT\r\n"), "Date header not in GMT");
}

#[test]
fn status_response_body_is_the_final_segment() {
    let raw = status_response(HttpStatus::MethodNotAllowed);

    assert!(
        raw.ends_with("\r\n\r\nMethod Not Allowed"),
        "body is not the last segment: {raw}"
    );
}

#[test]
fn status_response_body_contains_no_crlf() {
    for status in ALL_STATUSES {
        let raw = status_response(status);
        let (_, body) = split_message(&raw);

        assert!(
            !body.contains("\r\n"),
            "reason phrase for {status} would be read as a header boundary"
        );
    }
}

#[test]
fn status_response_matches_expected_shape_for_server_generated_errors() {
    for status in SERVER_GENERATED_STATUSES {
        let raw = status_response(status);
        let reason = status.to_string();
        let expected_prefix = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nDate: ",
            status as u32,
            reason,
            reason.len()
        );

        assert!(
            raw.starts_with(&expected_prefix),
            "unexpected header block for {status}: {raw}"
        );
        assert!(
            raw.ends_with(&format!("\r\n\r\n{reason}")),
            "unexpected body for {status}: {raw}"
        );
    }
}

// ── 204 No Content framing ───────────────────────────────────────────────────

#[test]
fn status_response_204_has_an_empty_body() {
    let raw = status_response(HttpStatus::NoContent);
    let (_, body) = split_message(&raw);

    assert_eq!(body, "", "204 must not carry content");
}

#[test]
fn status_response_204_omits_content_headers() {
    let raw = status_response(HttpStatus::NoContent);

    assert!(
        !raw.contains("Content-Length"),
        "204 must not describe content it does not have: {raw}"
    );
    assert!(
        !raw.contains("Content-Type"),
        "204 must not declare a content type: {raw}"
    );
}

#[test]
fn format_response_204_discards_the_serialized_body() {
    let raw = formatted(simple_body(), HttpStatus::NoContent);
    let (_, body) = split_message(&raw);

    assert_eq!(body, "", "204 must drop the handler's body");
    assert!(!raw.contains("hello"), "serialized body leaked into a 204");
}

#[test]
fn format_response_204_still_carries_date() {
    let raw = formatted(simple_body(), HttpStatus::NoContent);

    assert!(raw.contains("Date: "), "204 lost its Date header");
}

// ── Connection handling ──────────────────────────────────────────────────────
//
// Serialization emits no `Connection` header at all. An HTTP/1.1 connection is
// persistent unless something says otherwise, so silence here is the accurate
// answer: whether this particular response is the last one is a fact about the
// connection, and only the connection loop in `server` holds it. These tests
// exist to catch the header being reintroduced by a well-meaning edit.

#[test]
fn no_status_response_decides_the_connection_lifetime() {
    for status in ALL_STATUSES {
        let raw = status_response(status);

        assert!(
            !raw.contains("Connection:"),
            "{status} pre-empts the connection loop's decision"
        );
    }
}

#[test]
fn format_response_leaves_the_connection_lifetime_to_the_server() {
    let raw = formatted(simple_body(), HttpStatus::Ok);

    assert!(
        !raw.contains("Connection:"),
        "a serialized response cannot know whether it is the last one"
    );
}

#[test]
fn a_closing_response_is_built_by_adding_the_header() {
    let raw = with_headers(
        &formatted(simple_body(), HttpStatus::Ok),
        &[("Connection", "close".to_string())],
    );

    assert!(
        raw.contains("Connection: close\r\n"),
        "the seam the connection loop closes through is broken: {raw}"
    );
}

#[test]
fn status_response_and_format_response_share_a_header_set() {
    let error = status_response(HttpStatus::NotFound);
    let success = formatted(simple_body(), HttpStatus::Ok);

    for header in ["Content-Type: ", "Content-Length: ", "Date: "] {
        assert!(error.contains(header), "error response missing {header}");
        assert!(success.contains(header), "success response missing {header}");
    }
}

// ── with_headers ─────────────────────────────────────────────────────────────

#[test]
fn with_headers_inserts_after_the_status_line() {
    let raw = with_headers(
        &status_response(HttpStatus::NotFound),
        &[("Allow", "GET, POST".to_string())],
    );

    assert!(
        raw.starts_with("HTTP/1.1 404 Not Found\r\nAllow: GET, POST\r\n"),
        "{raw}"
    );
}

#[test]
fn with_headers_keeps_the_body_and_the_existing_headers() {
    let response = formatted(simple_body(), HttpStatus::Ok);
    let raw = with_headers(&response, &[("Vary", "Origin".to_string())]);

    let (_, original_body) = response
        .split_once("\r\n\r\n")
        .expect("no header/body separator");
    let (_, body) = raw.split_once("\r\n\r\n").expect("no header/body separator");

    assert_eq!(body, original_body);
    assert!(raw.contains("Content-Type: application/json\r\n"));
    assert!(raw.contains("Content-Length: "));
}

#[test]
fn with_headers_preserves_a_204s_missing_body() {
    // A 204 carries neither body nor content headers, and adding a header must
    // not accidentally give it a body separator it did not have.
    let raw = with_headers(
        &status_response(HttpStatus::NoContent),
        &[("Access-Control-Allow-Origin", "*".to_string())],
    );

    assert!(raw.ends_with("\r\n\r\n"), "{raw}");
    assert!(!raw.contains("Content-Length"), "{raw}");
}

#[test]
fn with_headers_adds_every_header_in_order() {
    let raw = with_headers(
        &status_response(HttpStatus::NoContent),
        &[
            ("Access-Control-Allow-Origin", "http://a.test".to_string()),
            ("Vary", "Origin".to_string()),
        ],
    );

    let origin_at = raw.find("Access-Control-Allow-Origin").expect("origin header");
    let vary_at = raw.find("Vary").expect("vary header");

    assert!(origin_at < vary_at, "{raw}");
}

#[test]
fn with_headers_returns_the_response_unchanged_when_there_is_nothing_to_add() {
    let response = status_response(HttpStatus::Ok);

    assert_eq!(with_headers(&response, &[]), response);
}

#[test]
fn with_headers_leaves_a_response_with_no_status_line_alone() {
    // Nothing produces this, but corrupting an already-broken message further
    // would only make the cause harder to see.
    assert_eq!(
        with_headers("garbage", &[("Allow", "GET".to_string())]),
        "garbage"
    );
}
