/// Extracts the body from a raw HTTP request string.
///
/// Locates the header/body separator (`\r\n\r\n`) and returns everything
/// after it as the request body.
///
/// # Arguments
///
/// * `request` - A raw HTTP request string, e.g.:
///   ```text
///   POST /users HTTP/1.1\r\n
///   Content-Type: application/json\r\n
///   \r\n
///   {"name":"Alice"}
///   ```
///
/// # Returns
///
/// - `Some(String)` containing the body if `\r\n\r\n` is present and is
///   followed by at least one character.
/// - `None` if the separator is absent or the body is empty.
///
/// # Examples
///
/// ```
/// use utils::request::request_body::extract_request_body;
///
/// let body = extract_request_body("POST /users HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"name\":\"Alice\"}");
/// assert_eq!(body, Some("{\"name\":\"Alice\"}".to_string()));
///
/// // No body after the separator returns None
/// let no_body = extract_request_body("GET /users HTTP/1.1\r\nAccept: */*\r\n\r\n");
/// assert!(no_body.is_none());
///
/// // No separator at all returns None
/// let no_sep = extract_request_body("GET /users HTTP/1.1");
/// assert!(no_sep.is_none());
/// ```
pub fn extract_request_body(request: &str) -> Option<String> {
    let crlf_start_idx = request.find("\r\n\r\n")?; // + 4
    let request_after_crlf = &request[crlf_start_idx..];

    if request_after_crlf.len() == 4 {
        return None;
    }

    let body_start_idx: usize = crlf_start_idx + 4;

    Some(request[body_start_idx..].to_string())
}
