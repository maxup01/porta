use utils::request::path_param::extract_path_param_names_from_path;

const PATH_WITH_SOME_PATH_PARAMS: &str = "/api/v1/{userId}/posts/{postId}";
const PATH_WITH_NO_PATH_PARAMS: &str = "/api/v1/posts";

#[test]
pub fn should_return_empty_vec_when_no_path_params() {
    let path_params = extract_path_param_names_from_path(PATH_WITH_NO_PATH_PARAMS);
    assert!(path_params.is_empty());
}

#[test]
pub fn should_extract_path_param_names_from_path() {
    let path_params = extract_path_param_names_from_path(PATH_WITH_SOME_PATH_PARAMS);
    assert_eq!(path_params, vec!["userId".to_string(), "postId".to_string()]);
}