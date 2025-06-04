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