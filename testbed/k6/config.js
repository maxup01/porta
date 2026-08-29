// Shared settings for every k6 script in this directory.
//
// Imported with a relative path, which k6 resolves itself — there is no bundler,
// no package.json and no node_modules here. Only npm package resolution is
// unavailable in k6's runtime; local file imports work as they look.

// k6's own summary formatter, the one behind the default end-of-test report.
// Fetched over the network and cached, so this is the only thing here that needs
// connectivity, and only on a cold cache.
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.4/index.js';

// Needed only for `setResponseCallback` below. The scripts import `http` themselves.
import http from 'k6/http';

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

// Connections are persistent, so a VU making many requests pays one TLS handshake
// rather than one per request. Left at k6's default — reuse — because that is what
// a real client does and what the numbers below should reflect.
//
// Set `K6_NO_CONNECTION_REUSE=true` to measure the other side of it: every request
// forced through a fresh handshake, which is what this server did before keep-alive
// existed. The difference between the two runs is what persistence bought.
export const CONNECTION_OPTIONS = {
        noConnectionReuse: false,
};

// k6 counts every 4xx as a failed request by default, and these scripts ask for
// 4xx deliberately — five entries in ERROR_ROUTES, plus the oversized and
// non-UTF-8 bodies in smoke.js. Left at the default, `http_req_failed` measures
// how many error routes the suite exercises rather than anything about the server,
// and no threshold on it can be satisfied.
//
// A 4xx the suite asked for is a correct answer; `checks` is what judges whether
// it was the right one. What remains a failure here is what the server should
// never do: a 5xx, or a request that never completed.
//
// Set here rather than in BASE_OPTIONS: this is a function call, not a script
// option, and a `responseCallback` key in an exported `options` object is silently
// ignored. Every script imports this module, so importing it applies the callback.
http.setResponseCallback(http.expectedStatuses({ min: 200, max: 499 }));

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
        emptyBody: { method: 'POST', path: '/users', status: 400, body: null },
};

export const USER_BODY = JSON.stringify({ name: 'grace', active: true });

// Must match `allow_origins` in the `#[http_server]` attribute on sample-server's
// main, for the same reason the URL must: the macro takes it as a literal.
//
// Only smoke.js sends an `Origin` header. Every other script omits it, so the
// server treats those requests as same-origin and the CORS path costs them
// nothing — the load figures are comparable to runs taken before it existed.
export const ALLOWED_ORIGIN = 'http://localhost:1420';

export const JSON_PARAMS = {
        headers: { 'Content-Type': 'application/json' },
};

// Titles a run and renders it with k6's own summary formatter.
//
// `textSummary` is the same code that produces the default end-of-test report, so
// this reproduces the standard table rather than a hand-rolled imitation of it —
// including the `http_req_tls_handshaking` and `http_req_waiting` rows, which are
// the two that matter here. A connection serves exactly one request, so every
// request pays a full handshake; comparing those two rows is how you tell whether
// a slow result is the negotiation or the server.
//
// Lives here so the ramp and payload scenarios report in the same shape. Two runs
// formatted differently are two runs that are tedious to compare.
//
// k6 requires `handleSummary` to be exported from the entry script, so a scenario
// delegates rather than importing this as its hook:
//
//   export function handleSummary(data) {
//     return timingSummary('Floor — 1 VU, GET /', data);
//   }
export function timingSummary(title, data) {
        const report = textSummary(data, { indent: '  ', enableColors: true });

        return { stdout: `\n  ${title}\n${report}\n` };
}
