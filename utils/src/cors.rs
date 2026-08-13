//! Cross-origin resource sharing: deciding which foreign origins a browser may
//! expose this server's answers to, and saying so in headers.
//!
//! CORS is enforced by the browser, not by the server. What the server controls
//! is the permission slip: a response without the right headers is fetched
//! successfully and then withheld from the calling script, which is why a
//! misconfigured CORS setup reads as an opaque network failure rather than as a
//! status code. The two shapes of answer this module builds are the whole of
//! that permission slip:
//!
//! - the **preflight** reply to an `OPTIONS` request the browser sends on its own
//!   initiative, before a request that is not [simple] — anything with a
//!   `Content-Type: application/json` body, for instance, which is every JSON
//!   API call a page makes;
//! - the **actual-response** headers attached to the real answer afterwards.
//!
//! Both are needed. A server that answers the preflight but omits the headers on
//! the response that follows fails on the second leg, and one that does the
//! reverse never gets asked for the first.
//!
//! [simple]: https://developer.mozilla.org/docs/Glossary/CORS-safelisted_request_header

use crate::request::header::get_header;
use crate::request::route::Method;

/// How long a browser may cache a preflight result, in seconds.
///
/// Every non-simple cross-origin request costs two round trips without this, and
/// this server closes the connection after each one, so the preflight is a full
/// connection setup of its own. Ten minutes is long enough to make repeated calls
/// cheap and short enough that a route table change is picked up within a coffee
/// break. Chromium caps this at 2 hours and Firefox at 24, so a larger value would
/// be silently trimmed rather than honoured.
const PREFLIGHT_MAX_AGE_SECONDS: u32 = 600;

/// Which origins are permitted.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AllowedOrigins {
    /// Any origin at all, configured as `"*"`.
    Any,

    /// Exactly these origins, compared in full — scheme, host and port.
    Only(Vec<String>),
}

/// The CORS policy a server was configured with.
///
/// Construct with [`CorsConfig::new`], or [`CorsConfig::disabled`] for the default
/// of no CORS at all. A disabled config adds no headers to anything, which leaves
/// the server exactly as it behaved before CORS existed: usable by any non-browser
/// client, and by a page served from the same origin.
///
/// # Examples
///
/// ```
/// use utils::cors::CorsConfig;
///
/// let cors = CorsConfig::new("http://localhost:1420", false, None);
/// let request = "GET / HTTP/1.1\r\nOrigin: http://localhost:1420\r\n\r\n";
///
/// let headers = cors.response_headers(request);
/// assert_eq!(headers[0].0, "Access-Control-Allow-Origin");
/// assert_eq!(headers[0].1, "http://localhost:1420");
///
/// // An origin outside the list gets nothing, and the browser withholds the body.
/// let stranger = "GET / HTTP/1.1\r\nOrigin: http://evil.test\r\n\r\n";
/// assert!(cors.response_headers(stranger).is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// `None` disables CORS entirely — no header is ever emitted.
    allowed_origins: Option<AllowedOrigins>,

    /// Whether the browser may send cookies and HTTP authentication with the
    /// request, and read the response afterwards.
    allow_credentials: bool,

    /// The value for `Access-Control-Allow-Headers`, or `None` to echo whatever
    /// the preflight asked for.
    allow_headers: Option<String>,
}

impl CorsConfig {
    /// A policy that permits nothing and emits no headers.
    pub fn disabled() -> CorsConfig {
        CorsConfig {
            allowed_origins: None,
            allow_credentials: false,
            allow_headers: None,
        }
    }

    /// Builds a policy from the values `#[http_server]` takes as attribute
    /// arguments.
    ///
    /// # Parameters
    ///
    /// - `allowed_origins` — a comma-separated list of origins (`"http://a.test,
    ///   https://b.test"`), or `"*"` for any origin. Entries are trimmed; an empty
    ///   or all-whitespace list produces a [`disabled`](CorsConfig::disabled)
    ///   policy, since permitting nothing is what an empty allow-list means.
    /// - `allow_credentials` — whether to send `Access-Control-Allow-Credentials:
    ///   true`, which a browser requires before it will attach cookies to a
    ///   cross-origin request or let a script read the response to one.
    /// - `allow_headers` — the value for `Access-Control-Allow-Headers`, or `None`
    ///   to echo the preflight's `Access-Control-Request-Headers` back. Echoing
    ///   permits whatever was asked for, which is the permissive default; naming
    ///   the headers explicitly is the restrictive one.
    ///
    /// # `"*"` and credentials
    ///
    /// A browser rejects the literal `*` on a credentialed request (Fetch
    /// §3.2.4). When `allow_credentials` is set, the requesting origin is echoed
    /// back instead — the only form that works — so `"*"` with credentials means
    /// "reflect any origin" rather than failing every request. That reflection is
    /// as permissive as it sounds, and pairs a wide-open allow-list with the
    /// ability to ride the user's cookies; name your origins if the server is
    /// reachable by anything you do not control.
    ///
    /// # Examples
    ///
    /// ```
    /// use utils::cors::CorsConfig;
    ///
    /// let cors = CorsConfig::new("http://a.test, http://b.test", false, None);
    /// assert!(cors.is_enabled());
    ///
    /// assert!(!CorsConfig::new("", false, None).is_enabled());
    /// ```
    pub fn new(
        allowed_origins: &str,
        allow_credentials: bool,
        allow_headers: Option<&str>,
    ) -> CorsConfig {
        let allowed_origins = if allowed_origins.trim() == "*" {
            Some(AllowedOrigins::Any)
        } else {
            let origins: Vec<String> = allowed_origins
                .split(',')
                .map(|origin| origin.trim().to_string())
                .filter(|origin| !origin.is_empty())
                .collect();

            if origins.is_empty() {
                None
            } else {
                Some(AllowedOrigins::Only(origins))
            }
        };

        CorsConfig {
            allowed_origins,
            allow_credentials,
            allow_headers: allow_headers
                .map(str::trim)
                .filter(|headers| !headers.is_empty())
                .map(str::to_string),
        }
    }

    /// Whether any origin is permitted. A disabled policy adds no headers to any
    /// response and never answers a preflight.
    pub fn is_enabled(&self) -> bool {
        self.allowed_origins.is_some()
    }

    /// The value to send as `Access-Control-Allow-Origin` for a request from
    /// `origin`, or `None` if the policy does not cover it.
    ///
    /// `"*"` is returned only for an uncredentialed wildcard policy; every other
    /// permitted case echoes the origin, because that is the only form a browser
    /// accepts alongside credentials and the only one that can name a single
    /// entry from a list.
    fn allow_origin_value(&self, origin: &str) -> Option<String> {
        match self.allowed_origins.as_ref()? {
            AllowedOrigins::Any if !self.allow_credentials => Some("*".to_string()),
            AllowedOrigins::Any => Some(origin.to_string()),
            AllowedOrigins::Only(origins) => origins
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(origin))
                .then(|| origin.to_string()),
        }
    }

    /// The CORS headers belonging on the answer to a real (non-preflight) request.
    ///
    /// Empty when the policy is disabled, when the request carries no `Origin` —
    /// which is every request from a non-browser client — or when the origin is
    /// not permitted. In the last case the response is still sent in full; it is
    /// the browser that withholds it from the calling script, and there is nothing
    /// a server can add to a rejection that a browser would read.
    ///
    /// # Examples
    ///
    /// ```
    /// use utils::cors::CorsConfig;
    ///
    /// let cors = CorsConfig::new("*", false, None);
    ///
    /// // No Origin header: not a cross-origin request, so nothing to authorise.
    /// assert!(cors.response_headers("GET / HTTP/1.1\r\n\r\n").is_empty());
    /// ```
    pub fn response_headers(&self, request: &str) -> Vec<(&'static str, String)> {
        let Some(origin) = get_header(request, "origin") else {
            return Vec::new();
        };

        let Some(allow_origin) = self.allow_origin_value(origin) else {
            return Vec::new();
        };

        self.origin_headers(allow_origin)
    }

    /// The full set of headers answering a preflight, or `None` if this policy
    /// does not authorise it.
    ///
    /// `None` means the caller should answer the `OPTIONS` request as an ordinary
    /// one — a `204` carrying `Allow` — rather than as a granted preflight. That
    /// covers a disabled policy and a foreign origin alike: neither is an error to
    /// report, since a preflight that comes back without the CORS headers is
    /// exactly how a browser is told "no".
    ///
    /// `allowed_methods` is what the path actually serves, so the browser is told
    /// the truth rather than having its requested method echoed back at it. A
    /// preflight for `DELETE` against a path that only serves `GET` therefore
    /// comes back listing `GET`, and the browser blocks the request without the
    /// server ever having to reject it.
    ///
    /// # Examples
    ///
    /// ```
    /// use utils::cors::CorsConfig;
    /// use utils::request::route::Method;
    ///
    /// let cors = CorsConfig::new("http://localhost:1420", false, None);
    /// let preflight = "OPTIONS /users HTTP/1.1\r\n\
    ///                  Origin: http://localhost:1420\r\n\
    ///                  Access-Control-Request-Method: POST\r\n\
    ///                  Access-Control-Request-Headers: content-type\r\n\r\n";
    ///
    /// let headers = cors.preflight_headers(preflight, &[Method::POST]).unwrap();
    ///
    /// assert!(headers.contains(&("Access-Control-Allow-Methods", "POST".to_string())));
    /// assert!(headers.contains(&("Access-Control-Allow-Headers", "content-type".to_string())));
    /// ```
    pub fn preflight_headers(
        &self,
        request: &str,
        allowed_methods: &[Method],
    ) -> Option<Vec<(&'static str, String)>> {
        let origin = get_header(request, "origin")?;
        let allow_origin = self.allow_origin_value(origin)?;

        let mut headers = self.origin_headers(allow_origin);

        headers.push((
            "Access-Control-Allow-Methods",
            allowed_methods
                .iter()
                .map(Method::as_str)
                .collect::<Vec<&str>>()
                .join(", "),
        ));

        // Configured list, or whatever was asked for. A preflight that requested
        // no headers gets none back, rather than an empty header.
        let allow_headers = self
            .allow_headers
            .clone()
            .or_else(|| get_header(request, "access-control-request-headers").map(str::to_string));

        if let Some(allow_headers) = allow_headers {
            headers.push(("Access-Control-Allow-Headers", allow_headers));
        }

        headers.push((
            "Access-Control-Max-Age",
            PREFLIGHT_MAX_AGE_SECONDS.to_string(),
        ));

        Some(headers)
    }

    /// The headers common to a preflight and a real response: who is allowed, and
    /// whether the answer varies by who asked.
    fn origin_headers(&self, allow_origin: String) -> Vec<(&'static str, String)> {
        // `Vary: Origin` is a caching requirement, not a CORS one: any response
        // whose `Access-Control-Allow-Origin` was derived from the request must
        // say so, or a shared cache will serve one origin's permission slip to
        // another. A literal `*` is the same for everybody, so it needs no `Vary`.
        let varies_by_origin = allow_origin != "*";

        let mut headers = vec![("Access-Control-Allow-Origin", allow_origin)];

        if varies_by_origin {
            headers.push(("Vary", "Origin".to_string()));
        }

        if self.allow_credentials {
            headers.push(("Access-Control-Allow-Credentials", "true".to_string()));
        }

        headers
    }
}

/// Whether `request` is a CORS preflight rather than a plain `OPTIONS`.
///
/// A preflight is defined by carrying both `Origin` and
/// `Access-Control-Request-Method` (Fetch §3.2.2). Anything else is a client
/// asking what a resource supports, which is a perfectly ordinary `OPTIONS`
/// request and is answered with `Allow` instead.
///
/// The verb itself is not checked here — the caller has already matched it.
///
/// # Examples
///
/// ```
/// use utils::cors::is_preflight;
///
/// let preflight = "OPTIONS /users HTTP/1.1\r\n\
///                  Origin: http://localhost:1420\r\n\
///                  Access-Control-Request-Method: POST\r\n\r\n";
/// assert!(is_preflight(preflight));
///
/// // No Access-Control-Request-Method: just a capability query.
/// assert!(!is_preflight("OPTIONS /users HTTP/1.1\r\nOrigin: http://localhost:1420\r\n\r\n"));
/// ```
pub fn is_preflight(request: &str) -> bool {
    get_header(request, "origin").is_some()
        && get_header(request, "access-control-request-method").is_some()
}

#[cfg(test)]
#[path = "cors_tests.rs"]
mod tests;
