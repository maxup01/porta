use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Hash, Eq, PartialEq)]
pub enum HttpStatus {
    OK,
    CREATED,
    ACCEPTED,
    BAD_REQUEST,
    UNAUTHORIZED,
    FORBIDDEN,
    NOT_FOUND,
    CONFLICT,
    TOO_MANY_REQUESTS,
    INTERNAL_SERVER_ERROR,
    SERVICE_UNAVAILABLE,
    GATEWAY_TIMEOUT
}

pub static HTTP_STATUSES: LazyLock<Mutex<HashMap<HttpStatus, u32>>> = LazyLock::new(|| {
    let mut m: HashMap<HttpStatus, u32> = HashMap::new();
    m.insert(HttpStatus::OK, 200);
    m.insert(HttpStatus::CREATED, 201);
    m.insert(HttpStatus::ACCEPTED, 202);
    m.insert(HttpStatus::BAD_REQUEST, 400);
    m.insert(HttpStatus::UNAUTHORIZED, 401);
    m.insert(HttpStatus::FORBIDDEN, 403);
    m.insert(HttpStatus::NOT_FOUND, 404);
    m.insert(HttpStatus::CONFLICT, 409);
    m.insert(HttpStatus::TOO_MANY_REQUESTS, 429);
    m.insert(HttpStatus::INTERNAL_SERVER_ERROR, 500);
    m.insert(HttpStatus::SERVICE_UNAVAILABLE, 503);
    m.insert(HttpStatus::GATEWAY_TIMEOUT, 504);
    Mutex::new(m)
});

pub fn status_code_from_http_status(http_status: HttpStatus) -> u32 {
    let map_with_statuses = HTTP_STATUSES.lock().unwrap();
    map_with_statuses.get(&http_status).unwrap().clone()
}