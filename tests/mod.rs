use macros::{component, delete, get, patch, post, put};
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

// ── Components ───────────────────────────────────────────────────────────────
//
// `#[component]` hangs its accessors off `AppContext`, which `#[http_server]`
// generates into the caller's crate root. An integration test cannot run that
// attribute — it takes over `main` and binds a TLS listener — so the declaration it
// would emit is written out here instead. Everything below then exercises the real
// expansion: two components, an accessor apiece, and handlers that receive them.

#[derive(Default)]
pub struct AppContext {}

/// Mutable shared state. The lock is on the field rather than around the whole
/// component, which is what lets the accessor hand out a plain `&'static`.
#[component(name = "counter")]
#[derive(Default)]
struct Counter {
    hits: std::sync::Mutex<u32>,
}

impl Counter {
    fn record(&self) -> u32 {
        let mut hits = self.hits.lock().expect("counter lock poisoned");
        *hits += 1;
        *hits
    }
}

/// Immutable shared state, and a hand-written `Default` — the accessor builds the
/// instance through `Default::default`, so this is what a component that needs real
/// construction looks like today.
#[component(name = "greeter")]
struct Greeter {
    prefix: String,
}

impl Default for Greeter {
    fn default() -> Self {
        Greeter {
            prefix: "hello".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Hits {
    total: u32,
}

#[derive(Serialize, Deserialize)]
struct Greeting {
    message: String,
}

#[get(path = "/hits")]
fn record_hit(#[component] counter: &Counter) -> HttpResponse<Hits> {
    HttpResponse::new(
        Hits {
            total: counter.record(),
        },
        HttpStatus::Ok,
    )
}

#[get(path = "/greet/{name}")]
fn greet(name: String, #[component] greeter: &Greeter) -> HttpResponse<Greeting> {
    HttpResponse::new(
        Greeting {
            message: format!("{} {}", greeter.prefix, name),
        },
        HttpStatus::Ok,
    )
}

// The component is declared *ahead* of the body parameter on purpose. The body binds
// to the first parameter that is not a path param, so a component left in that list
// would take the body's place and the real parameter would answer 400.
#[post(path = "/greetings")]
fn create_greeting(#[component] greeter: &Greeter, name: String) -> HttpResponse<Greeting> {
    HttpResponse::new(
        Greeting {
            message: format!("{} {}", greeter.prefix, name),
        },
        HttpStatus::Ok,
    )
}

#[test]
fn a_handler_receives_the_component_it_asks_for() {
    let response = record_hit("GET /hits");

    assert!(response.starts_with(EXPECTED_OK_PREFIX), "{response}");

    let hits: Hits = serde_json::from_str(body_of(&response)).expect("body should be Hits");

    assert!(hits.total >= 1);
}

#[test]
fn a_component_keeps_its_state_between_requests() {
    // The whole point of a component: the second request sees what the first did.
    // Asserted as strictly-greater rather than exactly-one-more because the other
    // tests in this file share the counter and run on their own threads.
    let first: Hits = serde_json::from_str(body_of(&record_hit("GET /hits"))).expect("first");
    let second: Hits = serde_json::from_str(body_of(&record_hit("GET /hits"))).expect("second");

    assert!(
        second.total > first.total,
        "the counter restarted: {} then {}",
        first.total,
        second.total
    );
}

#[test]
fn the_context_hands_back_one_shared_instance() {
    // A fresh value per call would satisfy every assertion above and still be wrong.
    assert!(std::ptr::eq(AppContext::counter(), AppContext::counter()));
}

#[test]
fn a_component_sits_alongside_a_path_parameter() {
    let response = greet("GET /greet/world");

    assert!(response.starts_with(EXPECTED_OK_PREFIX), "{response}");

    let greeting: Greeting = serde_json::from_str(body_of(&response)).expect("body");

    assert_eq!(greeting.message, "hello world");
}

#[test]
fn a_component_declared_before_the_body_does_not_take_it() {
    let response =
        create_greeting("POST /greetings HTTP/1.1\r\nContent-Type: application/json\r\n\r\nworld");

    assert!(response.starts_with(EXPECTED_OK_PREFIX), "{response}");

    let greeting: Greeting = serde_json::from_str(body_of(&response)).expect("body");

    assert_eq!(greeting.message, "hello world");
}

#[test]
fn each_component_gets_its_own_accessor() {
    // Two `#[component]` invocations contribute two inherent impl blocks to the same
    // `AppContext`, which is only legal because both land in this crate.
    assert_eq!(AppContext::greeter().prefix, "hello");
    assert!(AppContext::counter().record() >= 1);
}
