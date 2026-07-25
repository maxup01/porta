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
/// \r\n
/// <json_body>
/// ```
///
/// The `Date` header is set to the current UTC time formatted as
/// `"Tue, 15 Apr 2025 10:00:00 GMT"`.
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
    let now = Utc::now();

    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nDate: {}\r\n\r\n{}",
        status as u32,
        status,
        serialized_value.len(),
        now.format("%a, %d %b %Y %H:%M:%S GMT"),
        serialized_value
    )
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
