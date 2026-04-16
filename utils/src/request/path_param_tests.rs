use super::*;

// ── extract_path_param_names_from_path ───────────────────────────────────────

// --- happy path ---

#[test]
fn single_param() {
    let result: Vec<String> = extract_path_param_names_from_path("/users/{id}").collect();
    assert_eq!(result, vec!["id"]);
}

#[test]
fn multiple_params() {
    let result: Vec<String> =
        extract_path_param_names_from_path("/orgs/{org}/repos/{repo}/commits/{sha}").collect();
    assert_eq!(result, vec!["org", "repo", "sha"]);
}

#[test]
fn param_at_root() {
    let result: Vec<String> = extract_path_param_names_from_path("/{id}").collect();
    assert_eq!(result, vec!["id"]);
}

#[test]
fn param_only_no_leading_slash() {
    let result: Vec<String> = extract_path_param_names_from_path("{id}").collect();
    assert_eq!(result, vec!["id"]);
}

#[test]
fn adjacent_params() {
    let result: Vec<String> = extract_path_param_names_from_path("/{a}/{b}").collect();
    assert_eq!(result, vec!["a", "b"]);
}

#[test]
fn preserves_order() {
    let result: Vec<String> =
        extract_path_param_names_from_path("/a/{first}/b/{second}/c/{third}").collect();
    assert_eq!(result, vec!["first", "second", "third"]);
}

// --- no params ---

#[test]
fn no_params_static_path() {
    let result: Vec<String> =
        extract_path_param_names_from_path("/users/profile/settings").collect();
    assert!(result.is_empty());
}

#[test]
fn empty_path() {
    let result: Vec<String> = extract_path_param_names_from_path("").collect();
    assert!(result.is_empty());
}

#[test]
fn root_path() {
    let result: Vec<String> = extract_path_param_names_from_path("/").collect();
    assert!(result.is_empty());
}

// --- edge cases ---

#[test]
fn ignores_only_opening_brace() {
    let result: Vec<String> = extract_path_param_names_from_path("/users/{id").collect();
    assert!(result.is_empty());
}

#[test]
fn ignores_only_closing_brace() {
    let result: Vec<String> = extract_path_param_names_from_path("/users/id}").collect();
    assert!(result.is_empty());
}

#[test]
fn ignores_braces_in_middle_of_segment() {
    let result: Vec<String> = extract_path_param_names_from_path("/users/u{id}r").collect();
    assert!(result.is_empty());
}

#[test]
fn empty_braces_returned_as_empty_string() {
    let result: Vec<String> = extract_path_param_names_from_path("/users/{}").collect();
    assert_eq!(result, vec![""]);
}

#[test]
fn param_name_with_underscores_and_digits() {
    let result: Vec<String> = extract_path_param_names_from_path("/v2/{user_id_42}").collect();
    assert_eq!(result, vec!["user_id_42"]);
}

// ── extract_path_params ───────────────────────────────────────────────────────

// --- happy path ---

#[test]
fn extract_single_param() {
    let params = extract_path_params("/users/{id}", "/users/42").unwrap();
    assert_eq!(params["id"], "42");
}

#[test]
fn extract_multiple_params() {
    let params =
        extract_path_params("/orgs/{org}/repos/{repo}", "/orgs/rust-lang/repos/cargo").unwrap();
    assert_eq!(params["org"], "rust-lang");
    assert_eq!(params["repo"], "cargo");
}

#[test]
fn extract_static_segments_are_ignored() {
    let params = extract_path_params("/users/all", "/users/all").unwrap();
    assert!(params.is_empty());
}

#[test]
fn extract_mixed_static_and_param_segments() {
    let params = extract_path_params("/users/{id}/settings", "/users/99/settings").unwrap();
    assert_eq!(params["id"], "99");
    assert_eq!(params.len(), 1);
}

#[test]
fn extract_param_value_with_hyphens_and_dots() {
    let params = extract_path_params("/files/{name}", "/files/my-report.pdf").unwrap();
    assert_eq!(params["name"], "my-report.pdf");
}

// --- errors ---

#[test]
fn extract_err_when_path_is_longer_than_route() {
    assert!(extract_path_params("/users/{id}", "/users/42/posts").is_err());
}

#[test]
fn extract_err_when_path_is_shorter_than_route() {
    assert!(extract_path_params("/users/{id}/posts", "/users/42").is_err());
}

#[test]
fn extract_err_for_empty_path_against_non_empty_route() {
    assert!(extract_path_params("/users/{id}", "").is_err());
}

#[test]
fn extract_err_for_root_path_against_non_root_route() {
    assert!(extract_path_params("/users/{id}", "/").is_err());
}

// --- edge cases ---

#[test]
fn extract_both_root_paths_returns_empty_map() {
    let params = extract_path_params("/", "/").unwrap();
    assert!(params.is_empty());
}

#[test]
fn extract_param_name_with_underscores_and_digits() {
    let params = extract_path_params("/v2/{user_id_42}", "/v2/hello").unwrap();
    assert_eq!(params["user_id_42"], "hello");
}
