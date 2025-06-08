use utils::request::route::{Method, get_route_function, register_route, extract_path_from_request,
     is_path_matching_route_path, get_matching_route_path};

const PATH_FOR_GET_METHOD: &str = "/get-mapping";
const PATH_FOR_POST_METHOD: &str = "/post-mapping";
const PATH_FOR_PATCH_METHOD: &str = "/patch-mapping";
const PATH_FOR_DELETE_METHOD: &str = "/delete-mapping";
const PATH_THAT_NOT_EXIST: &str = "/mapping-that-not-exists";
const REQUEST_WITH_PATH: &str = "GET /mapping/some";
const PATH: &str = "/mapping/some";
const RANDOM_STR_WITHOUT_SPACE: &str = "hbdabhiafhahifinhafnih";

const USER_ROUTE: &str = "/user/{id}";
const COMMENT_ROUTE: &str = "/post/{post_id}/comment/{comment_id}";
const ABOUT_ROUTE: &str = "/about";
const USER_PATH: &str = "/user/42";
const COMMENT_PATH: &str = "/post/1/comment/2";
const ABOUT_PATH: &str = "/about";
const NOT_FOUND_PATH: &str = "/notfound";

#[test]
pub fn test_method_from_str() {
    let get_method = Method::from_str("GET");
    let post_method = Method::from_str("POST");
    let patch_method = Method::from_str("PATCH");
    let delete_method = Method::from_str("DELETE");
    let not_existing_method = Method::from_str("NOT_EXISTING");

    assert!(get_method.is_some());
    assert!(post_method.is_some());
    assert!(patch_method.is_some());
    assert!(delete_method.is_some());
    assert!(not_existing_method.is_none());
}

#[test]
pub fn test_register_and_get_route() {

    let random_function = |param: &str| {param.to_string()};

    register_route(Method::GET, PATH_FOR_GET_METHOD, random_function);
    register_route(Method::POST, PATH_FOR_POST_METHOD, random_function);
    register_route(Method::PATCH, PATH_FOR_PATCH_METHOD, random_function);
    register_route(Method::DELETE, PATH_FOR_DELETE_METHOD, random_function);

    let func_opt = get_route_function(PATH_FOR_GET_METHOD, Method::GET);
    assert!(func_opt.is_some());
        
    let func_opt = get_route_function(PATH_FOR_POST_METHOD, Method::POST);
    assert!(func_opt.is_some());

    let func_opt = get_route_function(PATH_FOR_PATCH_METHOD, Method::PATCH);
    assert!(func_opt.is_some());

    let func_opt = get_route_function(PATH_FOR_DELETE_METHOD, Method::DELETE);
    assert!(func_opt.is_some());

    let func_opt = get_route_function(PATH_FOR_POST_METHOD, Method::GET);
    assert!(func_opt.is_none());

    let func_opt = get_route_function(PATH_THAT_NOT_EXIST, Method::POST);
    assert!(func_opt.is_none());

    let func_opt = get_route_function(PATH_THAT_NOT_EXIST, Method::PATCH);
    assert!(func_opt.is_none());

    let func_opt = get_route_function(PATH_THAT_NOT_EXIST, Method::DELETE);
    assert!(func_opt.is_none());

}

#[test]
pub fn test_extract_path_from_request() {

    let path = extract_path_from_request(REQUEST_WITH_PATH);
    let none = extract_path_from_request(RANDOM_STR_WITHOUT_SPACE);

    assert!(path.is_some());
    assert_eq!(path.unwrap(), PATH);
    assert!(none.is_none());
}

#[test]
pub fn is_path_matching_route_path_test() {
    let path = "/mapping/same";
    let route_path = r#"/mapping/{param}"#;
    let not_matching_path = "/mapping/other}";
    let second_not_matching_path = "/mapping";

    assert!(is_path_matching_route_path(route_path, path));
    assert!(!is_path_matching_route_path(route_path, not_matching_path));
    assert!(!is_path_matching_route_path(route_path, second_not_matching_path));
}

#[test]
pub fn test_get_matching_route_path() {

    let dummy_handler = |param: &str| {param.to_string()};

    register_route(Method::GET, USER_ROUTE, dummy_handler);
    register_route(Method::GET, COMMENT_ROUTE, dummy_handler);
    register_route(Method::GET, ABOUT_ROUTE, dummy_handler);

    assert_eq!(get_matching_route_path(USER_PATH).unwrap(), USER_ROUTE.to_string());
    assert_eq!(get_matching_route_path(COMMENT_PATH).unwrap(), COMMENT_ROUTE.to_string());
    assert_eq!(get_matching_route_path(ABOUT_PATH).unwrap(), ABOUT_ROUTE.to_string());
    assert_eq!(get_matching_route_path(NOT_FOUND_PATH), None);
}