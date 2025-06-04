use std::collections::HashMap;

pub fn extract_params(request: &str) -> HashMap<String, String> {
    let query = request.splitn(2, '?').nth(1).unwrap_or("");
    query.split('&').filter_map(|pair| {
        let mut iterator = pair.splitn(2, '=');
        let key = iterator.next()?;
        let value = iterator.next()?;

        if key == "" {
            None
        } else {
            Some((key.to_string(), value.to_string()))
        }
    }).collect()
}