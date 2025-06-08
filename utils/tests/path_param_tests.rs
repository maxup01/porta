use utils::request::path_param::{extract_path_param_names_from_path, extract_path_params};

const PATH_WITH_SOME_PATH_PARAMS: &str = "/api/v1/{userId}/posts/{postId}";
const PATH_WITH_SOME_PATH_PARAMS_FIELD_WITH_VALUES: &str = "/api/v1/10/posts/70";
const PATH_WITH_NO_PATH_PARAMS: &str = "/api/v1/posts";

#[test]
pub fn should_return_empty_vec_when_no_path_params() {
    let path_params = extract_path_param_names_from_path(PATH_WITH_NO_PATH_PARAMS);
    assert!(path_params.is_empty());
}

#[test]
pub fn should_extract_path_param_names_from_path() {
    let path_params = extract_path_param_names_from_path(PATH_WITH_SOME_PATH_PARAMS);
    assert_eq!(path_params, vec![String::from("userId"), String::from("postId")]);
}

#[test]
pub fn should_return_empty_vec_when_path_has_no_path_params() {

    let path_params = extract_path_params(PATH_WITH_NO_PATH_PARAMS, PATH_WITH_NO_PATH_PARAMS);
    let second_path_params = 
        extract_path_params(PATH_WITH_SOME_PATH_PARAMS_FIELD_WITH_VALUES, PATH_WITH_SOME_PATH_PARAMS);
    assert!(path_params.is_empty());
    assert!(second_path_params.is_empty());
}

#[test]
pub fn should_extract_path_param_values_from_path_that_has_params() {

    let path_params = extract_path_params(PATH_WITH_SOME_PATH_PARAMS, PATH_WITH_SOME_PATH_PARAMS_FIELD_WITH_VALUES);
    assert_eq!(path_params.len(), 2);
    assert_eq!(path_params.get("userId").unwrap().as_str(), "10");
    assert_eq!(path_params.get("postId").unwrap().as_str(), "70");
}

