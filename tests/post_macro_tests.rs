use macros::{inject_common_imports, post};
use serde::{Serialize, Deserialize};
use utils::response::http_response::{HttpResponse, HttpStatus};

inject_common_imports!();

const CORRECT_REQUEST: &str = "\
POST /test HTTP/1.1\r\n\
Host: localhost:8000\r\n\
Content-Type: application/json\r\n\
Content-Length: 15\r\n\
\r\n\
{\"value\":123}";

const CORRECT_REQUEST_FOR_SECOND_MAPPING: &str = "\
POST /user/42 HTTP/1.1\r\n\
Host: localhost:8000\r\n\
Content-Type: application/json\r\n\
Content-Length: 14\r\n\
\r\n\
{\"value\":99}";

const RESPONSE: &str = "\
HTTP/1.1 200 Ok\r\n\
Content-Type: application/json\r\n\
Content-Length: 3\r\n\
Date: Sat, 07 Jun 2025 12:00:00 GMT\r\n\
\r\n\
123";

const RESPONSE_FOR_SECOND_MAPPING: &str = "\
HTTP/1.1 200 Ok\r\n\
Content-Type: application/json\r\n\
Content-Length: 16\r\n\
Date: Sat, 07 Jun 2025 12:00:00 GMT\r\n\
\r\n\
\"id:42 value:99\"";



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

#[post(path = "/user/{id}")]
fn post_with_path_param_and_body(value: MyStruct, id: u32) -> HttpResponse<String> {
    HttpResponse::new(format!("id:{} value:{}", id, value.get_value()), HttpStatus::Ok)
}

#[test]
pub fn test_post_macro() {

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

#[test]
pub fn test_post_macro_with_path_param_and_body() {

    let response = post_with_path_param_and_body(CORRECT_REQUEST_FOR_SECOND_MAPPING);

    let mut response_lines = response.split("\r\n");
    let mut correct_response_lines = RESPONSE_FOR_SECOND_MAPPING.split("\r\n");

    assert_eq!(response_lines.next().unwrap(), correct_response_lines.next().unwrap());
    assert_eq!(response_lines.next().unwrap(), correct_response_lines.next().unwrap());
    assert_eq!(response_lines.next().unwrap(), correct_response_lines.next().unwrap());

    assert!(response_lines.next().unwrap().starts_with("Date: "));
    correct_response_lines.next().unwrap();

    assert_eq!(response_lines.next().unwrap(), correct_response_lines.next().unwrap());
}