use super::*;

const ORIGIN: &str = "http://localhost:1420";

fn request_from(origin: &str) -> String {
    format!("GET /users HTTP/1.1\r\nOrigin: {}\r\n\r\n", origin)
}

fn preflight_from(origin: &str, method: &str, headers: Option<&str>) -> String {
    let requested_headers = match headers {
        Some(headers) => format!("Access-Control-Request-Headers: {}\r\n", headers),
        None => String::new(),
    };

    format!(
        "OPTIONS /users HTTP/1.1\r\nOrigin: {}\r\nAccess-Control-Request-Method: {}\r\n{}\r\n",
        origin, method, requested_headers
    )
}

fn value_of<'a>(headers: &'a [(&'static str, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(header_name, _)| *header_name == name)
        .map(|(_, value)| value.as_str())
}

// ── Configuration ────────────────────────────────────────────────────────────

#[test]
fn a_disabled_policy_emits_nothing() {
    let cors = CorsConfig::disabled();

    assert!(!cors.is_enabled());
    assert!(cors.response_headers(&request_from(ORIGIN)).is_empty());
    assert!(
        cors.preflight_headers(&preflight_from(ORIGIN, "POST", None), &[Method::POST])
            .is_none()
    );
}

#[test]
fn an_empty_origin_list_is_a_disabled_policy() {
    // Permitting nothing and configuring nothing are the same thing, and the
    // alternative — an enabled policy that matches no origin — would differ only
    // in how confusing it is to debug.
    assert!(!CorsConfig::new(&[], false, None).is_enabled());
    assert!(!CorsConfig::new(&["   "], false, None).is_enabled());
    assert!(!CorsConfig::new(&["", ""], false, None).is_enabled());
}

#[test]
fn origins_are_split_and_trimmed() {
    let cors = CorsConfig::new(&["  http://a.test ", "http://b.test  "], false, None);

    assert!(!cors.response_headers(&request_from("http://a.test")).is_empty());
    assert!(!cors.response_headers(&request_from("http://b.test")).is_empty());
    assert!(cors.response_headers(&request_from("http://c.test")).is_empty());
}

// ── Actual-response headers ──────────────────────────────────────────────────

#[test]
fn a_request_without_an_origin_gets_no_headers() {
    // Every non-browser client — curl, the reverse proxy, k6 — lands here.
    let cors = CorsConfig::new(&["*"], false, None);

    assert!(cors.response_headers("GET /users HTTP/1.1\r\n\r\n").is_empty());
}

#[test]
fn a_wildcard_policy_answers_with_a_literal_star_and_no_vary() {
    let cors = CorsConfig::new(&["*"], false, None);
    let headers = cors.response_headers(&request_from(ORIGIN));

    assert_eq!(value_of(&headers, "Access-Control-Allow-Origin"), Some("*"));
    // The answer is the same for every origin, so there is nothing for a cache to
    // vary on.
    assert_eq!(value_of(&headers, "Vary"), None);
}

#[test]
fn a_listed_origin_is_echoed_and_marked_as_varying() {
    let cors = CorsConfig::new(&[ORIGIN], false, None);
    let headers = cors.response_headers(&request_from(ORIGIN));

    assert_eq!(value_of(&headers, "Access-Control-Allow-Origin"), Some(ORIGIN));
    assert_eq!(value_of(&headers, "Vary"), Some("Origin"));
}

#[test]
fn an_unlisted_origin_gets_no_headers() {
    let cors = CorsConfig::new(&[ORIGIN], false, None);

    assert!(
        cors.response_headers(&request_from("http://evil.test"))
            .is_empty()
    );
}

#[test]
fn origin_matching_ignores_case() {
    let cors = CorsConfig::new(&["http://Localhost:1420"], false, None);

    assert!(!cors.response_headers(&request_from("http://localhost:1420")).is_empty());
}

#[test]
fn credentials_are_announced_when_configured() {
    let cors = CorsConfig::new(&[ORIGIN], true, None);
    let headers = cors.response_headers(&request_from(ORIGIN));

    assert_eq!(
        value_of(&headers, "Access-Control-Allow-Credentials"),
        Some("true")
    );
}

#[test]
fn a_credentialed_wildcard_echoes_the_origin_instead_of_a_star() {
    // A browser refuses `*` on a credentialed request, so a config that sent one
    // would fail every request it was meant to permit.
    let cors = CorsConfig::new(&["*"], true, None);
    let headers = cors.response_headers(&request_from(ORIGIN));

    assert_eq!(value_of(&headers, "Access-Control-Allow-Origin"), Some(ORIGIN));
    assert_eq!(value_of(&headers, "Vary"), Some("Origin"));
}

// ── Preflight ────────────────────────────────────────────────────────────────

#[test]
fn a_preflight_is_recognised_by_its_two_headers() {
    assert!(is_preflight(&preflight_from(ORIGIN, "POST", None)));

    // Origin but no requested method: a capability query from a page, not a
    // preflight.
    assert!(!is_preflight(&request_from(ORIGIN)));

    // Requested method but no origin: not something a browser sends.
    assert!(!is_preflight(
        "OPTIONS /users HTTP/1.1\r\nAccess-Control-Request-Method: POST\r\n\r\n"
    ));
}

#[test]
fn a_preflight_lists_the_methods_the_path_serves() {
    let cors = CorsConfig::new(&[ORIGIN], false, None);

    let headers = cors
        .preflight_headers(
            &preflight_from(ORIGIN, "POST", None),
            &[Method::GET, Method::POST],
        )
        .expect("preflight should be authorised");

    assert_eq!(
        value_of(&headers, "Access-Control-Allow-Methods"),
        Some("GET, POST")
    );
}

#[test]
fn a_preflight_for_an_unserved_method_still_lists_what_is_served() {
    // The browser compares its requested method against this list and blocks the
    // request itself. Echoing DELETE back would authorise a request the server
    // would then answer with 405.
    let cors = CorsConfig::new(&[ORIGIN], false, None);

    let headers = cors
        .preflight_headers(&preflight_from(ORIGIN, "DELETE", None), &[Method::GET])
        .expect("preflight should be authorised");

    assert_eq!(value_of(&headers, "Access-Control-Allow-Methods"), Some("GET"));
}

#[test]
fn requested_headers_are_echoed_by_default() {
    let cors = CorsConfig::new(&[ORIGIN], false, None);

    let headers = cors
        .preflight_headers(
            &preflight_from(ORIGIN, "POST", Some("content-type, authorization")),
            &[Method::POST],
        )
        .expect("preflight should be authorised");

    assert_eq!(
        value_of(&headers, "Access-Control-Allow-Headers"),
        Some("content-type, authorization")
    );
}

#[test]
fn a_configured_header_list_overrides_the_echo() {
    let cors = CorsConfig::new(&[ORIGIN], false, Some(&["content-type"]));

    let headers = cors
        .preflight_headers(
            &preflight_from(ORIGIN, "POST", Some("x-secret")),
            &[Method::POST],
        )
        .expect("preflight should be authorised");

    assert_eq!(
        value_of(&headers, "Access-Control-Allow-Headers"),
        Some("content-type")
    );
}

#[test]
fn a_preflight_that_asked_for_no_headers_gets_no_allow_headers() {
    let cors = CorsConfig::new(&[ORIGIN], false, None);

    let headers = cors
        .preflight_headers(&preflight_from(ORIGIN, "POST", None), &[Method::POST])
        .expect("preflight should be authorised");

    assert_eq!(value_of(&headers, "Access-Control-Allow-Headers"), None);
}

#[test]
fn a_preflight_carries_a_max_age() {
    let cors = CorsConfig::new(&[ORIGIN], false, None);

    let headers = cors
        .preflight_headers(&preflight_from(ORIGIN, "POST", None), &[Method::POST])
        .expect("preflight should be authorised");

    assert_eq!(
        value_of(&headers, "Access-Control-Max-Age"),
        Some(PREFLIGHT_MAX_AGE_SECONDS.to_string().as_str())
    );
}

#[test]
fn a_preflight_from_an_unlisted_origin_is_not_authorised() {
    let cors = CorsConfig::new(&[ORIGIN], false, None);

    assert!(
        cors.preflight_headers(
            &preflight_from("http://evil.test", "POST", None),
            &[Method::POST]
        )
        .is_none()
    );
}
