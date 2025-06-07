pub fn extract_request_body(request: &str) -> Option<String> {
    let body_start = request.find("\r\n\r\n")? + 4;
    Some(request[body_start..].to_string())
}