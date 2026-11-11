use crate::request::header::get_header;
use serde::de::DeserializeOwned;

/// The media type a CBOR body is sent under (RFC 8949 §9.1).
const CBOR_MEDIA_TYPE: &str = "application/cbor";

/// The separator between the header block and the body.
const HEAD_TERMINATOR: &[u8] = b"\r\n\r\n";

/// Extracts the body from a raw HTTP request.
///
/// Locates the header/body separator (`\r\n\r\n`) and returns everything
/// after it as the request body.
///
/// The body is bytes rather than text because nothing about HTTP makes it text:
/// a CBOR body is binary, and only the header block is required to be readable.
/// Whether the bytes are UTF-8 is for the decoder chosen by `Content-Type` to say.
///
/// # Arguments
///
/// * `request` - A raw HTTP request, e.g.:
///   ```text
///   POST /users HTTP/1.1\r\n
///   Content-Type: application/json\r\n
///   \r\n
///   {"name":"Alice"}
///   ```
///
/// # Returns
///
/// - `Some(&[u8])` containing the body if `\r\n\r\n` is present and is
///   followed by at least one byte.
/// - `None` if the separator is absent or the body is empty.
///
/// # Examples
///
/// ```
/// use utils::request::request_body::extract_request_body;
///
/// let body = extract_request_body(b"POST /users HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"name\":\"Alice\"}");
/// assert_eq!(body, Some(&b"{\"name\":\"Alice\"}"[..]));
///
/// // No body after the separator returns None
/// let no_body = extract_request_body(b"GET /users HTTP/1.1\r\nAccept: */*\r\n\r\n");
/// assert!(no_body.is_none());
///
/// // No separator at all returns None
/// let no_sep = extract_request_body(b"GET /users HTTP/1.1");
/// assert!(no_sep.is_none());
/// ```
pub fn extract_request_body(request: &[u8]) -> Option<&[u8]> {
    let body_start_idx = head_len(request)?;
    let body = &request[body_start_idx..];

    if body.is_empty() {
        return None;
    }

    Some(body)
}

/// Extracts the header block — request line, headers and the `\r\n\r\n` that ends
/// them — from a raw HTTP request, as text.
///
/// Everything that routes a request reads only this part, and all of it is text, so
/// this is the one place a request is required to be UTF-8. The body is left as
/// bytes; see [`extract_request_body`].
///
/// A request with no separator is all head, which keeps a truncated or malformed
/// request reaching the parsers that answer it `400`, as it did when the whole
/// request was a string.
///
/// # Returns
///
/// - `Some(&str)` with the header block, separator included.
/// - `None` if the header block is not UTF-8.
///
/// # Examples
///
/// ```
/// use utils::request::request_body::extract_request_head;
///
/// let request = b"POST /blob HTTP/1.1\r\nContent-Type: application/cbor\r\n\r\n\xa1\x63num\x05";
/// assert_eq!(
///     extract_request_head(request),
///     Some("POST /blob HTTP/1.1\r\nContent-Type: application/cbor\r\n\r\n")
/// );
///
/// // A body that is not UTF-8 does not make the head unreadable
/// assert!(extract_request_head(b"POST / HTTP/1.1\r\n\r\n\xff").is_some());
///
/// // A head that is not UTF-8 is unreadable
/// assert!(extract_request_head(b"POST /\xff HTTP/1.1\r\n\r\n").is_none());
/// ```
pub fn extract_request_head(request: &[u8]) -> Option<&str> {
    let head_end_idx = head_len(request).unwrap_or(request.len());

    std::str::from_utf8(&request[..head_end_idx]).ok()
}

/// Whether the request declares a CBOR body.
///
/// Only the media type is compared — parameters such as `; charset=…` are ignored —
/// and case-insensitively, as RFC 9110 §8.3.1 requires.
///
/// # Examples
///
/// ```
/// use utils::request::request_body::is_cbor;
///
/// assert!(is_cbor("POST / HTTP/1.1\r\nContent-Type: application/cbor\r\n\r\n"));
/// assert!(is_cbor("POST / HTTP/1.1\r\ncontent-type: Application/CBOR; x=1\r\n\r\n"));
/// assert!(!is_cbor("POST / HTTP/1.1\r\nContent-Type: application/json\r\n\r\n"));
/// assert!(!is_cbor("POST / HTTP/1.1\r\n\r\n"));
/// ```
pub fn is_cbor(request_head: &str) -> bool {
    get_header(request_head, "Content-Type").is_some_and(|content_type| {
        let media_type = match content_type.split_once(';') {
            Some((media_type, _parameters)) => media_type,
            None => content_type,
        };

        media_type.trim().eq_ignore_ascii_case(CBOR_MEDIA_TYPE)
    })
}

/// Decodes a CBOR body into `T`.
///
/// CBOR is self-describing and typed, so unlike a text body no guessing is needed:
/// a `String`, a number and a struct are each decoded through serde as written.
///
/// # Examples
///
/// ```
/// use utils::request::request_body::decode_cbor;
///
/// // {"num": 5}
/// let decoded: std::collections::HashMap<String, u64> =
///     decode_cbor(&[0xa1, 0x63, b'n', b'u', b'm', 0x05]).unwrap();
/// assert_eq!(decoded["num"], 5);
///
/// assert!(decode_cbor::<u64>(&[0xff]).is_err());
/// ```
pub fn decode_cbor<T: DeserializeOwned>(
    body: &[u8],
) -> Result<T, ciborium::de::Error<std::io::Error>> {
    ciborium::from_reader(body)
}

/// Length of the header block, separator included, or `None` if there is no
/// separator.
fn head_len(request: &[u8]) -> Option<usize> {
    request
        .windows(HEAD_TERMINATOR.len())
        .position(|window| window == HEAD_TERMINATOR)
        .map(|separator_idx| separator_idx + HEAD_TERMINATOR.len())
}

#[cfg(test)]
#[path = "request_body_tests.rs"]
mod tests;
