use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use serde::{Deserialize, Serialize};
use serde_json; 
use chrono::Utc;

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub enum HttpStatus {
    Ok,
    Created,
    Accepted,
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    TooManyRequests,
    InternalServerError,
    ServiceUnavailable,
    GatewayTimeout
}

fn http_status_to_string(status: &HttpStatus) ->  String {
    match status {
        HttpStatus::Ok => "Ok".to_string(),
        HttpStatus::Created => "Created".to_string(),
        HttpStatus::Accepted => "Accepted".to_string(),
        HttpStatus::BadRequest => "Bad Request".to_string(),
        HttpStatus::Unauthorized => "Unauthorized".to_string(),
        HttpStatus::Forbidden => "Forbidden".to_string(),
        HttpStatus::NotFound => "Not Found".to_string(),
        HttpStatus::Conflict => "Conflict".to_string(),
        HttpStatus::TooManyRequests => "Too Many Requests".to_string(),
        HttpStatus::InternalServerError => "Internal Server Error".to_string(),
        HttpStatus::ServiceUnavailable => "Service Unavailable".to_string(),
        HttpStatus::GatewayTimeout => "Gateway Timeout".to_string(),
    }
}

pub static HTTP_STATUSES: LazyLock<Mutex<HashMap<HttpStatus, u32>>> = LazyLock::new(|| {
    let mut m: HashMap<HttpStatus, u32> = HashMap::new();
    m.insert(HttpStatus::Ok, 200);
    m.insert(HttpStatus::Created, 201);
    m.insert(HttpStatus::Accepted, 202);
    m.insert(HttpStatus::BadRequest, 400);
    m.insert(HttpStatus::Unauthorized, 401);
    m.insert(HttpStatus::Forbidden, 403);
    m.insert(HttpStatus::NotFound, 404);
    m.insert(HttpStatus::Conflict, 409);
    m.insert(HttpStatus::TooManyRequests, 429);
    m.insert(HttpStatus::InternalServerError, 500);
    m.insert(HttpStatus::ServiceUnavailable, 503);
    m.insert(HttpStatus::GatewayTimeout, 504);
    Mutex::new(m)
});

pub fn status_code_from_http_status(http_status: HttpStatus) -> u32 {
    let map_with_statuses = HTTP_STATUSES.lock().unwrap();
    map_with_statuses.get(&http_status).unwrap().clone()
}

pub struct HttpResponse<T>
where 
    T: Serialize + for<'de> Deserialize<'de>
{
    body: T,
    status: HttpStatus
}

impl<T> HttpResponse<T>
where 
    T: Serialize + for<'de> Deserialize<'de>
{
    pub fn new(body: T, status: HttpStatus) -> HttpResponse<T> {
        HttpResponse {
            body,
            status
        }
    }

    pub fn body(self) -> T {
        self.body
    }

    pub fn status(&self) -> HttpStatus {
        self.status.clone()
    }
}

pub fn format_response<T>(response :HttpResponse<T>) -> String
where 
    T: Serialize + for<'de> Deserialize<'de>
{

    let status = response.status();
    let value = response.body();
    let serialized_value = serde_json::to_string(&value)
            .expect("Failed to serialize response to JSON");
    let now = Utc::now();

    format!("HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nDate: {}\r\n\r\n{}",
        status_code_from_http_status(status), http_status_to_string(&status), serialized_value.as_bytes().len(),
         now.format("%a, %d %b %Y %H:%M:%S GMT"), serialized_value)
}