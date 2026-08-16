use super::*;
use syn::{FnArg, ItemFn, parse_quote};
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
    let args =
        parse_args(r#"ip = "0.0.0.0", port = 80, allow_origins = ["*"]"#).expect("should parse");

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
    assert!(
        parse_args(r#"ip = "127.0.0.1", port = 8443, allow_origins = "http://a.test""#).is_err()
    );
}

// ── ComponentArgs ────────────────────────────────────────────────────────────

fn parse_component_args(args: &str) -> syn::Result<ComponentArgs> {
    syn::parse_str::<ComponentArgs>(args)
}

#[test]
fn parses_the_accessor_name() {
    let args = parse_component_args(r#"name = "ticket_handler""#).expect("should parse");

    assert_eq!(args.name, "ticket_handler");
}

#[test]
fn a_trailing_comma_is_accepted_after_the_name() {
    assert!(parse_component_args(r#"name = "tickets","#).is_ok());
}

#[test]
fn a_missing_name_is_an_error() {
    // The name is what the accessor is called, and there is nothing to fall back
    // on: the type cannot be converted back into it.
    assert!(parse_component_args("").is_err());
}

#[test]
fn an_unknown_component_argument_is_an_error() {
    let error = parse_component_args(r#"name = "tickets", sync"#)
        .expect_err("unknown argument should be rejected");

    assert!(
        error.to_string().contains("sync"),
        "the error should name the argument: {error}"
    );
}

#[test]
fn a_repeated_name_is_an_error() {
    let error = parse_component_args(r#"name = "a", name = "b""#)
        .expect_err("a repeated argument should be rejected");

    assert!(error.to_string().contains("more than once"), "{error}");
}

#[test]
fn a_name_that_cannot_be_an_identifier_is_an_error() {
    // It becomes an accessor ident in generated code, so an unusable one has to be
    // caught here while there is still a span to report it against.
    assert!(parse_component_args(r#"name = "ticket handler""#).is_err());
    assert!(parse_component_args(r#"name = """#).is_err());
    assert!(parse_component_args(r#"name = "   ""#).is_err());
}

// ── split_component_args ─────────────────────────────────────────────────────

fn split(f: ItemFn) -> syn::Result<(Vec<(syn::Ident, syn::Type)>, Punctuated<FnArg, Comma>)> {
    split_component_args(&f.sig.inputs)
}

fn type_of(component: &(syn::Ident, syn::Type)) -> String {
    let ty = &component.1;
    quote!(#ty).to_string()
}

#[test]
fn a_handler_without_components_keeps_every_parameter() {
    let f: ItemFn = parse_quote! { fn foo(id: u32, name: String) {} };
    let (components, request_inputs) = split(f).expect("should split");

    assert!(components.is_empty());
    assert_eq!(request_inputs.len(), 2);
}

#[test]
fn a_component_is_taken_out_of_the_request_parameters() {
    // The point of the split: left in place, `tickets` would be looked up in the
    // parameter map, miss, and answer 400 on every request.
    let f: ItemFn = parse_quote! {
        fn foo(id: u32, #[component] tickets: &TicketHandler) {}
    };
    let (components, request_inputs) = split(f).expect("should split");

    assert_eq!(components.len(), 1);
    assert_eq!(components[0].0.to_string(), "tickets");
    assert_eq!(request_inputs.len(), 1);
}

#[test]
fn a_component_is_reported_as_its_referent_not_its_reference() {
    // `<TicketHandler as ..>` is what the accessor is implemented on, so returning
    // `&TicketHandler` here would make every caller undo the reference.
    let f: ItemFn = parse_quote! { fn foo(#[component] tickets: &TicketHandler) {} };
    let (components, _) = split(f).expect("should split");

    assert_eq!(type_of(&components[0]), "TicketHandler");
}

#[test]
fn components_are_returned_in_declaration_order() {
    let f: ItemFn = parse_quote! {
        fn foo(#[component] tickets: &TicketHandler, id: u32, #[component] users: &UserStore) {}
    };
    let (components, request_inputs) = split(f).expect("should split");

    assert_eq!(components.len(), 2);
    assert_eq!(components[0].0.to_string(), "tickets");
    assert_eq!(components[1].0.to_string(), "users");
    assert_eq!(request_inputs.len(), 1);
}

#[test]
fn an_unmarked_reference_parameter_is_left_alone() {
    // Only the attribute decides. A reference that was not marked stays a request
    // parameter, wrong though it will turn out to be, because guessing here would
    // silently reinterpret a signature the caller wrote deliberately.
    let f: ItemFn = parse_quote! { fn foo(tickets: &TicketHandler) {} };
    let (components, request_inputs) = split(f).expect("should split");

    assert!(components.is_empty());
    assert_eq!(request_inputs.len(), 1);
}

#[test]
fn a_component_taken_by_value_is_an_error() {
    // The instance lives in a static and cannot be moved out of it.
    let f: ItemFn = parse_quote! { fn foo(#[component] tickets: TicketHandler) {} };
    // The `Ok` half is mapped away because syn's types carry no `Debug` unless the
    // `extra-traits` feature is on, and `expect_err` needs one to print.
    let error = split(f)
        .map(|_| ())
        .expect_err("a by-value component should be rejected");

    assert!(error.to_string().contains("shared reference"), "{error}");
}

#[test]
fn a_component_taken_by_mutable_reference_is_an_error() {
    // One request cannot hold exclusive access to state every other request shares.
    let f: ItemFn = parse_quote! { fn foo(#[component] tickets: &mut TicketHandler) {} };
    let error = split(f)
        .map(|_| ())
        .expect_err("a `&mut` component should be rejected");

    assert!(error.to_string().contains("&mut"), "{error}");
}

#[test]
fn a_component_bound_by_a_pattern_is_an_error() {
    // The parameter name is the accessor name, so there has to be exactly one.
    let f: ItemFn = parse_quote! { fn foo(#[component] (a, b): &(u8, u8)) {} };
    let error = split(f)
        .map(|_| ())
        .expect_err("a pattern-bound component should be rejected");

    assert!(error.to_string().contains("plain name"), "{error}");
}

// ── generate_components_retrieval_block ──────────────────────────────────────

fn retrieval_src(f: ItemFn) -> String {
    let (components, _) = split_component_args(&f.sig.inputs).expect("should split");

    // Whitespace between tokens is not meaningful and varies with how the stream
    // was built, so it is removed rather than matched around.
    generate_components_retrieval_block(&components)
        .to_string()
        .replace(' ', "")
}

#[test]
fn a_handler_without_components_generates_nothing() {
    let f: ItemFn = parse_quote! { fn foo(id: u32) {} };

    assert!(retrieval_src(f).is_empty());
}

#[test]
fn a_component_is_bound_from_the_app_context() {
    let f: ItemFn = parse_quote! { fn foo(#[component] tickets: &TicketHandler) {} };
    let src = retrieval_src(f);

    assert!(src.contains("crate::AppContext::tickets()"), "{src}");
}

#[test]
fn a_component_binding_is_annotated_with_the_declared_type() {
    // The annotation is what makes the compiler check that the accessor hands back
    // what the handler asked for, instead of the pair silently disagreeing.
    let f: ItemFn = parse_quote! { fn foo(#[component] tickets: &TicketHandler) {} };
    let src = retrieval_src(f);

    assert!(src.contains("lettickets:&'staticTicketHandler"), "{src}");
}

#[test]
fn every_component_gets_its_own_binding() {
    let f: ItemFn = parse_quote! {
        fn foo(#[component] tickets: &TicketHandler, #[component] users: &UserStore) {}
    };
    let src = retrieval_src(f);

    assert!(src.contains("crate::AppContext::tickets()"), "{src}");
    assert!(src.contains("crate::AppContext::users()"), "{src}");
}

// ── components in a generated handler ────────────────────────────────────────

#[test]
fn a_component_is_not_looked_up_in_the_parameter_map() {
    let f: ItemFn = parse_quote! {
        fn get_ticket(id: u32, #[component] tickets: &TicketHandler) -> String { String::new() }
    };
    let src = handler_src(f, "/tickets/{id}", Method::GET).replace(' ', "");

    assert!(src.contains("crate::AppContext::tickets()"), "{src}");
    assert!(
        !src.contains(r#"map_with_params.get(&"tickets"[..])"#),
        "the component was treated as a request parameter"
    );
}

#[test]
fn a_component_does_not_become_the_request_body() {
    // The body binds to the first parameter that is not a path param. A component
    // declared ahead of the real body parameter used to take its place.
    let f: ItemFn = parse_quote! {
        fn create_ticket(#[component] tickets: &TicketHandler, body: String) -> String { body }
    };
    let src = handler_src(f, "/tickets", Method::POST).replace(' ', "");

    assert!(
        src.contains(r#"map_with_params.insert("body".to_string(),body)"#),
        "{src}"
    );
}
