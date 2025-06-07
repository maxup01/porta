use macros::post;
use serde::{Serialize, Deserialize};
use utils::response::http_response::{HttpResponse, HttpStatus};

const CORRECT_REQUEST: &str = "\
POST /test HTTP/1.1\r\n\
Host: localhost:8000\r\n\
Content-Type: application/json\r\n\
Content-Length: 15\r\n\
\r\n\
{\"value\":123}";

const RESPONSE: &str = "\
HTTP/1.1 200 Ok\r\n\
Content-Type: application/json\r\n\
Content-Length: 3\r\n\
Date: Sat, 07 Jun 2025 12:00:00 GMT\r\n\
\r\n\
123";



#[derive(Serialize, Deserialize)]
struct MyStruct {
    pub value: i32
}

impl MyStruct {
    pub fn get_value(&self) -> i32 {
        self.value
    }
}

#[post(path = "/test")]
fn example_handler(value: MyStruct) -> HttpResponse<i32> {
    HttpResponse::new(value.get_value(), HttpStatus::Ok)
}

#[test]
fn test_post_macro() {

    let response = example_handler(CORRECT_REQUEST);

    let mut response_lines = response.split("\r\n");
    let mut correct_response_lines = RESPONSE.split("\r\n");

    assert_eq!(response_lines.next().unwrap(), correct_response_lines.next().unwrap());
    assert_eq!(response_lines.next().unwrap(), correct_response_lines.next().unwrap());
    assert_eq!(response_lines.next().unwrap(), correct_response_lines.next().unwrap());

    assert!(response_lines.next().unwrap().starts_with("Date: "));
    correct_response_lines.next().unwrap();

    assert_eq!(response_lines.next().unwrap(), correct_response_lines.next().unwrap());
}