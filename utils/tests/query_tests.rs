use utils::request::query;

const MAPPING_WITH_ZERO_PARAMS: &str = "/some-mapping";
const MAPPING_WITH_THREE_PARAMS_AND_ONE_CORRECT: &str = "/some-mapping?name=maks&some&=something";
const MAPPING_WITH_TWO_CORRECT_PARAMS: &str = "/some-mapping?name=maks&value=something";
const FIRST_PARAM_NAME: &str = "name";
const FIRST_PARAM_VALUE: &str = "maks";
const SECOND_PARAM_NAME: &str = "value";
const SECOND_PARAM_VALUE: &str = "something";

#[test]
pub fn should_create_hash_map_with_zero_pairs() {
    let map_with_zero_pairs = query::extract_params(MAPPING_WITH_ZERO_PARAMS);

    assert_eq!(map_with_zero_pairs.len(), 0);
}

#[test]
pub fn should_create_hash_map_with_one_element() {
    let map_with_one_pair = query::extract_params(MAPPING_WITH_THREE_PARAMS_AND_ONE_CORRECT);

    assert_eq!(map_with_one_pair.len(), 1);
    assert_eq!(map_with_one_pair.get(FIRST_PARAM_NAME).unwrap(), FIRST_PARAM_VALUE);
}

#[test]
pub fn should_create_hash_map_with_two_pairs() {
    let map_with_two_pairs = query::extract_params(MAPPING_WITH_TWO_CORRECT_PARAMS);

    assert_eq!(map_with_two_pairs.len(), 2);
    assert_eq!(map_with_two_pairs.get(FIRST_PARAM_NAME).unwrap(), FIRST_PARAM_VALUE);
    assert_eq!(map_with_two_pairs.get(SECOND_PARAM_NAME).unwrap(), SECOND_PARAM_VALUE);
}