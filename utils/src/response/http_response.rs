use chrono::Utc;
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json;

#[repr(u32)]
#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub enum HttpStatus {
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    MovedPermanently = 301,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    Conflict = 409,
    PayloadTooLarge = 413,
    UnprocessableEntity = 422,
    TooManyRequests = 429,
    InternalServerError = 500,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
}

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

pub struct HttpResponse<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    body: T,
    status: HttpStatus,
}

impl<T> HttpResponse<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(body: T, status: HttpStatus) -> HttpResponse<T> {
        HttpResponse { body, status }
    }

    pub fn body(self) -> T {
        self.body
    }

    pub fn status(&self) -> HttpStatus {
        self.status
    }
}

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
