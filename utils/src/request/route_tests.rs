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
    assert!(Method::from_str("PUT").is_err());
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

    let result = get_route_function("/test/exact", Method::GET).unwrap();
    assert!(result.is_some());
}

#[test]
fn register_and_lookup_route_with_query_string() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::POST, "/test/query", handler);

    let result = get_route_function("/test/query?foo=bar", Method::POST).unwrap();
    assert!(result.is_some());
}

#[test]
fn lookup_unregistered_route_returns_none() {
    let result = get_route_function("/does/not/exist", Method::GET).unwrap();
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
    assert!(get_route_function("/test/method-agnostic", Method::GET).unwrap().is_none());
}

#[test]
fn registered_route_not_found_under_wrong_method() {
    fn handler(_: &str) -> String {
        "ok".to_string()
    }

    register_route(Method::GET, "/test/method-check", handler);

    let result = get_route_function("/test/method-check", Method::DELETE).unwrap();
    assert!(result.is_none());
}
