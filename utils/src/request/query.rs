use std::collections::HashMap;

/// Extracts query string parameters from an HTTP request line or URL.
///
/// Splits the input on the first `?`, then parses the query string into
/// key-value pairs separated by `&`, with each pair split on the first `=`.
/// Pairs with an empty key are silently discarded.
///
/// # Arguments
///
/// * `request` - A raw HTTP request line or URL string, e.g.
///   `"GET /search?q=rust&page=2 HTTP/1.1"` or `"/search?q=rust&page=2"`
///
/// # Returns
///
/// - `Some(HashMap)` containing all parsed key-value pairs if a `?` is present.
/// - `None` if the input contains no `?`, indicating no query string exists.
///
/// Note that `Some` with an empty map is possible if the query string contains
/// only malformed or empty-key pairs (e.g. `"/?=value&=other"`).
///
/// # Examples
///
/// ```
/// use utils::request::query::extract_params;
///
/// let params = extract_params("/search?q=rust&page=2").unwrap();
/// assert_eq!(params["q"], "rust");
/// assert_eq!(params["page"], "2");
///
/// // No query string returns None
/// assert!(extract_params("/search").is_none());
///
/// // Empty-key pairs are discarded
/// let params = extract_params("/path?=value&key=hello").unwrap();
/// assert_eq!(params.len(), 1);
/// assert_eq!(params["key"], "hello");
/// ```
///
/// # Caveats
///
/// - Values are taken verbatim; percent-decoding is left to the caller.
/// - Duplicate keys are silently collapsed — the last occurrence wins.
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
