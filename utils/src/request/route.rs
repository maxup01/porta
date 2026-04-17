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

pub enum Method {
    GET,
    POST,
    PATCH,
    DELETE,
}

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

pub fn get_route_function(request: &str, method: Method) -> Option<fn(&str) -> String> {
    let path = request.splitn(2, '?').next().unwrap();

    let routes = match method {
        Method::GET => GET_ROUTES.lock().unwrap(),
        Method::POST => POST_ROUTES.lock().unwrap(),
        Method::PATCH => PATCH_ROUTES.lock().unwrap(),
        Method::DELETE => DELETE_ROUTES.lock().unwrap(),
    };

    routes.get(path).copied()
}

pub fn register_route(method: Method, path: &str, function: fn(&str) -> String) {
    let mut map_with_routes = match method {
        Method::GET => GET_ROUTES.lock().unwrap(),
        Method::POST => POST_ROUTES.lock().unwrap(),
        Method::PATCH => PATCH_ROUTES.lock().unwrap(),
        Method::DELETE => DELETE_ROUTES.lock().unwrap(),
    };

    PATHS.lock().unwrap().push(path.to_string());

    map_with_routes.insert(path.to_string(), function);
}

pub fn extract_path_from_request(request: &str) -> Option<String> {
    let mut parts = request.split(' ');
    parts.next()?;

    let path = parts.next()?;

    Some(path.to_string())
}

pub fn is_path_matching_route_path(route_path: &str, path: &str) -> bool {
    let route_path_parts: Vec<&str> = route_path.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    if route_path_parts.len() != path_parts.len() {
        return false;
    }

    for (route_path_part, path_part) in route_path_parts.iter().zip(path_parts.iter()) {
        if (route_path_part != path_part
            && !route_path_part.starts_with('{')
            && !route_path_part.ends_with('}'))
            || (route_path_part.starts_with('{')
                && route_path_part.ends_with('}')
                && !(path_part.starts_with('{') && path_part.ends_with('}'))
                && (path_part.starts_with('{') || path_part.ends_with('}')))
        {
            return false;
        }
    }

    true
}

pub fn get_matching_route_path(path: &str) -> Option<String> {
    let paths = PATHS.lock().unwrap();
    for route_path in paths.iter() {
        if is_path_matching_route_path(route_path, path) {
            return Some(route_path.clone());
        }
    }
    None
}

pub fn extract_method_from_request(request: &str) -> Result<Method, Error> {
    let (method, _) = request
        .split_once(' ')
        .ok_or(Error::InvalidData("Invalid request lines"))?;

    Method::from_str(method)
}
