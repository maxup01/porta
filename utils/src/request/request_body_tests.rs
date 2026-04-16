use super::*;

// ── extract_request_body ──────────────────────────────────────────────────────

// --- happy path ---

#[test]
fn extracts_json_body() {
    let body = extract_request_body(
        "POST /users HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"name\":\"Alice\"}",
    );
    assert_eq!(body, Some("{\"name\":\"Alice\"}".to_string()));
}

#[test]
fn extracts_plain_text_body() {
    let body = extract_request_body("POST /users HTTP/1.1\r\n\r\nhello world");
    assert_eq!(body, Some("hello world".to_string()));
}

#[test]
fn extracts_body_with_multiple_headers() {
    let body = extract_request_body(
        "POST /users HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
    );
    assert_eq!(body, Some("{}".to_string()));
}

#[test]
fn extracts_body_with_newlines_inside() {
    let body = extract_request_body("POST /users HTTP/1.1\r\n\r\nline1\nline2\nline3");
    assert_eq!(body, Some("line1\nline2\nline3".to_string()));
}

#[test]
fn extracts_single_character_body() {
    let body = extract_request_body("POST /users HTTP/1.1\r\n\r\nX");
    assert_eq!(body, Some("X".to_string()));
}

// --- no body ---

#[test]
fn empty_body_after_separator_returns_none() {
    let body = extract_request_body("GET /users HTTP/1.1\r\nAccept: */*\r\n\r\n");
    assert!(body.is_none());
}

#[test]
fn no_separator_returns_none() {
    let body = extract_request_body("GET /users HTTP/1.1");
    assert!(body.is_none());
}

#[test]
fn empty_request_returns_none() {
    let body = extract_request_body("");
    assert!(body.is_none());
}

// --- edge cases ---

#[test]
fn separator_at_start_with_body() {
    let body = extract_request_body("\r\n\r\nbody");
    assert_eq!(body, Some("body".to_string()));
}

#[test]
fn separator_at_start_without_body_returns_none() {
    let body = extract_request_body("\r\n\r\n");
    assert!(body.is_none());
}

#[test]
fn only_partial_separator_returns_none() {
    let body = extract_request_body("POST /users HTTP/1.1\r\n\r");
    assert!(body.is_none());
}

#[test]
fn multiple_separators_body_starts_after_first() {
    let body = extract_request_body("POST /users HTTP/1.1\r\n\r\nfirst\r\n\r\nsecond");
    assert_eq!(body, Some("first\r\n\r\nsecond".to_string()));
}
