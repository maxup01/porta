use utils::response::http_response::{HttpStatus, status_code_from_http_status, HttpResponse};

const CODE_FOR_OK_STATUS: u32 = 200;
const CODE_FOR_CREATED_STATUS: u32 = 201;
const CODE_FOR_ACCEPTED_STATUS: u32 = 202;
const CODE_FOR_BAD_REQUEST_STATUS: u32 = 400;
const CODE_FOR_UNAUTHORIZED_STATUS: u32 = 401;
const CODE_FOR_FORBIDDEN_STATUS: u32 = 403;
const CODE_FOR_NOT_FOUND_STATUS: u32 = 404;
const CODE_FOR_CONFLICT_STATUS: u32 = 409;
const CODE_FOR_TOO_MANY_REQUESTS_STATUS: u32 = 429;
const CODE_FOR_INTERNAL_SERVER_ERROR_STATUS: u32 = 500;
const CODE_FOR_SERVICE_UNAVAILABLE_STATUS: u32 = 503;
const CODE_FOR_GATEAWAY_TIMEOUT_STATUS: u32 = 504;

const RANDOM_VALUE_WITH_TYPE_IMPLEMENTING_SERIALIZABLE: &str = "random-value";
const RANDOM_HTTP_STATUS: HttpStatus = HttpStatus::Ok;

#[test]
pub fn test_of_status_code_from_http_status() {
    
    let code_for_ok = status_code_from_http_status(HttpStatus::Ok);
    let code_for_created = status_code_from_http_status(HttpStatus::Created);
    let code_for_accepted = status_code_from_http_status(HttpStatus::Accepted);
    let code_for_bad_request = status_code_from_http_status(HttpStatus::BadRequest);
    let code_for_unauthorized = status_code_from_http_status(HttpStatus::Unauthorized);
    let code_for_forbidden = status_code_from_http_status(HttpStatus::Forbidden);
    let code_for_not_found = status_code_from_http_status(HttpStatus::NotFound);
    let code_for_conflict = status_code_from_http_status(HttpStatus::Conflict);
    let code_for_too_many_requests = status_code_from_http_status(HttpStatus::TooManyRequests);
    let code_for_internal_server_error = status_code_from_http_status(HttpStatus::InternalServerError);
    let code_for_service_unavailable = status_code_from_http_status(HttpStatus::ServiceUnavailable);
    let code_for_gateaway_timeout = status_code_from_http_status(HttpStatus::GatewayTimeout);

    assert_eq!(code_for_ok, CODE_FOR_OK_STATUS);
    assert_eq!(code_for_created, CODE_FOR_CREATED_STATUS);
    assert_eq!(code_for_accepted, CODE_FOR_ACCEPTED_STATUS);
    assert_eq!(code_for_bad_request, CODE_FOR_BAD_REQUEST_STATUS);
    assert_eq!(code_for_unauthorized, CODE_FOR_UNAUTHORIZED_STATUS);
    assert_eq!(code_for_forbidden, CODE_FOR_FORBIDDEN_STATUS);
    assert_eq!(code_for_not_found, CODE_FOR_NOT_FOUND_STATUS);
    assert_eq!(code_for_conflict, CODE_FOR_CONFLICT_STATUS);
    assert_eq!(code_for_too_many_requests, CODE_FOR_TOO_MANY_REQUESTS_STATUS);
    assert_eq!(code_for_internal_server_error, CODE_FOR_INTERNAL_SERVER_ERROR_STATUS);
    assert_eq!(code_for_service_unavailable, CODE_FOR_SERVICE_UNAVAILABLE_STATUS);
    assert_eq!(code_for_gateaway_timeout, CODE_FOR_GATEAWAY_TIMEOUT_STATUS);
}

#[test]
pub fn test_of_http_response_structure() {

    let random_http_response = HttpResponse::new(RANDOM_VALUE_WITH_TYPE_IMPLEMENTING_SERIALIZABLE, RANDOM_HTTP_STATUS);

    assert_eq!(status_code_from_http_status(random_http_response.status()), status_code_from_http_status(RANDOM_HTTP_STATUS));
    assert_eq!(random_http_response.value(), RANDOM_VALUE_WITH_TYPE_IMPLEMENTING_SERIALIZABLE);
}