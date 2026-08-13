/// Looks up a header value in a raw HTTP request string.
///
/// The request line is skipped, and only the header block — everything before
/// the `\r\n\r\n` separator — is searched, so a body that happens to contain a
/// line shaped like a header can never be mistaken for one.
///
/// Header names are case-insensitive per RFC 9110 §5.1, so `"origin"` finds
/// `Origin:` as readily as `ORIGIN:`. The value is returned with surrounding
/// whitespace trimmed.
///
/// When a header appears more than once the first occurrence wins. That is not
/// the general HTTP rule — repeated headers are defined to combine — but the
/// headers read here (`Origin`, `Access-Control-Request-Method`,
/// `Access-Control-Request-Headers`) are single-valued, and a request carrying
/// two of them is a client error rather than something to merge.
///
/// # Arguments
///
/// * `request` - A raw HTTP request string.
/// * `name` - The header name to look for, without the colon.
///
/// # Returns
///
/// - `Some(&str)` with the trimmed value if the header is present.
/// - `None` if no header block line carries that name.
///
/// # Examples
///
/// ```
/// use utils::request::header::get_header;
///
/// let request = "GET /users HTTP/1.1\r\nHost: example\r\nOrigin: http://localhost:1420\r\n\r\n";
///
/// assert_eq!(get_header(request, "origin"), Some("http://localhost:1420"));
/// assert_eq!(get_header(request, "Host"), Some("example"));
/// assert_eq!(get_header(request, "accept"), None);
/// ```
pub fn get_header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    let header_block = match request.split_once("\r\n\r\n") {
        Some((header_block, _body)) => header_block,
        // No separator yet: the whole string is header block at most. Reading it
        // anyway costs nothing and keeps this usable on a partial request.
        None => request,
    };

    header_block
        .split("\r\n")
        // The request line is not a header, and `GET /a:b HTTP/1.1` would
        // otherwise parse as one named `GET /a`.
        .skip(1)
        .find_map(|line| {
            let (header_name, value) = line.split_once(':')?;

            header_name
                .trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim())
        })
}

#[cfg(test)]
#[path = "header_tests.rs"]
mod tests;
