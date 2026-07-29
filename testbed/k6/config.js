// Shared settings for every k6 script in this directory.
//
// Imported with a relative path, which k6 resolves itself — there is no bundler,
// no package.json and no node_modules here. Only npm package resolution is
// unavailable in k6's runtime; local file imports work as they look.

// Must match the ip and port in the `#[http_server]` attribute on sample-server's
// main. The macro takes them as literals, so the server cannot be pointed
// elsewhere at runtime and this cannot be overridden by an environment variable
// without editing that attribute too.
export const BASE_URL = 'https://127.0.0.1:8443';

// The server generates a self-signed certificate in-process at every startup, so
// there is no CA to trust and no fingerprint that survives a restart. Without
// this every request fails at the TLS layer rather than reaching a route.
export const TLS_OPTIONS = {
        insecureSkipTLSVerify: true,
};

// The server closes after a single request and says so with `Connection: close`,
// so a connection is never reusable. Declaring that here makes k6 stop trying and
// makes the cost explicit: every request in every scenario pays a full TLS
// handshake. That handshake, not routing, dominates the numbers below.
export const CONNECTION_OPTIONS = {
        noConnectionReuse: true,
};

export const BASE_OPTIONS = {
        ...TLS_OPTIONS,
        ...CONNECTION_OPTIONS,
};

// Deliberately loose. These are a smoke alarm for "something broke badly", not a
// performance target — a threshold tight enough to be interesting would fail on a
// different machine, and the meaningful comparison is against a baseline taken on
// the same hardware in the same sitting.
export const BASE_THRESHOLDS = {
        http_req_failed: ['rate<0.01'],
        checks: ['rate>0.99'],
};

// Every route the sample server registers, with the status each should answer.
// Kept here so smoke.js and load.js cannot drift apart on what the contract is.
export const ROUTES = {
        index: { method: 'GET', path: '/', status: 200 },
        getUser: { method: 'GET', path: '/users/42', status: 200 },
        currentUser: { method: 'GET', path: '/users/me', status: 200 },
        search: { method: 'GET', path: '/search?q=rust&limit=5', status: 200 },
        createUser: { method: 'POST', path: '/users', status: 201 },
        replaceUser: { method: 'PUT', path: '/users/42', status: 200 },
        updateUser: { method: 'PATCH', path: '/users/42', status: 200 },
        deleteUser: { method: 'DELETE', path: '/users/42', status: 200 },
        endSession: { method: 'DELETE', path: '/sessions/1', status: 204 },
};

// Statuses the server produces on its own, without reaching a handler. These are
// the ones worth watching under load: they should stay reachable and correct when
// the connection ceiling is saturated, not degrade into timeouts.
export const ERROR_ROUTES = {
        unparseableParam: { method: 'GET', path: '/users/abc', status: 404 },
        missingParam: { method: 'GET', path: '/search', status: 400 },
        noSuchRoute: { method: 'GET', path: '/nowhere', status: 404 },
        wrongVerb: { method: 'POST', path: '/users/42', status: 405 },
        emptyBody: { method: 'POST', path: '/users', status: 400 },
};

export const USER_BODY = JSON.stringify({ name: 'grace', active: true });

export const JSON_PARAMS = {
        headers: { 'Content-Type': 'application/json' },
};
