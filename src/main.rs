use macros::*;
use utils::response::http_response::HttpResponse;
use utils::response::http_response::HttpStatus;
use serde::{Deserialize, Serialize};

inject_common_imports!();

#[get(path = "/")]
fn index() -> HttpResponse<String> {
    HttpResponse::new("Hello, world!".to_string(), HttpStatus::Ok)
}

#[get(path = "/hello/{name}")]
fn hello(name: String) -> HttpResponse<String> {
    HttpResponse::new(format!("Hello, {}!", name), HttpStatus::Ok)
}

#[derive(Serialize, Deserialize)]
struct RandomStruct {
    pub num: u64,
    pub name: String
}

#[post(path = "/something/{id}")]
fn something(id: u64, body :RandomStruct) -> HttpResponse<String> {
    HttpResponse::new(
        format!("Received id: {} num: {} name: {}", id, body.num, body.name),
        HttpStatus::Ok,
    )
}

#[unsecure_http_server]
async fn main() {}