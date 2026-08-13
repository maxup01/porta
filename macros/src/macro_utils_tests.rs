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

// These assert on the *shape* of the emitted tokens, not on the response bytes.
// The bytes are `utils::response::status_response`'s responsibility and are
// covered directly in `response_tests.rs`; all that matters here is that the
// generated code reaches for the right `HttpStatus` variant.

#[test]
fn numeric_parse_failure_returns_404() {
    let f: ItemFn = parse_quote! { fn h(id: u32) -> String { String::new() } };
    let src = handler_src(f, "/r/{id}", Method::GET);
    assert!(src.contains("status_response") && src.contains("NotFound"));
}

#[test]
fn bool_unknown_value_returns_404() {
    let f: ItemFn = parse_quote! { fn h(flag: bool) -> String { String::new() } };
    let src = handler_src(f, "/r/{flag}", Method::GET);
    assert!(src.contains("status_response") && src.contains("NotFound"));
}

#[test]
fn serde_failure_returns_404() {
    let f: ItemFn = parse_quote! { fn h(payload: MyDto) -> String { String::new() } };
    let src = handler_src(f, "/r/{payload}", Method::GET);
    assert!(src.contains("status_response") && src.contains("NotFound"));
}

#[test]
fn missing_param_returns_400() {
    let f: ItemFn = parse_quote! { fn h(id: u32) -> String { String::new() } };
    let src = handler_src(f, "/r/{id}", Method::GET);
    assert!(src.contains("status_response") && src.contains("BadRequest"));
}

#[test]
fn no_response_bytes_are_built_inside_the_macro() {
    let f: ItemFn = parse_quote! { fn h(id: u32, payload: MyDto) -> String { String::new() } };

    for method in [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
    ] {
        let src = handler_src(f.clone(), "/r/{id}", method);

        assert!(
            !src.contains("HTTP/1.1"),
            "generated code hand-rolls a response instead of calling status_response"
        );
    }
}

// ── HttpServerArgs ───────────────────────────────────────────────────────────

fn parse_args(args: &str) -> syn::Result<HttpServerArgs> {
    syn::parse_str::<HttpServerArgs>(args)
}

#[test]
fn parses_the_required_arguments() {
    let args = parse_args(r#"ip = "127.0.0.1", port = 8443"#).expect("should parse");

    assert_eq!(args.ip, "127.0.0.1");
    assert_eq!(args.port, 8443);
    // Absent means no CORS at all, not CORS with nothing allowed — the same
    // behaviour the server had before the argument existed.
    assert!(args.allow_origins.is_empty());
}

#[test]
fn parses_an_origin_list() {
    let args = parse_args(
        r#"ip = "0.0.0.0", port = 80, allow_origins = ["http://localhost:1420", "tauri://localhost"]"#,
    )
    .expect("should parse");

    assert_eq!(
        args.allow_origins,
        vec!["http://localhost:1420", "tauri://localhost"]
    );
}

#[test]
fn parses_a_single_origin_and_a_wildcard() {
    let args = parse_args(r#"ip = "0.0.0.0", port = 80, allow_origins = ["*"]"#)
        .expect("should parse");

    assert_eq!(args.allow_origins, vec!["*"]);
}

#[test]
fn an_empty_origin_list_parses_and_permits_nothing() {
    let args =
        parse_args(r#"ip = "0.0.0.0", port = 80, allow_origins = []"#).expect("should parse");

    assert!(args.allow_origins.is_empty());
}

#[test]
fn arguments_may_be_given_in_any_order() {
    let args = parse_args(r#"allow_origins = ["http://a.test"], port = 8443, ip = "127.0.0.1""#)
        .expect("should parse");

    assert_eq!(args.ip, "127.0.0.1");
    assert_eq!(args.port, 8443);
    assert_eq!(args.allow_origins, vec!["http://a.test"]);
}

#[test]
fn a_trailing_comma_is_accepted() {
    assert!(parse_args(r#"ip = "127.0.0.1", port = 8443,"#).is_ok());
    assert!(parse_args(r#"ip = "127.0.0.1", port = 8443, allow_origins = ["a", ],"#).is_ok());
}

#[test]
fn a_missing_required_argument_is_an_error() {
    assert!(parse_args(r#"port = 8443"#).is_err());
    assert!(parse_args(r#"ip = "127.0.0.1""#).is_err());
    assert!(parse_args("").is_err());
}

#[test]
fn an_unknown_argument_is_an_error() {
    // Silently ignoring it is how `allow_origin = [..]` turns into a CORS failure
    // with nothing to read in the build output.
    let error = parse_args(r#"ip = "127.0.0.1", port = 8443, allow_origin = ["a"]"#)
        .expect_err("unknown argument should be rejected");

    assert!(
        error.to_string().contains("allow_origin"),
        "the error should name the argument: {error}"
    );
}

#[test]
fn a_repeated_argument_is_an_error() {
    let error = parse_args(r#"ip = "0.0.0.0", ip = "127.0.0.1", port = 8443"#)
        .expect_err("a repeated argument should be rejected");

    assert!(error.to_string().contains("more than once"), "{error}");
}

#[test]
fn a_port_outside_u16_is_an_error() {
    assert!(parse_args(r#"ip = "127.0.0.1", port = 70000"#).is_err());
}

#[test]
fn an_origin_list_of_non_strings_is_an_error() {
    assert!(parse_args(r#"ip = "127.0.0.1", port = 8443, allow_origins = [42]"#).is_err());
    assert!(parse_args(r#"ip = "127.0.0.1", port = 8443, allow_origins = "http://a.test""#).is_err());
}
