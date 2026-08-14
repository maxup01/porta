use macros::{delete, get, patch, post, put};
use serde::{Deserialize, Serialize};
use utils::response::{HttpResponse, HttpStatus};

const EXPECTED_OK_PREFIX: &str = "HTTP/1.1 200 OK\r\n";
const EXPECTED_404_PREFIX: &str = "HTTP/1.1 404 Not Found\r\n";
const EXPECTED_400_PREFIX: &str = "HTTP/1.1 400 Bad Request\r\n";

fn body_of(response: &str) -> &str {
    &response[response.find("\r\n\r\n").expect("no header/body separator") + 4..]
}

#[derive(Serialize, Deserialize)]
struct Item {
    id: u32,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct Created {
    id: u32,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct Updated {
    id: u32,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct Deleted {
    id: u32,
}

// ── GET ──────────────────────────────────────────────────────────────────────

#[get(path = "/items/{id}")]
fn get_item(id: u32) -> HttpResponse<Item> {
    HttpResponse::new(
        Item {
            id,
            name: "test".to_string(),
        },
        HttpStatus::Ok,
    )
}

#[get(path = "/items")]
fn get_item_by_query(id: u32) -> HttpResponse<Item> {
    HttpResponse::new(
        Item {
            id,
            name: "test".to_string(),
        },
        HttpStatus::Ok,
    )
}

#[test]
fn test_get_with_path_param() {
    let response = get_item("GET /items/1");
    eprintln!("{}", response);
    assert!(response.starts_with(EXPECTED_OK_PREFIX));
    let body = &response[response.find("\r\n\r\n").unwrap() + 4..];
    let item: Item = serde_json::from_str(body).unwrap();
    assert_eq!(item.id, 1);
    assert_eq!(item.name, "test");
}

#[test]
fn test_get_with_query_param() {
    let response = get_item_by_query("GET /items?id=5");
    assert!(response.starts_with(EXPECTED_OK_PREFIX));
    let body = &response[response.find("\r\n\r\n").unwrap() + 4..];
    let item: Item = serde_json::from_str(body).unwrap();
    assert_eq!(item.id, 5);
}

#[test]
fn test_get_with_invalid_path_param_returns_404() {
    let response = get_item("GET /items/abc");
    eprintln!("{}", response);
    assert!(response.starts_with(EXPECTED_404_PREFIX));
}

#[test]
fn test_get_with_invalid_query_param_returns_404() {
    let response = get_item_by_query("GET /items?id=abc");
    eprintln!("{}", response);
    assert!(response.starts_with(EXPECTED_404_PREFIX));
}

// ── POST ─────────────────────────────────────────────────────────────────────

#[post(path = "/items")]
fn create_item(name: String) -> HttpResponse<Created> {
    HttpResponse::new(Created { id: 1, name }, HttpStatus::Ok)
}

#[test]
fn test_post_extracts_body() {
    let response =
        create_item("POST /items HTTP/1.1\r\nContent-Type: application/json\r\n\r\nwidget");
    assert!(response.starts_with(EXPECTED_OK_PREFIX));
    let body = &response[response.find("\r\n\r\n").unwrap() + 4..];
    let created: Created = serde_json::from_str(body).unwrap();
    assert_eq!(created.name, "widget");
    assert_eq!(created.id, 1);
}

// ── PATCH ────────────────────────────────────────────────────────────────────

#[patch(path = "/items/{id}")]
fn update_item(id: u32, name: String) -> HttpResponse<Updated> {
    HttpResponse::new(Updated { id, name }, HttpStatus::Ok)
}

#[test]
fn test_patch_with_path_param_and_body() {
    let response =
        update_item("PATCH /items/7 HTTP/1.1\r\nContent-Type: application/json\r\n\r\nnewname");
    assert!(response.starts_with(EXPECTED_OK_PREFIX));
    let body = &response[response.find("\r\n\r\n").unwrap() + 4..];
    let updated: Updated = serde_json::from_str(body).unwrap();
    assert_eq!(updated.id, 7);
    assert_eq!(updated.name, "newname");
}

#[test]
fn test_patch_with_invalid_id_returns_404() {
    let response =
        update_item("PATCH /items/abc HTTP/1.1\r\nContent-Type: application/json\r\n\r\nnewname");
    assert!(response.starts_with(EXPECTED_404_PREFIX));
}

// ── PUT ──────────────────────────────────────────────────────────────────────

#[put(path = "/items/{id}")]
fn replace_item(id: u32, name: String) -> HttpResponse<Updated> {
    HttpResponse::new(Updated { id, name }, HttpStatus::Ok)
}

#[test]
fn test_put_with_path_param_and_body() {
    let response =
        replace_item("PUT /items/3 HTTP/1.1\r\nContent-Type: application/json\r\n\r\nreplaced");
    assert!(response.starts_with(EXPECTED_OK_PREFIX));
    let body = &response[response.find("\r\n\r\n").unwrap() + 4..];
    let updated: Updated = serde_json::from_str(body).unwrap();
    assert_eq!(updated.id, 3);
    assert_eq!(updated.name, "replaced");
}

#[test]
fn test_put_with_invalid_id_returns_404() {
    let response =
        replace_item("PUT /items/abc HTTP/1.1\r\nContent-Type: application/json\r\n\r\nreplaced");
    assert!(response.starts_with(EXPECTED_404_PREFIX));
}

#[test]
fn test_put_without_body_returns_400() {
    let response = replace_item("PUT /items/3 HTTP/1.1\r\nContent-Type: application/json\r\n\r\n");
    assert!(response.starts_with(EXPECTED_400_PREFIX));
}

// ── DELETE ───────────────────────────────────────────────────────────────────

#[delete(path = "/items/{id}")]
fn delete_item(id: u32) -> HttpResponse<Deleted> {
    HttpResponse::new(Deleted { id }, HttpStatus::Ok)
}

#[delete(path = "/items")]
fn delete_item_by_query(id: u32) -> HttpResponse<Deleted> {
    HttpResponse::new(Deleted { id }, HttpStatus::Ok)
}

#[test]
fn test_delete_with_path_param() {
    let response = delete_item("DELETE /items/3");
    eprintln!("{}", response);
    assert!(response.starts_with(EXPECTED_OK_PREFIX));
    let body = &response[response.find("\r\n\r\n").unwrap() + 4..];
    let deleted: Deleted = serde_json::from_str(body).unwrap();
    assert_eq!(deleted.id, 3);
}

#[test]
fn test_delete_with_query_param() {
    let response = delete_item_by_query("DELETE /items?id=9");
    assert!(response.starts_with(EXPECTED_OK_PREFIX));
    let body = &response[response.find("\r\n\r\n").unwrap() + 4..];
    let deleted: Deleted = serde_json::from_str(body).unwrap();
    assert_eq!(deleted.id, 9);
}

#[test]
fn test_delete_with_invalid_path_param_returns_404() {
    let response = delete_item("DELETE /items/abc");
    eprintln!("{}", response);
    assert!(response.starts_with(EXPECTED_404_PREFIX));
}

#[test]
fn test_delete_with_invalid_query_param_returns_404() {
    let response = delete_item_by_query("DELETE /items?id=abc");
    assert!(response.starts_with(EXPECTED_404_PREFIX));
}

// ── Generated handler error paths ────────────────────────────────────────────
//
// Everything below drives a macro-generated handler directly, which is the only
// part of the expansion reachable from this workspace. The accept loop, the read
// loop, the 413 ceiling and the request deadline live inside `http_server`'s
// `quote!` block and have no callable form here — see the note at the bottom.

#[get(path = "/errors/{id}")]
fn error_item(id: u32) -> HttpResponse<Item> {
    HttpResponse::new(
        Item {
            id,
            name: "test".to_string(),
        },
        HttpStatus::Ok,
    )
}

#[get(path = "/errors-query")]
fn error_by_query(id: u32) -> HttpResponse<Item> {
    HttpResponse::new(
        Item {
            id,
            name: "test".to_string(),
        },
        HttpStatus::Ok,
    )
}

#[post(path = "/errors-body")]
fn error_body(name: String) -> HttpResponse<Created> {
    HttpResponse::new(Created { id: 1, name }, HttpStatus::Ok)
}

#[test]
fn malformed_request_line_returns_400() {
    // No second whitespace-delimited token, so there is no request target to read.
    let response = error_item("GARBAGE");

    assert!(
        response.starts_with(EXPECTED_400_PREFIX),
        "unexpected response: {response}"
    );
}

#[test]
fn missing_parameter_returns_400_not_404() {
    // `id` is neither a path segment nor a query parameter here. A value that is
    // present but unparseable is a 404; one that is absent is a 400.
    let response = error_by_query("GET /errors-query");

    assert!(
        response.starts_with(EXPECTED_400_PREFIX),
        "unexpected response: {response}"
    );
}

#[test]
fn post_with_empty_body_returns_400() {
    let response =
        error_body("POST /errors-body HTTP/1.1\r\nContent-Type: application/json\r\n\r\n");

    assert!(
        response.starts_with(EXPECTED_400_PREFIX),
        "unexpected response: {response}"
    );
}

// ── Query string handling in generated handlers ──────────────────────────────

#[test]
fn query_string_is_stripped_before_path_params_are_matched() {
    // The handler splits the target itself: `/errors/7` must match `/errors/{id}`
    // even though the request carries a query string the route pattern cannot.
    let response = error_item("GET /errors/7?unrelated=ignored");

    assert!(
        response.starts_with(EXPECTED_OK_PREFIX),
        "unexpected response: {response}"
    );

    let item: Item = serde_json::from_str(body_of(&response)).unwrap();
    assert_eq!(item.id, 7);
}

#[test]
fn query_parameter_overrides_path_parameter_of_the_same_name() {
    // Path params are collected first, then query params are merged over them.
    // Pinning this because it is a silent precedence rule, not an obvious one.
    let response = error_item("GET /errors/7?id=9");

    assert!(response.starts_with(EXPECTED_OK_PREFIX));

    let item: Item = serde_json::from_str(body_of(&response)).unwrap();
    assert_eq!(item.id, 9, "the query string should win over the path");
}

// ── Response framing ─────────────────────────────────────────────────────────

#[test]
fn successful_response_carries_framing_headers() {
    let response = get_item("GET /items/1");

    assert!(response.contains("Content-Type: application/json\r\n"));
    assert!(response.contains("Date: "));

    // A handler's own output says nothing about the connection. Whether this is
    // the last response on it is decided by `server`, after the handler returns.
    assert!(!response.contains("Connection:"), "{response}");
}

#[test]
fn error_response_carries_framing_headers() {
    let response = get_item("GET /items/abc");

    assert!(response.contains("Content-Type: text/plain\r\n"));
    assert!(response.contains("Date: "));
    assert!(!response.contains("Connection:"), "{response}");
}

#[test]
fn content_length_matches_the_body_it_describes() {
    for response in [get_item("GET /items/1"), get_item("GET /items/abc")] {
        let body = body_of(&response);
        let expected = format!("Content-Length: {}\r\n", body.len());

        assert!(
            response.contains(&expected),
            "Content-Length disagrees with the body: {response}"
        );
    }
}
