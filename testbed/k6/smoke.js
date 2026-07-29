// One virtual user, one pass over every route the sample server registers.
//
// This is an end-to-end check rather than a load test. It is the only thing that
// exercises the parts of `#[http_server]` that exist nowhere in the workspace as
// callable code — the accept loop, the TLS handshake, the connection semaphore —
// so a failure here means the macro expansion is broken in a way `cargo test`
// cannot see.
//
// It deliberately does not re-test what the Rust suite already covers. It asserts
// the status reached the wire, not that routing chose it correctly; if a status is
// wrong, the Rust tests will say which line is responsible and this will not.
//
//   k6 run smoke.js

import http from 'k6/http';
import { check, fail } from 'k6';
import {
        BASE_URL,
        BASE_OPTIONS,
        ROUTES,
        ERROR_ROUTES,
        USER_BODY,
        JSON_PARAMS,
} from './config.js';

export const options = {
        ...BASE_OPTIONS,
        vus: 1,
        iterations: 1,

        // Stricter than the shared thresholds on purpose. A smoke run has no
        // concurrency and no contention, so anything less than a clean sweep is a
        // genuine defect rather than load-induced noise.
        thresholds: {
                checks: ['rate==1.0'],
                http_req_failed: ['rate==0.0'],
        },
};

// Bodies are only sent for the verbs that bind one. Sending a body to GET or
// DELETE would not break anything — the server ignores it for those methods — but
// it would misrepresent what is being tested.
const SENDS_BODY = ['POST', 'PUT', 'PATCH'];

function request(name, route) {
        const url = `${BASE_URL}${route.path}`;
        const body = SENDS_BODY.includes(route.method) ? USER_BODY : null;

        const response = http.request(route.method, url, body, JSON_PARAMS);

        const ok = check(response, {
                [`${name}: ${route.method} ${route.path} → ${route.status}`]: (r) =>
                        r.status === route.status,
        });

        if (!ok) {
                // Printed rather than swallowed: a bare "check failed" line gives no way to
                // tell a wrong status from a connection that never completed.
                console.error(
                        `${name}: expected ${route.status}, got ${response.status} — ${response.error || 'no transport error'}`,
                );
        }

        return response;
}

export default function() {
        // ── Handler routes ────────────────────────────────────────────────────────
        for (const [name, route] of Object.entries(ROUTES)) {
                request(name, route);
        }

        // ── Statuses the server generates without reaching a handler ──────────────
        for (const [name, route] of Object.entries(ERROR_ROUTES)) {
                request(name, route);
        }

        // ── Framing checks that only exist on the wire ────────────────────────────
        //
        // Everything below inspects headers and body bytes rather than status codes.
        // These are properties of how the response was serialised, which the Rust
        // tests assert against a String — here they are read back off a socket.

        const index = http.get(`${BASE_URL}/`);

        check(index, {
                'declares Connection: close': (r) =>
                        (r.headers['Connection'] || '').toLowerCase() === 'close',
                'sends a Date header': (r) => r.headers['Date'] !== undefined,
                'serves JSON from a handler': (r) =>
                        (r.headers['Content-Type'] || '').includes('application/json'),
        });

        // 204 is the one status where the body must not be sent at all. The handler
        // behind this route returns a non-empty body on purpose, so a body arriving
        // here means the suppression happens nowhere or only in the unit test.
        const noContent = http.del(`${BASE_URL}/sessions/1`);

        check(noContent, {
                '204 carries no body': (r) => r.body === null || r.body === '',
                '204 omits Content-Length': (r) => r.headers['Content-Length'] === undefined,
                '204 omits Content-Type': (r) => r.headers['Content-Type'] === undefined,
        });

        // The literal route and the parameterised one are registered for the same
        // method and the same shape. Specificity ranking is what keeps them apart, and
        // it is decided inside the macro-generated dispatch.
        const literal = http.get(`${BASE_URL}/users/me`);
        const parameterised = http.get(`${BASE_URL}/users/42`);

        check({ literal, parameterised }, {
                '/users/me reaches the literal route': () =>
                        literal.status === 200 && JSON.parse(literal.body).name === 'me',
                '/users/42 reaches the parameterised route': () =>
                        parameterised.status === 200 && JSON.parse(parameterised.body).id === 42,
        });

        // Path params are collected first, then query params are merged over them, so
        // a query string wins a name collision. Silent precedence rules are worth
        // pinning somewhere a reader will find them.
        const collision = http.get(`${BASE_URL}/users/7?id=9`);

        check(collision, {
                'query parameter overrides path parameter': (r) =>
                        r.status === 200 && JSON.parse(r.body).id === 9,
        });

        // Over the 1 MiB ceiling. Built here rather than in config.js so no other
        // script pays the allocation just by importing.
        const oversized = http.post(
                `${BASE_URL}/users`,
                'x'.repeat(2 * 1024 * 1024),
                JSON_PARAMS,
        );

        check(oversized, {
                'body over the ceiling is refused with 413': (r) => r.status === 413,
        });

        // A body that is bytes rather than text. The server decodes with a checked
        // conversion, so this must be refused rather than silently mangled.
        const invalidUtf8 = http.post(
                `${BASE_URL}/users`,
                new Uint8Array([0xff, 0xfe, 0xfd, 0xfc]).buffer,
                JSON_PARAMS,
        );

        check(invalidUtf8, {
                'body that is not UTF-8 is refused with 400': (r) => r.status === 400,
        });
}

// Runs once before the iteration. If the server is not up, every check below
// would fail in the same confusing way, so fail loudly and immediately instead.
export function setup() {
        const response = http.get(`${BASE_URL}/`, { timeout: '5s' });

        if (response.status !== 200) {
                fail(
                        `sample-server is not answering on ${BASE_URL} ` +
                        `(status ${response.status}, ${response.error || 'no transport error'}) — ` +
                        `start it with: cd testbed/sample-server && cargo run`,
                );
        }
}
