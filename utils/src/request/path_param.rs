use error::Error;
use std::collections::HashMap;

/// Extracts path parameter names from a URL path template.
///
/// Parses a path string and yields the names of all path parameters,
/// which are segments enclosed in curly braces (e.g. `{id}`).
///
/// # Arguments
///
/// * `path` - A URL path template string, e.g. `"/users/{id}/posts/{post_id}"`
///
/// # Returns
///
/// An iterator over [`String`]s, each being a parameter name with the
/// surrounding braces stripped.
///
/// # Examples
///
/// ```
/// use utils::request::path_param::extract_path_param_names_from_path;
///
/// let names: Vec<String> = extract_path_param_names_from_path("/users/{id}/posts/{post_id}").collect();
/// assert_eq!(names, vec!["id", "post_id"]);
///
/// // Returns an empty iterator if no parameters are present
/// let empty: Vec<String> = extract_path_param_names_from_path("/users/all").collect();
/// assert!(empty.is_empty());
/// ```
pub fn extract_path_param_names_from_path(path: &str) -> impl Iterator<Item = String> {
    path.split('/').filter_map(|s| {
        if s.starts_with('{') && s.ends_with('}') {
            Some((s[1..s.len() - 1]).to_string())
        } else {
            None
        }
    })
}

/// Extracts path parameter values by matching a route template against an actual request path.
///
/// Zips the segments of `route_path` and `path` together, and for every segment
/// in `route_path` that is a parameter placeholder (i.e. wrapped in `{}`), maps
/// the parameter name to the corresponding segment value from `path`.
///
/// # Arguments
///
/// * `route_path` - A URL route template containing parameter placeholders,
///   e.g. `"/users/{id}/posts/{post_id}"`
/// * `path` - The actual request path to extract values from,
///   e.g. `"/users/42/posts/7"`
///
/// # Returns
///
/// A [`HashMap`] mapping each parameter name (with braces stripped) to its
/// corresponding value from `path`.
///
/// # Errors
///
/// Returns [`Error::InvalidData`] if `route_path` and `path` have a different
/// number of segments, indicating that `path` cannot be a valid match for the
/// given route template.
///
/// # Examples
///
/// ```
/// use utils::request::path_param::extract_path_params;
///
/// let params = extract_path_params("/users/{id}/posts/{post_id}", "/users/42/posts/7").unwrap();
/// assert_eq!(params["id"], "42");
/// assert_eq!(params["post_id"], "7");
///
/// // Static segments are ignored
/// let params = extract_path_params("/users/all", "/users/all").unwrap();
/// assert!(params.is_empty());
///
/// // Mismatched segment counts return an error
/// assert!(extract_path_params("/users/{id}", "/users/42/posts").is_err());
/// ```
///
/// # Caveats
///
/// Parameter values are taken verbatim; percent-decoding or any other
/// normalisation is left to the caller.
pub fn extract_path_params(route_path: &str, path: &str) -> Result<HashMap<String, String>, Error> {
    let route_path_parts: Vec<&str> = route_path.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    if route_path_parts.len() != path_parts.len() {
        return Err(Error::InvalidData(
            "Passed path that doesn't match route's path",
        ));
    }

    let mut param_values_as_json: HashMap<String, String> = HashMap::new();

    for (route_path_part, path_part) in route_path_parts.into_iter().zip(path_parts.into_iter()) {
        if !route_path_part.starts_with('{') || !route_path_part.ends_with('}') {
            continue;
        }

        param_values_as_json.insert(
            route_path_part[1..(route_path_part.len() - 1)].to_string(),
            path_part[0..path_part.len()].to_string(),
        );
    }

    Ok(param_values_as_json)
}

#[cfg(test)]
#[path = "path_param_tests.rs"]
mod tests;
