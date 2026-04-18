use error::Error;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{LazyLock, Mutex},
    vec::Vec,
};

type RouteHandler = fn(&str) -> String;

/// Lazily initialized, thread-safe map of GET route paths to their handler functions.
/// Populated at startup via route registration and consulted on each incoming GET request.
static GET_ROUTES: LazyLock<Mutex<HashMap<String, RouteHandler>>> = LazyLock::new(|| {
    let m: HashMap<String, fn(&str) -> String> = HashMap::new();
    Mutex::new(m)
});

/// Lazily initialized, thread-safe map of POST route paths to their handler functions.
/// Populated at startup via route registration and consulted on each incoming POST request.
static POST_ROUTES: LazyLock<Mutex<HashMap<String, RouteHandler>>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

/// Lazily initialized, thread-safe map of PATCH route paths to their handler functions.
/// Populated at startup via route registration and consulted on each incoming PATCH request.
static PATCH_ROUTES: LazyLock<Mutex<HashMap<String, RouteHandler>>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

/// Lazily initialized, thread-safe map of DELETE route paths to their handler functions.
/// Populated at startup via route registration and consulted on each incoming DELETE request.
static DELETE_ROUTES: LazyLock<Mutex<HashMap<String, RouteHandler>>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

/// Lazily initialized, thread-safe list of all registered route paths across all HTTP methods.
/// Used for introspection, validation, or generating route listings (e.g. a health/debug endpoint).
static PATHS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| {
    let m = Vec::new();
    Mutex::new(m)
});

/// Enum representing http methods
pub enum Method {
    GET,
    POST,
    PATCH,
    DELETE,
}

/// Parses a string slice into an HTTP [`Method`].
///
/// Matching is case-insensitive, so `"get"`, `"GET"`, and `"Get"` all produce [`Method::GET`].
///
/// # Errors
///
/// Returns [`Error::InvalidData`] if the string does not correspond to a supported HTTP method.
///
/// # Examples
///
/// ```
/// let method: Method = "POST".parse()?;
/// assert_eq!(method, Method::POST);
///
/// let method: Method = "delete".parse()?;
/// assert_eq!(method, Method::DELETE);
///
/// assert!("CONNECT".parse::<Method>().is_err());
/// ```
impl FromStr for Method {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PATCH" => Ok(Method::PATCH),
            "DELETE" => Ok(Method::DELETE),
            _ => Err(Error::InvalidData(
                "Passed string that doesn't match any http method",
            )),
        }
    }
}

/// Returns a reference to the route table for the given HTTP method.
///
/// The returned reference points to one of the global [`LazyLock`]-wrapped route maps,
/// giving the caller direct access to lock and mutate it — useful for bulk operations
/// such as registering multiple routes or inspecting the full handler map for a method.
///
/// For single-route lookups, prefer [`get_route_function`] instead.
///
/// # Examples
///
/// ```
/// let routes = get_route_handlers_by_method(Method::GET);
/// let mut map = routes.lock().unwrap();
/// map.insert("/health".to_string(), health_handler);
/// ```
pub fn get_route_handlers_by_method(
    method: Method,
) -> &'static LazyLock<Mutex<HashMap<String, RouteHandler>>> {
    match method {
        Method::GET => &GET_ROUTES,
        Method::POST => &POST_ROUTES,
        Method::PATCH => &PATCH_ROUTES,
        Method::DELETE => &DELETE_ROUTES,
    }
}

/// Looks up the handler function registered for the given URL and HTTP method.
///
/// The URL is normalized before lookup: any query string (everything from `?` onward)
/// is stripped so that `/users?id=1` resolves the same route as `/users`.
///
/// Returns `Ok(Some(handler))` if a matching route is found, `Ok(None)` if the path
/// is not registered for that method, or an [`Error`] if the route table lock is poisoned.
///
/// # Examples
///
/// ```
/// let handler = get_route_function("/users?id=42", Method::GET).unwrap();
/// assert!(handler.is_some());
///
/// let handler = get_route_function("/nonexistent", Method::DELETE).unwrap();
/// assert!(handler.is_none());
/// ```
pub fn get_route_function(url: &str, method: Method) -> Result<Option<RouteHandler>, Error> {
    let path = match url.split_once('?') {
        Some((path, _)) => path,
        None => url,
    };

    let route_handlers = get_route_handlers_by_method(method).lock().unwrap();

    Ok(route_handlers.get(path).copied())
}

/// Registers a handler function for the given HTTP method and path.
///
/// The path is added to the global [`PATHS`] list and the handler is inserted into
/// the corresponding method's route table, making it available for dispatch on
/// incoming requests.
///
/// # Panics
///
/// Panics if either the method's route table mutex or the [`PATHS`] mutex is poisoned.
///
/// # Examples
///
/// ```
/// fn hello_handler(body: &str) -> String {
///     "Hello, world!".to_string()
/// }
///
/// register_route(Method::GET, "/hello", hello_handler);
/// ```
pub fn register_route(method: Method, path: &str, function: RouteHandler) {
    let mut route_handlers = get_route_handlers_by_method(method).lock().unwrap();

    PATHS.lock().unwrap().push(path.to_string());

    route_handlers.insert(path.to_string(), function);
}

/// Extracts the request target (path and optional query string) from a raw HTTP request line.
///
/// Expects the standard HTTP request line format: `METHOD PATH HTTP/VERSION`, for example
/// `GET /users?id=1 HTTP/1.1`. The second whitespace-delimited token is returned as the path.
///
/// # Errors
///
/// Returns [`Error::InvalidData`] if the request line does not contain at least two
/// whitespace-delimited tokens.
///
/// # Examples
///
/// ```
/// let path = extract_path_from_request("GET /users?id=1 HTTP/1.1")?;
/// assert_eq!(path, "/users?id=1");
///
/// assert!(extract_path_from_request("MALFORMED").is_err());
/// ```
pub fn extract_path_from_request(request: &str) -> Result<String, Error> {
    let path = request
        .splitn(3, ' ')
        .nth(1)
        .ok_or(Error::InvalidData("Invalid request lines"))?;

    Ok(path.to_string())
}

/// Returns `true` if the route segment is a path parameter placeholder, e.g. `{id}`.
fn path_param_segment(route_segment: &str) -> bool {
    route_segment.starts_with('{') && route_segment.ends_with('}')
}

/// Returns `true` if the route segment is a literal path segment, e.g. `users`.
fn fixed_path_segment(route_segment: &str) -> bool {
    !route_segment.starts_with('{') || !route_segment.ends_with('}')
}

/// Returns `true` if a concrete request path matches a registered route path pattern.
///
/// Paths are compared segment by segment after splitting on `/`. A route segment wrapped
/// in curly braces (e.g. `{id}`) matches any value in the corresponding position, while
/// a fixed segment must match exactly. Paths with a different number of segments never match.
///
/// # Examples
///
/// ```
/// assert!(is_path_matching_route_path("/users/42", "/users/{id}"));
/// assert!(is_path_matching_route_path("/users/list", "/users/list"));
///
/// assert!(!is_path_matching_route_path("/users/42/posts", "/users/{id}"));
/// assert!(!is_path_matching_route_path("/posts/42", "/users/{id}"));
/// ```
pub fn is_path_matching_route_path(path: &str, route_path: &str) -> bool {
    let route_path_segments: Vec<&str> = route_path.split('/').collect();
    let path_segments: Vec<&str> = path.split('/').collect();

    if route_path_segments.len() != path_segments.len() {
        return false;
    }

    for (route_path_segment, path_segment) in route_path_segments.iter().zip(path_segments.iter()) {
        if !(path_param_segment(route_path_segment)
            || (fixed_path_segment(route_path_segment) && route_path_segment == path_segment))
        {
            return false;
        }
    }

    true
}

/// Finds the first registered route path that matches the given request path.
///
/// Iterates over all paths in [`PATHS`] and returns the first one that matches
/// according to [`is_path_matching_route_path`]. Returns `None` if no registered
/// route matches.
///
/// # Panics
///
/// Panics if the [`PATHS`] mutex is poisoned.
///
/// # Examples
///
/// ```
/// register_route(Method::GET, "/users/{id}", handler);
///
/// assert_eq!(get_matching_route_path("/users/42"), Some("/users/{id}".to_string()));
/// assert_eq!(get_matching_route_path("/nonexistent"), None);
/// ```
pub fn get_matching_route_path(path: &str) -> Option<String> {
    let route_paths = PATHS.lock().unwrap();

    for route_path in route_paths.iter() {
        if is_path_matching_route_path(path, route_path) {
            return Some(route_path.to_string());
        }
    }

    None
}

/// Extracts and parses the HTTP method from a raw HTTP request line.
///
/// Expects the standard HTTP request line format: `METHOD PATH HTTP/VERSION`, for example
/// `POST /users HTTP/1.1`. The first whitespace-delimited token is parsed into a [`Method`].
///
/// # Errors
///
/// Returns [`Error::InvalidData`] if the request line contains no whitespace, or if the
/// first token does not correspond to a supported HTTP method (see [`Method::from_str`]).
///
/// # Examples
///
/// ```
/// let method = extract_method_from_request("POST /users HTTP/1.1")?;
/// assert_eq!(method, Method::POST);
///
/// assert!(extract_method_from_request("MALFORMED").is_err());
/// assert!(extract_method_from_request("CONNECT /users HTTP/1.1").is_err());
/// ```
pub fn extract_method_from_request(request: &str) -> Result<Method, Error> {
    let (method, _) = request
        .split_once(' ')
        .ok_or(Error::InvalidData("Invalid request lines"))?;

    Method::from_str(method)
}
