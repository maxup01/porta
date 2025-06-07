use macros::get;
use serde::{Serialize, Deserialize};
use utils::response::http_response::{HttpResponse, HttpStatus};

const RANDOM_REQUEST: &str = "GET /hello?parameter=random";
const CORRECT_RESPONSE: &str = "HTTP/1.1 200 Ok\r\n\
Content-Type: application/json\r\n\
Content-Length: 18\r\n\
Date: Sat, 07 Jun 2025 00:00:00 GMT\r\n\
\r\n\
{\"value\":\"random\"}";

#[derive(Serialize, Deserialize)]
struct RandomStruct {
    value: String
}

#[get(path = "/hello")]
fn random_fn(parameter: String) -> HttpResponse<RandomStruct> {
    HttpResponse::new(RandomStruct {value: parameter}, HttpStatus::Ok)
}

#[test]
pub fn test_get_macro() {
    let response = random_fn(RANDOM_REQUEST);

    let mut lines_in_correct_response = CORRECT_RESPONSE.split("\r\n");
    let mut lines_in_actual_response = response.split("\r\n");

    assert_eq!(lines_in_actual_response.next().unwrap(), lines_in_correct_response.next().unwrap());
    assert_eq!(lines_in_actual_response.next().unwrap(), lines_in_correct_response.next().unwrap());
    assert_eq!(lines_in_actual_response.next().unwrap(), lines_in_correct_response.next().unwrap());

    let date_line = lines_in_actual_response.next();
    lines_in_correct_response.next();

    assert!(date_line.map_or(false, |line| line.starts_with("Date: ")));
    assert_eq!(lines_in_actual_response.next().unwrap(), lines_in_correct_response.next().unwrap());
    assert_eq!(lines_in_actual_response.next().unwrap(), lines_in_correct_response.next().unwrap());
}