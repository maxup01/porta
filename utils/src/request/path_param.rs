pub fn extract_path_param_names_from_path(path: &str) -> Vec<String> {
    let param_names: Vec<String> = path.split('/').filter(|s| s.starts_with('{') && s.ends_with('}'))
        .map(|s| (&s[1..s.len() - 1]).to_string()).collect();

    param_names
}

pub fn extract_path_param_values_from_path(route_path: &str, path: &str) -> Vec<String> {
    let route_path_parts: Vec<&str> = route_path.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    let mut param_values_as_json: Vec<String> = vec![];

    for (route_path_part, path_part) in route_path_parts.iter().zip(path_parts.iter()) {
        if !route_path_part.starts_with('{') || !route_path_part.ends_with('}') {
            continue;
        }
        else if route_path_part != path_part {
            if path_part.starts_with("{") && path_part.ends_with("}") {
                param_values_as_json.push((&path_part[0..path_part.len()]).to_string());
            } else {
                param_values_as_json.push(format!("\"{}\"", (&path_part[0..path_part.len()])));
            }
        }
    }

    param_values_as_json
}