use utils::request::request_body::extract_request_body;

const REQUEST_WITH_REQUEST_BODY: &str = "POST / HTTP/1.1
\r\nHost: example.com
\r\nContent-Type: application/json
\r\nContent-Length: 27
\r\n\r\n{\"key\":\"value\"}";

const REQUEST_WITHOUT_REQUEST_BODY: &str = "POST / HTTP/1.1
\r\nHost: example.com
\r\nContent-Type: application/json
\r\nContent-Length: 27";

#[test] 
pub fn test_extract_request_body() {
    
    let body = extract_request_body(REQUEST_WITH_REQUEST_BODY);
    let none = extract_request_body(REQUEST_WITHOUT_REQUEST_BODY);

    assert!(body.is_some());
    assert!(none.is_none());
}