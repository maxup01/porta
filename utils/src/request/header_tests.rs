use super::get_header;

#[test]
fn finds_a_header_regardless_of_case() {
    let request = "GET / HTTP/1.1\r\nOrigin: http://localhost:1420\r\n\r\n";

    assert_eq!(get_header(request, "origin"), Some("http://localhost:1420"));
    assert_eq!(get_header(request, "ORIGIN"), Some("http://localhost:1420"));
    assert_eq!(get_header(request, "Origin"), Some("http://localhost:1420"));
}

#[test]
fn trims_surrounding_whitespace_from_the_value() {
    let request = "GET / HTTP/1.1\r\nOrigin:   http://a.test  \r\n\r\n";

    assert_eq!(get_header(request, "origin"), Some("http://a.test"));
}

#[test]
fn a_missing_header_is_none() {
    let request = "GET / HTTP/1.1\r\nHost: example\r\n\r\n";

    assert!(get_header(request, "origin").is_none());
}

#[test]
fn the_request_line_is_not_searched() {
    // `GET /a:b HTTP/1.1` splits on a colon just as a header does. Skipping the
    // first line is what stops it resolving as one.
    let request = "GET /a:b HTTP/1.1\r\nHost: example\r\n\r\n";

    assert!(get_header(request, "GET /a").is_none());
}

#[test]
fn the_body_is_not_searched() {
    let request = "POST /users HTTP/1.1\r\nHost: example\r\n\r\nOrigin: http://evil.test";

    assert!(get_header(request, "origin").is_none());
}

#[test]
fn a_value_containing_a_colon_is_returned_whole() {
    let request = "GET / HTTP/1.1\r\nOrigin: http://localhost:1420\r\n\r\n";

    assert_eq!(get_header(request, "origin"), Some("http://localhost:1420"));
}

#[test]
fn the_first_occurrence_wins() {
    let request = "GET / HTTP/1.1\r\nOrigin: http://a.test\r\nOrigin: http://b.test\r\n\r\n";

    assert_eq!(get_header(request, "origin"), Some("http://a.test"));
}

#[test]
fn headers_are_found_before_the_block_is_terminated() {
    let request = "GET / HTTP/1.1\r\nOrigin: http://a.test";

    assert_eq!(get_header(request, "origin"), Some("http://a.test"));
}

#[test]
fn an_empty_value_is_some_and_empty() {
    let request = "GET / HTTP/1.1\r\nOrigin:\r\n\r\n";

    assert_eq!(get_header(request, "origin"), Some(""));
}
