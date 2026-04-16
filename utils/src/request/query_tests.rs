use super::*;

// ── extract_params ────────────────────────────────────────────────────────────

// --- happy path ---

#[test]
fn single_param() {
    let params = extract_params("/search?q=rust").unwrap();
    assert_eq!(params["q"], "rust");
}

#[test]
fn multiple_params() {
    let params = extract_params("/search?q=rust&page=2&limit=10").unwrap();
    assert_eq!(params["q"], "rust");
    assert_eq!(params["page"], "2");
    assert_eq!(params["limit"], "10");
}

#[test]
fn param_with_empty_value() {
    let params = extract_params("/search?q=").unwrap();
    assert_eq!(params["q"], "");
}

#[test]
fn param_value_with_special_characters() {
    let params = extract_params("/search?q=hello+world").unwrap();
    assert_eq!(params["q"], "hello+world");
}

#[test]
fn param_value_with_equals_sign_is_preserved() {
    // split_once means only the first '=' splits the pair; rest goes into value
    let params = extract_params("/search?q=a=b").unwrap();
    assert_eq!(params["q"], "a=b");
}

// --- no query string ---

#[test]
fn no_query_string_returns_none() {
    assert!(extract_params("/search").is_none());
}

#[test]
fn root_path_no_query_returns_none() {
    assert!(extract_params("/").is_none());
}

#[test]
fn empty_input_returns_none() {
    assert!(extract_params("").is_none());
}

// --- empty / malformed pairs ---

#[test]
fn empty_key_pairs_are_discarded() {
    let params = extract_params("/path?=value&key=hello").unwrap();
    assert_eq!(params.len(), 1);
    assert_eq!(params["key"], "hello");
}

#[test]
fn all_empty_key_pairs_returns_empty_map() {
    let params = extract_params("/path?=value&=other").unwrap();
    assert!(params.is_empty());
}

#[test]
fn pair_without_equals_is_discarded() {
    // "foo" has no '=', split_once returns None, filter_map drops it
    let params = extract_params("/path?foo&key=hello").unwrap();
    assert_eq!(params.len(), 1);
    assert_eq!(params["key"], "hello");
}

#[test]
fn empty_query_string_returns_empty_map() {
    // "?" present but nothing after it
    let params = extract_params("/path?").unwrap();
    assert!(params.is_empty());
}

// --- edge cases ---

#[test]
fn duplicate_key_last_value_wins() {
    let params = extract_params("/path?key=first&key=second").unwrap();
    assert_eq!(params["key"], "second");
}

#[test]
fn query_string_only_no_path() {
    let params = extract_params("?key=value").unwrap();
    assert_eq!(params["key"], "value");
}

#[test]
fn only_question_mark_returns_empty_map() {
    let params = extract_params("?").unwrap();
    assert!(params.is_empty());
}
