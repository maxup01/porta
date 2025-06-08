use std::collections::HashMap;
use std::sync::{Mutex, LazyLock};

static GET_ROUTES: LazyLock<Mutex<HashMap<String, fn(&str) -> String>>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

static POST_ROUTES: LazyLock<Mutex<HashMap<String, fn(&str) -> String>>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

static PATCH_ROUTES: LazyLock<Mutex<HashMap<String, fn(&str) -> String>>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

static DELETE_ROUTES: LazyLock<Mutex<HashMap<String, fn(&str) -> String>>> = LazyLock::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

pub enum Method {
    GET,
    POST,
    PATCH,
    DELETE
}

impl Method {
    pub fn from_str(method: &str) -> Option<Method> {
        match method.to_uppercase().as_str() {
            "GET" => Some(Method::GET),
            "POST" => Some(Method::POST),
            "PATCH" => Some(Method::PATCH),
            "DELETE" => Some(Method::DELETE),
            _ => None,
        }
    }
}

pub fn get_route_function(request: &str, method: Method) -> Option<fn(&str) -> String> {
    let path = request.splitn(2, '?').next().unwrap();

    let routes = match method {
        Method::GET => {
            GET_ROUTES.lock().unwrap()
        }
        Method::POST => {
            POST_ROUTES.lock().unwrap()
        }
        Method::PATCH => {
            PATCH_ROUTES.lock().unwrap()
        }
        Method::DELETE => {
            DELETE_ROUTES.lock().unwrap()
        }
    };

    routes.get(path).copied()
}

pub fn register_route(method: Method, path: &str, function: fn(&str) -> String) {

    let mut map_with_routes = match method {
        Method::GET => {
            GET_ROUTES.lock().unwrap()
        }
        Method::POST => {
            POST_ROUTES.lock().unwrap()
        }
        Method::PATCH => {
            PATCH_ROUTES.lock().unwrap()
        }
        Method::DELETE => {
            DELETE_ROUTES.lock().unwrap()
        }
    };

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
        if (route_path_part != path_part && !route_path_part.starts_with('{') && !route_path_part.ends_with('}')) 
            || (route_path_part.starts_with('{') && route_path_part.ends_with('}') 
            && !(path_part.starts_with('{') && path_part.ends_with('}')) 
            && (path_part.starts_with('{') || path_part.ends_with('}'))) {
            return false;
        }
    }

    true
}