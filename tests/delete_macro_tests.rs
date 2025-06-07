use macros::delete;
use serde::{Serialize, Deserialize};
use utils::response::http_response::{HttpResponse, HttpStatus};

const DELETE_REQUEST: &str = "DELETE /resource?id=42";
const EXPECTED_RESPONSE_PREFIX: &str = "HTTP/1.1 200 Ok\r\n";

#[derive(Serialize, Deserialize)]
struct DeleteResult {
    success: bool,
    id: u32,
}

#[delete(path = "/resource")]
fn delete_resource(id: u32) -> HttpResponse<DeleteResult> {
    HttpResponse::new(DeleteResult { success: true, id }, HttpStatus::Ok)
}

#[test]
fn test_delete_macro() {
    let response = delete_resource(DELETE_REQUEST);

    assert!(response.starts_with(EXPECTED_RESPONSE_PREFIX));

    let body_start = response.find("\r\n\r\n").unwrap() + 4;
    let body = &response[body_start..];

    println!("Response body: {}", body);

    let result: DeleteResult = serde_json::from_str(body).expect("Failed to parse response JSON");

    assert!(result.success);
    assert_eq!(result.id, 42);
}