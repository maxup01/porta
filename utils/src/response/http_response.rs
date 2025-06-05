use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use serde::Serialize;

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

pub struct HttpResponse<T: Serialize> {
    value: T,
    status: HttpStatus
}

impl<T: Serialize> HttpResponse<T> {
    
    pub fn new(value: T, status: HttpStatus) -> HttpResponse<T> {
        HttpResponse {
            value,
            status
        }
    }

    pub fn value(self) -> T {
        self.value
    }

    pub fn status(&self) -> HttpStatus {
        self.status.clone()
    }
}