pub fn extract_path_param_names_from_path(path: &str) -> Vec<String> {
    let param_names: Vec<String> = path.split('/').filter(|s| s.starts_with('{') && s.ends_with('}'))
        .map(|s| (&s[1..s.len() - 1]).to_string()).collect();

    param_names
}