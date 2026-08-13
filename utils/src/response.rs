use chrono::Utc;
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json;

/// Strongly-typed HTTP status codes.
///
/// Each variant is explicitly mapped to its numeric value via `#[repr(u32)]`,
/// so casting with `status as u32` yields the correct code (e.g. `HttpStatus::Ok as u32 == 200`).
///
/// # Examples
///
/// ```rust
/// use utils::response::HttpStatus;
///
/// let code = HttpStatus::NotFound as u32;
/// assert_eq!(code, 404);
///
/// let label = HttpStatus::NotFound.to_string();
/// assert_eq!(label, "Not Found");
/// ```
#[repr(u32)]
#[derive(Hash, Debug, Eq, PartialEq, Copy, Clone)]
pub enum HttpStatus {
    /// 200 — The request succeeded.
    Ok = 200,

    /// 201 — The request succeeded and a new resource was created.
    Created = 201,

    /// 202 — The request was accepted for processing, but processing is not yet complete.
    Accepted = 202,

    /// 204 — The request succeeded but there is no content to return.
    NoContent = 204,

    /// 301 — The requested resource has been permanently moved to a new URL.
    MovedPermanently = 301,

    /// 400 — The server could not understand the request due to invalid syntax.
    BadRequest = 400,

    /// 401 — The client must authenticate itself to get the requested response.
    Unauthorized = 401,

    /// 403 — The client does not have access rights to the content.
    Forbidden = 403,

    /// 404 — The server cannot find the requested resource.
    NotFound = 404,

    /// 405 — The HTTP method is not allowed for the targeted resource.
    MethodNotAllowed = 405,

    /// 408 — The client did not produce a complete request within the time the
    /// server was prepared to wait.
    RequestTimeout = 408,

    /// 409 — The request conflicts with the current state of the server.
    Conflict = 409,

    /// 413 — The request body exceeds the limit the server is willing to process.
    PayloadTooLarge = 413,

    /// 422 — The request was well-formed but could not be followed due to semantic errors.
    UnprocessableEntity = 422,

    /// 429 — The client has sent too many requests in a given amount of time.
    TooManyRequests = 429,

    /// 500 — The server encountered an unexpected condition it could not recover from.
    InternalServerError = 500,

    /// 503 — The server is not ready to handle the request, often due to maintenance or overload.
    ServiceUnavailable = 503,

    /// 504 — The server, acting as a gateway, did not receive a timely response from an upstream server.
    GatewayTimeout = 504,
}

/// Formats an [`HttpStatus`] as its standard reason phrase (e.g. `"Not Found"`).
///
/// This implementation is used by [`format_response`] to populate the reason
/// phrase field of the HTTP status line.
impl fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let status_str = match self {
            HttpStatus::Ok => "OK",
            HttpStatus::Created => "Created",
            HttpStatus::Accepted => "Accepted",
            HttpStatus::NoContent => "No Content",
            HttpStatus::MovedPermanently => "Moved Permanently",
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::Forbidden => "Forbidden",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::MethodNotAllowed => "Method Not Allowed",
            HttpStatus::RequestTimeout => "Request Timeout",
            HttpStatus::Conflict => "Conflict",
            HttpStatus::PayloadTooLarge => "Payload Too Large",
            HttpStatus::UnprocessableEntity => "Unprocessable Entity",
            HttpStatus::TooManyRequests => "Too Many Requests",
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::ServiceUnavailable => "Service Unavailable",
            HttpStatus::GatewayTimeout => "Gateway Timeout",
        };

        write!(f, "{}", status_str)
    }
}

/// A generic HTTP response pairing a typed body with an [`HttpStatus`].
///
/// `T` must implement [`Serialize`] and [`Deserialize`] so that the body can be
/// converted to and from JSON when the response is formatted or transmitted.
///
/// # Examples
///
/// ```rust
/// use utils::response::{HttpResponse, HttpStatus};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Payload { message: String }
///
/// let resp = HttpResponse::new(
///     Payload { message: "created".to_string() },
///     HttpStatus::Created,
/// );
///
/// assert_eq!(resp.status(), HttpStatus::Created);
/// ```
pub struct HttpResponse<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// The response body, serialized to JSON when the response is formatted.
    body: T,

    /// The HTTP status code associated with this response.
    status: HttpStatus,
}

impl<T> HttpResponse<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Creates a new [`HttpResponse`] with the given body and status code.
    ///
    /// # Parameters
    ///
    /// - `body`   — Any value that implements `Serialize + Deserialize`. It will
    ///              be JSON-serialized when the response is passed to [`format_response`].
    /// - `status` — The [`HttpStatus`] to associate with this response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use utils::response::{HttpResponse, HttpStatus};
    ///
    /// let resp = HttpResponse::new("hello".to_string(), HttpStatus::Ok);
    /// ```
    pub fn new(body: T, status: HttpStatus) -> HttpResponse<T> {
        HttpResponse { body, status }
    }

    /// Consumes the response and returns the body.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use utils::response::{HttpResponse, HttpStatus};
    ///
    /// let resp = HttpResponse::new(42u32, HttpStatus::Ok);
    /// assert_eq!(resp.body(), 42);
    /// ```
    pub fn body(self) -> T {
        self.body
    }

    /// Returns the [`HttpStatus`] of this response without consuming it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use utils::response::{HttpResponse, HttpStatus};
    ///
    /// let resp = HttpResponse::new("data".to_string(), HttpStatus::Accepted);
    /// assert_eq!(resp.status(), HttpStatus::Accepted);
    /// ```
    pub fn status(&self) -> HttpStatus {
        self.status
    }
}

/// Serializes an [`HttpResponse`] into a raw HTTP/1.1 message string.
///
/// The output conforms to the HTTP/1.1 wire format:
///
/// ```text
/// HTTP/1.1 <code> <reason>\r\n
/// Content-Type: application/json\r\n
/// Content-Length: <byte_length>\r\n
/// Date: <rfc-date>\r\n
/// Connection: close\r\n
/// \r\n
/// <json_body>
/// ```
///
/// The `Date` header is set to the current UTC time formatted as
/// `"Tue, 15 Apr 2025 10:00:00 GMT"`.
///
/// A `204 No Content` response is the exception: the serialized body is
/// discarded and the content headers are omitted entirely. See
/// [`format_http_message`].
///
/// # Panics
///
/// Panics if `T` cannot be serialized to JSON (via [`serde_json::to_string`]).
/// In practice this should only occur for types that explicitly implement
/// [`Serialize`] in a way that always errors.
///
/// # Examples
///
/// ```rust
/// use utils::response::{HttpResponse, HttpStatus, format_response};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Body { ok: bool }
///
/// let resp = HttpResponse::new(Body { ok: true }, HttpStatus::Ok);
/// let raw = format_response(resp);
///
/// assert!(raw.starts_with("HTTP/1.1 200 OK\r\n"));
/// assert!(raw.contains("Content-Type: application/json"));
/// assert!(raw.ends_with("{\"ok\":true}"));
/// ```
pub fn format_response<T>(response: HttpResponse<T>) -> String
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let status = response.status();
    let value = response.body();
    let serialized_value =
        serde_json::to_string(&value).expect("Failed to serialize response to JSON");

    format_http_message(status, "application/json", &serialized_value)
}

/// Builds a complete HTTP/1.1 message from a status, a content type and an
/// already-encoded body.
///
/// Every response the server emits — successful handler results and error
/// replies alike — is assembled here, so header set, header order and the
/// `Content-Length` calculation exist in exactly one place. Adding a header to
/// this function adds it to every response.
///
/// `Content-Length` is `body.len()`, which is a byte count rather than a
/// character count, as HTTP requires.
///
/// `Connection: close` is sent on every response because the server handles
/// exactly one request per connection. Without it an HTTP/1.1 client is entitled
/// to assume the connection persists and will send its next request into a socket
/// that has already been closed.
///
/// # 204 No Content
///
/// RFC 9110 §15.3.5 forbids content on a `204`, and a `Content-Length` describing
/// content that is not there is a framing error. Such a response is therefore
/// emitted with no body and neither content header, whatever `body` was passed.
fn format_http_message(status: HttpStatus, content_type: &str, body: &str) -> String {
    let now = Utc::now();
    let date = now.format("%a, %d %b %Y %H:%M:%S GMT");

    if status == HttpStatus::NoContent {
        return format!(
            "HTTP/1.1 {} {}\r\nDate: {}\r\nConnection: close\r\n\r\n",
            status as u32, status, date
        );
    }

    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nDate: {}\r\nConnection: close\r\n\r\n{}",
        status as u32,
        status,
        content_type,
        body.len(),
        date,
        body
    )
}

/// Builds a `text/plain` HTTP response carrying nothing but a status.
///
/// This is the counterpart to [`format_response`] for replies the server
/// generates itself — a request it could not parse, a route it could not
/// resolve, a body larger than it will accept — where there is no handler
/// return value to serialize.
///
/// The body is the status's reason phrase, so [`HttpStatus`] is the single
/// source of the status code, the reason phrase in the status line, and the
/// body text. None of the three can drift from the others.
///
/// # Examples
///
/// ```rust
/// use utils::response::{HttpStatus, status_response};
///
/// let raw = status_response(HttpStatus::NotFound);
///
/// assert!(raw.starts_with("HTTP/1.1 404 Not Found\r\n"));
/// assert!(raw.contains("Content-Type: text/plain\r\n"));
/// assert!(raw.contains("Content-Length: 9\r\n"));
/// assert!(raw.ends_with("\r\n\r\nNot Found"));
/// ```
pub fn status_response(status: HttpStatus) -> String {
    format_http_message(status, "text/plain", &status.to_string())
}

/// Returns `response` with `headers` inserted directly after the status line.
///
/// A handler returns a finished HTTP message as a `String`, so headers the
/// server decides on afterwards — `Access-Control-Allow-Origin`, `Allow` — have
/// no other seam to enter through. Inserting after the status line rather than
/// before the blank line keeps the operation independent of whether the message
/// has a body, and is valid either way: header order carries no meaning in
/// HTTP/1.1 (RFC 9110 §5.3).
///
/// Nothing is de-duplicated. Headers added here are ones only the server emits,
/// so a collision with a header already in `response` would be a bug in the
/// caller rather than input to defend against.
///
/// A `response` with no CRLF at all is returned unchanged: there is no status
/// line to insert after, and corrupting an already-malformed message helps
/// nobody.
///
/// # Examples
///
/// ```rust
/// use utils::response::{HttpStatus, status_response, with_headers};
///
/// let response = status_response(HttpStatus::NoContent);
/// let with_cors = with_headers(
///     &response,
///     &[("Access-Control-Allow-Origin", "*".to_string())],
/// );
///
/// assert!(with_cors.starts_with("HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\n"));
/// assert!(with_cors.ends_with("\r\n\r\n"));
/// ```
pub fn with_headers(response: &str, headers: &[(&str, String)]) -> String {
    if headers.is_empty() {
        return response.to_string();
    }

    let Some(status_line_end) = response.find("\r\n") else {
        return response.to_string();
    };

    let insertion_point = status_line_end + 2;

    let mut with_headers = String::with_capacity(response.len() + headers.len() * 64);

    with_headers.push_str(&response[..insertion_point]);

    for (name, value) in headers {
        with_headers.push_str(name);
        with_headers.push_str(": ");
        with_headers.push_str(value);
        with_headers.push_str("\r\n");
    }

    with_headers.push_str(&response[insertion_point..]);

    with_headers
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
