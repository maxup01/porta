use utils::request::route::{Method, get_route_function, register_route};

const PATH_FOR_GET_METHOD: &str = "/get-mapping";
const PATH_FOR_POST_METHOD: &str = "/post-mapping";
const PATH_FOR_PATCH_METHOD: &str = "/patch-mapping";
const PATH_FOR_DELETE_METHOD: &str = "/delete-mapping";
const PATH_THAT_NOT_EXIST: &str = "/mapping-that-not-exists";

#[test]
fn test_register_and_get_route() {

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