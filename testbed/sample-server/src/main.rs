//! A downstream consumer of `porta`, used as a manual playground and
//! as the target for the k6 load tests in `../k6`.
//!
//! Nothing here reaches into the crate's internals — it depends only on what a real
//! user gets from `use porta::*`, which is what makes it a test of the
//! macro expansion rather than of the workspace. Every path the macros emit resolves
//! through this crate's single dependency.
//!
//! The routes are chosen to cover behaviour the workspace tests cannot reach over a
//! real socket: parameter binding of each kind, route specificity, and the statuses
//! the server generates on its own.

use porta::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    active: bool,
}

#[derive(Serialize, Deserialize)]
struct NewUser {
    name: String,
    active: bool,
}

// ── GET ──────────────────────────────────────────────────────────────────────

/// No parameters at all — the cheapest possible route, and the one to point a load
/// test at when measuring the handshake floor rather than the dispatch path.
#[get(path = "/")]
fn index() -> HttpResponse<String> {
    HttpResponse::new("up".to_string(), HttpStatus::Ok)
}

/// A path parameter parsed into a numeric type. `/users/abc` exercises the failure
/// side of the same code and answers `404`.
#[get(path = "/users/{id}")]
fn get_user(id: u64) -> HttpResponse<User> {
    HttpResponse::new(
        User {
            id,
            name: "ada".to_string(),
            active: true,
        },
        HttpStatus::Ok,
    )
}

/// Registered alongside `/users/{id}` deliberately. A literal segment has fewer
/// `{param}` segments than a parameterised one, so this must win for `/users/me`
/// while `/users/42` still reaches the route above.
#[get(path = "/users/me")]
fn current_user() -> HttpResponse<User> {
    HttpResponse::new(
        User {
            id: 1,
            name: "me".to_string(),
            active: true,
        },
        HttpStatus::Ok,
    )
}

/// Two query parameters bound by name, one of them parsed to a number. Omitting
/// either answers `400`, since a missing parameter is a client error rather than a
/// missing resource.
#[get(path = "/search")]
fn search(q: String, limit: u32) -> HttpResponse<String> {
    HttpResponse::new(format!("{} x{}", q, limit), HttpStatus::Ok)
}

// ── POST ─────────────────────────────────────────────────────────────────────

/// A JSON body deserialized straight into an argument, answering `201` rather than
/// `200` so the status line is visibly not hardcoded.
#[post(path = "/users")]
fn create_user(body: NewUser) -> HttpResponse<User> {
    HttpResponse::new(
        User {
            id: 7,
            name: body.name,
            active: body.active,
        },
        HttpStatus::Created,
    )
}

// ── PUT ──────────────────────────────────────────────────────────────────────

/// A path parameter and a body in the same handler: `id` comes from the URL, and
/// `body` is bound because it is the argument that is not a path parameter.
#[put(path = "/users/{id}")]
fn replace_user(id: u64, body: NewUser) -> HttpResponse<User> {
    HttpResponse::new(
        User {
            id,
            name: body.name,
            active: body.active,
        },
        HttpStatus::Ok,
    )
}

// ── PATCH ────────────────────────────────────────────────────────────────────

#[patch(path = "/users/{id}")]
fn update_user(id: u64, body: NewUser) -> HttpResponse<User> {
    HttpResponse::new(
        User {
            id,
            name: body.name,
            active: body.active,
        },
        HttpStatus::Ok,
    )
}

// ── DELETE ───────────────────────────────────────────────────────────────────

#[delete(path = "/users/{id}")]
fn delete_user(id: u64) -> HttpResponse<String> {
    HttpResponse::new(format!("deleted {}", id), HttpStatus::Ok)
}

/// Returns a body that must never reach the wire: `204` is defined to carry no
/// content, so the response should arrive with neither a body nor `Content-Length`.
#[delete(path = "/sessions/{id}")]
fn end_session(id: u64) -> HttpResponse<String> {
    HttpResponse::new(format!("ended {}", id), HttpStatus::NoContent)
}

// ── Server ───────────────────────────────────────────────────────────────────

// `/users` is served by GET, PUT, PATCH and DELETE but not POST-with-an-id, so
// `POST /users/42` answers `405` while `POST /nowhere` answers `404`. That split is
// the only thing distinguishing a wrong verb from a missing resource.
//
// `allow_origins` is set so the preflight path is exercised by something other
// than a unit test. k6 sends no `Origin` header, so it sees none of it: the load
// figures are unaffected, and the CORS headers appear only for a request that
// claims to come from that origin.
#[http_server(
    ip = "127.0.0.1",
    port = 8443,
    allow_origins = ["http://localhost:1420"]
)]
async fn main() {}
