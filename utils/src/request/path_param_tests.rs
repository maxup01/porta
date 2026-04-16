use super::*;

fn collect(path: &str) -> Vec<String> {
    extract_path_param_names_from_path(path).collect()
}

// --- happy path ---

#[test]
fn single_param() {
    assert_eq!(collect("/users/{id}"), vec!["id"]);
}

#[test]
fn multiple_params() {
    assert_eq!(
        collect("/orgs/{org}/repos/{repo}/commits/{sha}"),
        vec!["org", "repo", "sha"]
    );
}

#[test]
fn param_at_root() {
    assert_eq!(collect("/{id}"), vec!["id"]);
}

#[test]
fn param_only_no_leading_slash() {
    assert_eq!(collect("{id}"), vec!["id"]);
}

#[test]
fn adjacent_params() {
    assert_eq!(collect("/{a}/{b}"), vec!["a", "b"]);
}

#[test]
fn preserves_order() {
    let result = collect("/a/{first}/b/{second}/c/{third}");
    assert_eq!(result, vec!["first", "second", "third"]);
}

// --- no params ---

#[test]
fn no_params_static_path() {
    assert!(collect("/users/profile/settings").is_empty());
}

#[test]
fn empty_path() {
    assert!(collect("").is_empty());
}

#[test]
fn root_path() {
    assert!(collect("/").is_empty());
}

// --- edge cases ---

#[test]
fn ignores_only_opening_brace() {
    // "{id" — not closed, must not be extracted
    assert!(collect("/users/{id").is_empty());
}

#[test]
fn ignores_only_closing_brace() {
    assert!(collect("/users/id}").is_empty());
}

#[test]
fn ignores_braces_in_middle_of_segment() {
    // "u{id}r" — braces not at boundaries
    assert!(collect("/users/u{id}r").is_empty());
}

#[test]
fn empty_braces_returned_as_empty_string() {
    // "{}" is technically matched; the extracted name is ""
    assert_eq!(collect("/users/{}"), vec![""]);
}

#[test]
fn param_name_with_underscores_and_digits() {
    assert_eq!(collect("/v2/{user_id_42}"), vec!["user_id_42"]);
}
