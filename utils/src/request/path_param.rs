use std::{collections::HashMap, str::Split};

pub fn extract_path_param_names_from_path(path: &str) -> impl Iterator<Item = String> {
    path.split('/').filter_map(|s| {
        if s.starts_with('{') && s.ends_with('}') {
            Some((s[1..s.len() - 1]).to_string())
        } else {
            None
        }
    })
}

pub fn extract_path_params(route_path: &str, path: &str) -> HashMap<String, String> {
    let route_path_parts: Split<'_, _> = route_path.split('/');
    let path_parts: Split<'_, _> = path.split('/');

    let mut param_values_as_json: HashMap<String, String> = HashMap::new();

    for (route_path_part, path_part) in route_path_parts.zip(path_parts) {
        if !route_path_part.starts_with('{') || !route_path_part.ends_with('}') {
            continue;
        } else if route_path_part != path_part {
            param_values_as_json.insert(
                (route_path_part[1..(route_path_part.len() - 1)]).to_string(),
                (path_part[0..path_part.len()]).to_string(),
            );
        }
    }

    param_values_as_json
}

#[cfg(test)]
#[path = "path_param_tests.rs"]
mod tests;
