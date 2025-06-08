use macros::*;
use utils::response::http_response::HttpResponse;
use utils::response::http_response::HttpStatus;

inject_common_imports!();

#[get(path = "/")]
fn index() -> HttpResponse<String> {
    HttpResponse::new("Hello, world!".to_string(), HttpStatus::Ok)
}

#[get(path = "/hello/{name}")]
fn hello(name: String) -> HttpResponse<String> {
    HttpResponse::new(format!("Hello, {}!", name), HttpStatus::Ok)
}

#[unsecure_http_server]
async fn main() {}