use macros::{delete, get, patch, post};
use serde::{Deserialize, Serialize};
use utils::response::{HttpResponse, HttpStatus};

const EXPECTED_OK_PREFIX: &str = "HTTP/1.1 200 Ok\r\n";
const EXPECTED_404_PREFIX: &str = "HTTP/1.1 404 Not Found\r\n";

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
