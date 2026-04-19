use super::*;
use syn::{parse_quote, FnArg, ItemFn};
use utils::request::route::Method;

fn parse_fn_args(input: ItemFn) -> syn::punctuated::Punctuated<FnArg, syn::token::Comma> {
    input.sig.inputs
}

fn handler_src(f: ItemFn, path: &str, method: Method) -> String {
    generate_route_handler_tokens(path, method, &f).to_string()
}

// ── get_input_arg_idents_and_types ───────────────────────────────────────────

#[test]
fn extracts_single_primitive_arg() {
    let f: ItemFn = parse_quote! { fn foo(id: u32) {} };
    let result = get_input_arg_idents_and_types(&parse_fn_args(f));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0.to_string(), "id");
}

#[test]
fn extracts_multiple_args() {
    let f: ItemFn = parse_quote! { fn foo(id: u32, name: String, active: bool) {} };
    let result = get_input_arg_idents_and_types(&parse_fn_args(f));
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].0.to_string(), "id");
    assert_eq!(result[1].0.to_string(), "name");
    assert_eq!(result[2].0.to_string(), "active");
}

#[test]
fn returns_empty_vec_for_no_args() {
    let f: ItemFn = parse_quote! { fn foo() {} };
    let result = get_input_arg_idents_and_types(&parse_fn_args(f));
    assert!(result.is_empty());
}

#[test]
fn extracts_complex_type_arg() {
    let f: ItemFn = parse_quote! { fn foo(payload: MyStruct) {} };
    let result = get_input_arg_idents_and_types(&parse_fn_args(f));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0.to_string(), "payload");
}

// ── generate_route_handler_tokens ────────────────────────────────────────────

#[test]
fn get_handler_contains_fn_name() {
    let f: ItemFn = parse_quote! { pub fn get_user(id: u32) -> String { format!("{}", id) } };
    let src = handler_src(f, "/users/{id}", Method::GET);
    assert!(src.contains("get_user"));
}

#[test]
fn get_handler_registers_route() {
    let f: ItemFn = parse_quote! { fn list_items() -> String { String::new() } };
    let src = handler_src(f, "/items", Method::GET);
    assert!(src.contains("register_route_list_items"));
    assert!(src.contains("register_route"));
}

#[test]
fn post_handler_includes_request_body_extraction() {
    let f: ItemFn = parse_quote! { fn create_item(name: String) -> String { name } };
    let src = handler_src(f, "/items", Method::POST);
    assert!(src.contains("extract_request_body"));
}

#[test]
fn patch_handler_includes_request_body_extraction() {
    let f: ItemFn = parse_quote! { fn update_item(id: u32, payload: String) -> String { payload } };
    let src = handler_src(f, "/items/{id}", Method::PATCH);
    assert!(src.contains("extract_request_body"));
}

#[test]
fn delete_handler_omits_request_body_extraction() {
    let f: ItemFn = parse_quote! { fn delete_item(id: u32) -> String { String::new() } };
    let src = handler_src(f, "/items/{id}", Method::DELETE);
    assert!(!src.contains("extract_request_body"));
}

#[test]
fn get_handler_omits_request_body_extraction() {
    let f: ItemFn = parse_quote! { fn get_item(id: u32) -> String { String::new() } };
    let src = handler_src(f, "/items/{id}", Method::GET);
    assert!(!src.contains("extract_request_body"));
}

#[test]
fn handler_uses_correct_path() {
    let f: ItemFn = parse_quote! { fn handler(id: u32) -> String { String::new() } };
    let src = handler_src(f, "/things/{id}", Method::GET);
    assert!(src.contains("/things/{id}"));
}

#[test]
fn handler_contains_format_response_call() {
    let f: ItemFn = parse_quote! { fn handler() -> String { String::new() } };
    let src = handler_src(f, "/ping", Method::GET);
    assert!(src.contains("format_response"));
}

#[test]
fn handler_method_token_get() {
    let f: ItemFn = parse_quote! { fn handler() -> String { String::new() } };
    let src = handler_src(f, "/ping", Method::GET);
    assert!(src.contains("GET"));
}

#[test]
fn handler_method_token_post() {
    let f: ItemFn = parse_quote! { fn handler(body: String) -> String { body } };
    let src = handler_src(f, "/ping", Method::POST);
    assert!(src.contains("POST"));
}

#[test]
fn handler_method_token_delete() {
    let f: ItemFn = parse_quote! { fn handler(id: u32) -> String { String::new() } };
    let src = handler_src(f, "/items/{id}", Method::DELETE);
    assert!(src.contains("DELETE"));
}

#[test]
fn handler_method_token_patch() {
    let f: ItemFn = parse_quote! { fn handler(id: u32, data: String) -> String { data } };
    let src = handler_src(f, "/items/{id}", Method::PATCH);
    assert!(src.contains("PATCH"));
}

// ── generate_deserialization_block ───────────────────────────────────────────

#[test]
fn numeric_types_use_parse() {
    for ty in &[
        "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize", "f32", "f64",
    ] {
        let f: ItemFn =
            syn::parse_str(&format!("fn h(val: {}) -> String {{ String::new() }}", ty)).unwrap();
        let src = handler_src(f, "/r/{val}", Method::GET);
        eprintln!("{}", src);
        assert!(
            src.contains(" . parse ()"),
            "expected .parse() for type {}",
            ty
        );
    }
}

#[test]
fn bool_type_matches_true_false_strings() {
    let f: ItemFn = parse_quote! { fn h(flag: bool) -> String { String::new() } };
    let src = handler_src(f, "/r/{flag}", Method::GET);
    assert!(src.contains("true") && src.contains("false"));
}

#[test]
fn string_type_calls_to_string() {
    let f: ItemFn = parse_quote! { fn h(name: String) -> String { name } };
    let src = handler_src(f, "/r/{name}", Method::GET);
    assert!(src.contains("to_string"));
}

#[test]
fn custom_type_uses_serde_json_from_str() {
    let f: ItemFn = parse_quote! { fn h(payload: MyDto) -> String { String::new() } };
    let src = handler_src(f, "/r/{payload}", Method::GET);
    assert!(src.contains("serde_json") && src.contains("from_str"));
}

#[test]
fn custom_type_wraps_in_quotes_when_not_json_object() {
    let f: ItemFn = parse_quote! { fn h(val: MyEnum) -> String { String::new() } };
    let src = handler_src(f, "/r/{val}", Method::GET);
    assert!(src.contains("starts_with") && src.contains("ends_with"));
}

#[test]
fn numeric_parse_failure_returns_404() {
    let f: ItemFn = parse_quote! { fn h(id: u32) -> String { String::new() } };
    let src = handler_src(f, "/r/{id}", Method::GET);
    assert!(src.contains("404 Not Found"));
}

#[test]
fn bool_unknown_value_returns_404() {
    let f: ItemFn = parse_quote! { fn h(flag: bool) -> String { String::new() } };
    let src = handler_src(f, "/r/{flag}", Method::GET);
    assert!(src.contains("404 Not Found"));
}

#[test]
fn serde_failure_returns_404() {
    let f: ItemFn = parse_quote! { fn h(payload: MyDto) -> String { String::new() } };
    let src = handler_src(f, "/r/{payload}", Method::GET);
    assert!(src.contains("404 Not Found"));
}
