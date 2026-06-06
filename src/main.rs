use embedded_web_server::response::{HttpResponse, HttpStatus};
use embedded_web_server::*;
use serde::{Deserialize, Serialize};

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
    pub name: String,
}

#[post(path = "/something/{id}")]
fn something(id: u64, body: RandomStruct) -> HttpResponse<String> {
    HttpResponse::new(
        format!("Received id: {} num: {} name: {}", id, body.num, body.name),
        HttpStatus::Ok,
    )
}

#[http_server(ip = "127.0.0.1", port = 8443)]
async fn main() {}
