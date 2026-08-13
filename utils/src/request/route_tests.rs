use super::*;

// ── Method::from_str ────────────────────────────────────────────────────

#[test]
fn method_from_str_uppercase() {
    assert!(matches!(Method::from_str("GET").unwrap(), Method::GET));
    assert!(matches!(Method::from_str("POST").unwrap(), Method::POST));
    assert!(matches!(Method::from_str("PATCH").unwrap(), Method::PATCH));
    assert!(matches!(
        Method::from_str("DELETE").unwrap(),
        Method::DELETE
    ));
}

#[test]
fn method_from_str_lowercase() {
    assert!(matches!(Method::from_str("get").unwrap(), Method::GET));
    assert!(matches!(Method::from_str("post").unwrap(), Method::POST));
    assert!(matches!(Method::from_str("put").unwrap(), Method::PUT));
    assert!(matches!(Method::from_str("patch").unwrap(), Method::PATCH));
    assert!(matches!(
        Method::from_str("delete").unwrap(),
        Method::DELETE
    ));
}

#[test]
fn method_from_str_mixed_case() {
    assert!(matches!(Method::from_str("Get").unwrap(), Method::GET));
    assert!(matches!(Method::from_str("pOsT").unwrap(), Method::POST));
}

#[test]
fn method_from_str_unsupported_returns_err() {
    assert!(Method::from_str("CONNECT").is_err());
    assert!(Method::from_str("OPTIONS").is_err());
    assert!(Method::from_str("HEAD").is_err());
    assert!(Method::from_str("").is_err());
}

// ── path_param_segment ──────────────────────────────────────────────────

#[test]
fn path_param_segment_valid() {
    assert!(path_param_segment("{id}"));
    assert!(path_param_segment("{user_id}"));
}

#[test]
fn path_param_segment_invalid() {
    assert!(!path_param_segment("users"));
    assert!(!path_param_segment("{id"));
    assert!(!path_param_segment("id}"));
    assert!(!path_param_segment(""));
}

// ── fixed_path_segment ──────────────────────────────────────────────────

#[test]
fn fixed_path_segment_valid() {
    assert!(fixed_path_segment("users"));
    assert!(fixed_path_segment("health"));
    assert!(fixed_path_segment("{id"));
    assert!(fixed_path_segment("id}"));
}

#[test]
fn fixed_path_segment_invalid() {
    assert!(!fixed_path_segment("{id}"));
    assert!(!fixed_path_segment("{user_id}"));
}

// ── is_path_matching_route_path ─────────────────────────────────────────

#[test]
fn path_matches_exact_route() {
    assert!(is_path_matching_route_path("/users/list", "/users/list"));
    assert!(is_path_matching_route_path("/health", "/health"));
}

#[test]
fn path_matches_parameterised_route() {
    assert!(is_path_matching_route_path("/users/42", "/users/{id}"));
    assert!(is_path_matching_route_path("/users/abc", "/users/{id}"));
}

#[test]
fn path_does_not_match_different_fixed_segment() {
    assert!(!is_path_matching_route_path("/posts/42", "/users/{id}"));
    assert!(!is_path_matching_route_path("/users/list", "/users/detail"));
}

#[test]
fn path_does_not_match_different_segment_count() {
    assert!(!is_path_matching_route_path(
        "/users/42/posts",
        "/users/{id}"
    ));
    assert!(!is_path_matching_route_path("/users", "/users/{id}"));
}

#[test]
fn path_matches_multiple_params() {
    assert!(is_path_matching_route_path(
        "/users/42/posts/7",
        "/users/{id}/posts/{post_id}"
    ));
}

// ── extract_path_from_request ───────────────────────────────────────────

#[test]
fn extract_path_standard_request_line() {
    assert_eq!(
        extract_path_from_request("GET /users HTTP/1.1").unwrap(),
        "/users"
    );
}

#[test]
fn extract_path_preserves_query_string() {
    assert_eq!(
        extract_path_from_request("GET /users?id=1 HTTP/1.1").unwrap(),
        "/users?id=1"
    );
}

#[test]
fn extract_path_malformed_request_returns_err() {
    assert!(extract_path_from_request("MALFORMED").is_err());
    assert!(extract_path_from_request("").is_err());
}

// ── extract_method_from_request ─────────────────────────────────────────

#[test]
fn extract_method_standard_request_line() {
    assert!(matches!(
        extract_method_from_request("GET /users HTTP/1.1").unwrap(),
        Method::GET
    ));
    assert!(matches!(
        extract_method_from_request("POST /users HTTP/1.1").unwrap(),
        Method::POST
    ));
}

#[test]
fn extract_method_malformed_request_returns_err() {
    assert!(extract_method_from_request("MALFORMED").is_err());
    assert!(extract_method_from_request("").is_err());
}

#[test]
fn extract_method_unsupported_method_returns_err() {
    assert!(extract_method_from_request("CONNECT /users HTTP/1.1").is_err());
}

// ── register_route / get_route_function / path_exists ───────────────────

#[test]
fn register_and_lookup_exact_route() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::GET, "/test/exact", handler);

    let result = get_route_function("/test/exact", Method::GET);
    assert!(result.is_some());
}

#[test]
fn register_and_lookup_route_with_query_string() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::POST, "/test/query", handler);

    let result = get_route_function("/test/query?foo=bar", Method::POST);
    assert!(result.is_some());
}

#[test]
fn lookup_unregistered_route_returns_none() {
    let result = get_route_function("/does/not/exist", Method::GET);
    assert!(result.is_none());
}

#[test]
fn register_and_match_parameterised_route() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::GET, "/test/{id}/param", handler);

    assert!(path_exists("/test/42/param"));
}

#[test]
fn path_exists_no_match_returns_false() {
    assert!(!path_exists("/definitely/not/registered/ever"));
}

#[test]
fn path_exists_ignores_the_method_it_was_registered_under() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::PATCH, "/test/method-agnostic", handler);

    // Registered for PATCH only, but the path itself is served — this is what
    // separates a 405 from a 404.
    assert!(path_exists("/test/method-agnostic"));
    assert!(
        get_route_function("/test/method-agnostic", Method::GET).is_none()
    );
}

#[test]
fn registered_route_not_found_under_wrong_method() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::GET, "/test/method-check", handler);

    let result = get_route_function("/test/method-check", Method::DELETE);
    assert!(result.is_none());
}

// ── 405 vs 404 dispatch decision ────────────────────────────────────────
//
// `http_server` answers a request by asking two questions in order:
//
//   1. `get_route_function(path, method)` — Some => dispatch to the handler
//   2. `path_exists(path)`                — true => 405, false => 404
//
// The tests below pin every combination of those two answers. Each uses its
// own first path segment, because the route tables are process-wide statics
// shared by every test in this binary and matching is structural, not exact.

/// `Some(handler)` — the request is dispatched, neither 404 nor 405.
#[test]
fn dispatch_handler_found_for_matching_method() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::GET, "/dispatch-hit/{id}", handler);

    assert!(
        get_route_function("/dispatch-hit/42", Method::GET).is_some()
    );
}

/// `None` + `path_exists` == true — the 405 case, on a parameterised route.
#[test]
fn dispatch_405_when_path_is_served_by_another_method() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::POST, "/dispatch-405/{id}", handler);

    assert!(
        get_route_function("/dispatch-405/42", Method::GET).is_none(),
        "GET must not resolve a POST-only route"
    );
    assert!(
        path_exists("/dispatch-405/42"),
        "the path is served by POST, so this is a 405 and not a 404"
    );
}

/// `None` + `path_exists` == false — the 404 case.
#[test]
fn dispatch_404_when_no_method_serves_the_path() {
    assert!(
        get_route_function("/dispatch-404/nothing/here", Method::GET).is_none()
    );
    assert!(!path_exists("/dispatch-404/nothing/here"));
}

/// A verb no handler was ever registered for still yields 405, not 404.
#[test]
fn dispatch_405_for_every_other_verb() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::GET, "/dispatch-verbs/resource", handler);

    for method in [Method::POST, Method::PUT, Method::PATCH, Method::DELETE] {
        assert!(
            get_route_function("/dispatch-verbs/resource", method).is_none()
        );
    }

    assert!(path_exists("/dispatch-verbs/resource"));
}

/// A 405 survives a query string, which is stripped before matching.
#[test]
fn dispatch_405_ignores_the_query_string() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::DELETE, "/dispatch-query/items", handler);

    assert!(
        get_route_function("/dispatch-query/items?id=5", Method::GET).is_none()
    );
    assert!(path_exists("/dispatch-query/items"));
}

// ── regressions ─────────────────────────────────────────────────────────

/// Two methods on the same route, spelled with *different* parameter names.
///
/// Resolution used to run method-blind over a shared path list and then look the
/// winning pattern up in the target method's table, so `/regression-names/{id}`
/// would win for a POST and miss `POST_ROUTES`, which is keyed by
/// `/regression-names/{user_id}` — a 404 despite a registered handler.
#[test]
fn both_methods_resolve_when_param_names_differ() {
    fn get_handler(_: &str) -> String {
        "get".to_string()
    }
    fn post_handler(_: &str) -> String {
        "post".to_string()
    }

    register_route(Method::GET, "/regression-names/{id}", get_handler);
    register_route(Method::POST, "/regression-names/{user_id}", post_handler);

    assert!(
        get_route_function("/regression-names/42", Method::GET).is_some()
    );
    assert!(
        get_route_function("/regression-names/42", Method::POST).is_some(),
        "a POST route must not be shadowed by a GET route of the same shape"
    );
}

/// A literal segment must beat a parameter in the same position, whatever the
/// order they were registered in or the map happens to iterate them.
#[test]
fn literal_segment_wins_over_parameter() {
    fn param_handler(_: &str) -> String {
        "param".to_string()
    }
    fn literal_handler(_: &str) -> String {
        "literal".to_string()
    }

    register_route(Method::GET, "/regression-specificity/{id}", param_handler);
    register_route(Method::GET, "/regression-specificity/me", literal_handler);

    let resolved = get_route_function("/regression-specificity/me", Method::GET)
        .expect("the literal route must be reachable");

    assert_eq!(
        resolved(""),
        "literal",
        "/regression-specificity/me must not be shadowed by /regression-specificity/{{id}}"
    );

    // The parameterised route still serves everything else.
    let resolved = get_route_function("/regression-specificity/42", Method::GET)
        .expect("the parameterised route must still match other values");

    assert_eq!(resolved(""), "param");
}

// ── methods_for_path ────────────────────────────────────────────────────

/// The `Allow` header and `Access-Control-Allow-Methods` both need the list, in
/// a stable order, not just the yes/no `path_exists` gives.
#[test]
fn methods_for_path_collects_every_verb_that_serves_it() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::GET, "/allow-list/{id}", handler);
    register_route(Method::DELETE, "/allow-list/{id}", handler);
    register_route(Method::PUT, "/allow-list/{id}", handler);

    assert_eq!(
        methods_for_path("/allow-list/42"),
        vec![Method::GET, Method::PUT, Method::DELETE],
        "methods must come back in ROUTABLE_METHODS order, not table order"
    );
}

#[test]
fn methods_for_path_is_empty_when_nothing_serves_the_path() {
    assert!(methods_for_path("/allow-list-nothing/here").is_empty());
}

#[test]
fn methods_for_path_and_path_exists_agree() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::PATCH, "/allow-agreement/thing", handler);

    assert_eq!(
        path_exists("/allow-agreement/thing"),
        !methods_for_path("/allow-agreement/thing").is_empty()
    );
    assert_eq!(
        path_exists("/allow-agreement/absent"),
        !methods_for_path("/allow-agreement/absent").is_empty()
    );
}

// ── extract_method_token_from_request ───────────────────────────────────

/// The verbs `Method` refuses are exactly the ones this has to survive: OPTIONS
/// is answered by the server itself, and cannot be recognised through a parser
/// that rejects it.
#[test]
fn method_token_survives_verbs_that_do_not_parse() {
    assert_eq!(
        extract_method_token_from_request("OPTIONS /users HTTP/1.1").unwrap(),
        "OPTIONS"
    );
    assert!(extract_method_from_request("OPTIONS /users HTTP/1.1").is_err());
}

#[test]
fn method_token_is_returned_verbatim() {
    assert_eq!(
        extract_method_token_from_request("get /users HTTP/1.1").unwrap(),
        "get"
    );
}

#[test]
fn method_token_needs_a_request_line() {
    assert!(extract_method_token_from_request("MALFORMED").is_err());
}
