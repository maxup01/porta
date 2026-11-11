use super::*;

// ── extract_request_body ──────────────────────────────────────────────────────

// --- happy path ---

#[test]
fn extracts_json_body() {
    let body = extract_request_body(
        b"POST /users HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"name\":\"Alice\"}",
    );
    assert_eq!(body, Some(&b"{\"name\":\"Alice\"}"[..]));
}

#[test]
fn extracts_plain_text_body() {
    let body = extract_request_body(b"POST /users HTTP/1.1\r\n\r\nhello world");
    assert_eq!(body, Some(&b"hello world"[..]));
}

#[test]
fn extracts_body_with_multiple_headers() {
    let body = extract_request_body(
        b"POST /users HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
    );
    assert_eq!(body, Some(&b"{}"[..]));
}

#[test]
fn extracts_body_with_newlines_inside() {
    let body = extract_request_body(b"POST /users HTTP/1.1\r\n\r\nline1\nline2\nline3");
    assert_eq!(body, Some(&b"line1\nline2\nline3"[..]));
}

#[test]
fn extracts_single_character_body() {
    let body = extract_request_body(b"POST /users HTTP/1.1\r\n\r\nX");
    assert_eq!(body, Some(&b"X"[..]));
}

// --- no body ---

#[test]
fn empty_body_after_separator_returns_none() {
    let body = extract_request_body(b"GET /users HTTP/1.1\r\nAccept: */*\r\n\r\n");
    assert!(body.is_none());
}

#[test]
fn no_separator_returns_none() {
    let body = extract_request_body(b"GET /users HTTP/1.1");
    assert!(body.is_none());
}

#[test]
fn empty_request_returns_none() {
    let body = extract_request_body(b"");
    assert!(body.is_none());
}

// --- edge cases ---

#[test]
fn separator_at_start_with_body() {
    let body = extract_request_body(b"\r\n\r\nbody");
    assert_eq!(body, Some(&b"body"[..]));
}

#[test]
fn separator_at_start_without_body_returns_none() {
    let body = extract_request_body(b"\r\n\r\n");
    assert!(body.is_none());
}

#[test]
fn only_partial_separator_returns_none() {
    let body = extract_request_body(b"POST /users HTTP/1.1\r\n\r");
    assert!(body.is_none());
}

#[test]
fn multiple_separators_body_starts_after_first() {
    let body = extract_request_body(b"POST /users HTTP/1.1\r\n\r\nfirst\r\n\r\nsecond");
    assert_eq!(body, Some(&b"first\r\n\r\nsecond"[..]));
}

// ── extract_request_head ──────────────────────────────────────────────────────

#[test]
fn head_includes_the_separator() {
    let head = extract_request_head(b"POST /users HTTP/1.1\r\nHost: x\r\n\r\n{}");
    assert_eq!(head, Some("POST /users HTTP/1.1\r\nHost: x\r\n\r\n"));
}

#[test]
fn head_without_separator_is_the_whole_request() {
    assert_eq!(extract_request_head(b"GARBAGE"), Some("GARBAGE"));
}

#[test]
fn head_is_readable_when_the_body_is_not_utf8() {
    let head = extract_request_head(b"POST / HTTP/1.1\r\n\r\n\xa1\xff\x00");
    assert_eq!(head, Some("POST / HTTP/1.1\r\n\r\n"));
}

#[test]
fn head_that_is_not_utf8_returns_none() {
    assert!(extract_request_head(b"POST /\xff HTTP/1.1\r\n\r\n").is_none());
}

#[test]
fn binary_body_is_returned_verbatim() {
    let body = extract_request_body(b"POST / HTTP/1.1\r\n\r\n\xa1\xff\x00+");
    assert_eq!(body, Some(&b"\xa1\xff\x00+"[..]));
}

// ── is_cbor ───────────────────────────────────────────────────────────────────

#[test]
fn cbor_content_type_is_recognised() {
    assert!(is_cbor(
        "POST / HTTP/1.1\r\nContent-Type: application/cbor\r\n\r\n"
    ));
}

#[test]
fn cbor_content_type_ignores_case_and_parameters() {
    assert!(is_cbor(
        "POST / HTTP/1.1\r\ncontent-type: Application/CBOR ; x=1\r\n\r\n"
    ));
}

#[test]
fn json_content_type_is_not_cbor() {
    assert!(!is_cbor(
        "POST / HTTP/1.1\r\nContent-Type: application/json\r\n\r\n"
    ));
}

#[test]
fn missing_content_type_is_not_cbor() {
    assert!(!is_cbor("POST / HTTP/1.1\r\n\r\n"));
}

#[test]
fn media_type_with_cbor_suffix_is_not_cbor() {
    // A structured-syntax suffix names a different media type, not this one.
    assert!(!is_cbor(
        "POST / HTTP/1.1\r\nContent-Type: application/senml+cbor\r\n\r\n"
    ));
}

// ── decode_cbor ───────────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Debug, PartialEq)]
struct Item {
    num: u64,
    name: String,
}

#[test]
fn decodes_cbor_struct() {
    let mut bytes = Vec::new();
    ciborium::into_writer(&serde_json::json!({"num": 5, "name": "a+b"}), &mut bytes).unwrap();

    let item: Item = decode_cbor(&bytes).unwrap();
    assert_eq!(
        item,
        Item {
            num: 5,
            name: "a+b".to_string()
        }
    );
}

#[test]
fn decodes_cbor_primitives() {
    assert_eq!(decode_cbor::<u64>(&[0x05]).unwrap(), 5);
    assert_eq!(
        decode_cbor::<String>(&[0x63, b'a', b'b', b'c']).unwrap(),
        "abc"
    );
}

#[test]
fn malformed_cbor_is_an_error() {
    assert!(decode_cbor::<Item>(&[0xff, 0x00]).is_err());
}

#[test]
fn cbor_of_the_wrong_shape_is_an_error() {
    // A text string where a struct is expected.
    assert!(decode_cbor::<Item>(&[0x63, b'a', b'b', b'c']).is_err());
}
