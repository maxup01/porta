use std::collections::HashMap;

pub fn extract_params(request: &str) -> Option<HashMap<String, String>> {
    match request.split_once('?') {
        Some((_path, query)) => {
            let params = query
                .split('&')
                .filter_map(|pair| {
                    let (key, value) = pair.split_once('=')?;

                    if !key.is_empty() {
                        Some((key.to_string(), value.to_string()))
                    } else {
                        None
                    }
                })
                .collect();

            Some(params)
        }
        None => None,
    }
}
