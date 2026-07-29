// Offered load climbing from well under the connection ceiling to well past it.
//
// The server serves at most 512 connections at once; the rest wait in the kernel
// accept backlog. Nothing so far establishes what happens at that boundary, and
// the two possibilities look nothing alike:
//
//   graceful — latency climbs as clients queue, throughput plateaus, no errors
//   collapse — the backlog fills too, the kernel refuses connections, clients
//              see resets and timeouts
//
// Both are plausible from reading the code. This is how you find out which one
// you have.
//
//   k6 run ramp.js
//
// Expect this to be noisy on a laptop. k6 and the server compete for the same
// cores, and above a few hundred connections you may be measuring the client.
// Treat the shape of the curve as the finding, not the absolute numbers.

import http from 'k6/http';
import { check } from 'k6';
import { Counter, Trend } from 'k6/metrics';
import { BASE_URL, BASE_OPTIONS, timingSummary } from './config.js';

// The connection ceiling in the generated accept loop. The stages below are
// arranged around it, so if it changes in the macro this has to change with it.
const MAX_CONNECTIONS = 512;

// The cheapest route, so the measurement is about concurrency rather than about
// what a handler does.
const TARGET = `${BASE_URL}/`;

export const options = {
        ...BASE_OPTIONS,

        scenarios: {
                ramp: {
                        // `ramping-arrival-rate`, not `ramping-vus`, and the distinction decides
                        // whether this test can work at all. VU-based load is self-limiting: if
                        // the server slows down, each VU completes fewer iterations, offered load
                        // falls, and the test politely settles just below saturation without ever
                        // finding it. Arrival rate keeps pushing at the target regardless of how
                        // the server is coping, which is the only way to locate a ceiling.
                        executor: 'ramping-arrival-rate',
                        startRate: 50,
                        timeUnit: '1s',

                        // Each request needs its own connection, so k6 needs at least as many VUs
                        // in reserve as the highest rate it will attempt. Too few and k6 reports
                        // "insufficient VUs" — which looks like a server limit but is the client
                        // running out of workers.
                        preAllocatedVUs: 200,
                        maxVUs: 2000,

                        stages: [
                                { target: 100, duration: '20s' },
                                { target: 300, duration: '20s' },
                                { target: MAX_CONNECTIONS, duration: '20s' },
                                { target: 800, duration: '20s' },
                                { target: 1500, duration: '20s' },
                                { target: 0, duration: '10s' },
                        ],
                },
        },

        // No pass/fail thresholds. Every stage past the ceiling is *expected* to
        // degrade — that is the measurement. A threshold here would mark a successful
        // experiment as a failed test.
        thresholds: {},
};

// Failures are separated by kind, because they mean different things. A refused
// or reset connection is the kernel turning clients away; a timeout is a client
// giving up while still queued. Lumping both into `http_req_failed` hides which
// mode the server is in.
const refused = new Counter('conn_refused');
const timedOut = new Counter('conn_timed_out');
const otherError = new Counter('other_error');

// Time spent before the request was even written. Under overload this is where
// queueing shows up: `blocked` covers waiting for a connection slot, and it will
// dominate long before time-to-first-byte moves.
const queueing = new Trend('queue_time_ms');

export default function() {
        // An explicit timeout, well under the server's 30-second request deadline.
        // Without one a queued client waits indefinitely, and the run would report
        // enormous latencies instead of the failure it actually experienced.
        const response = http.get(TARGET, { timeout: '10s' });

        if (response.status === 200) {
                check(response, { 'served': (r) => r.status === 200 });
                queueing.add(response.timings.blocked + response.timings.connecting);
                return;
        }

        const error = (response.error || '').toLowerCase();

        if (error.includes('refused') || error.includes('reset')) {
                refused.add(1);
        } else if (error.includes('timeout') || error.includes('deadline')) {
                timedOut.add(1);
        } else {
                otherError.add(1);
        }

        check(response, { 'served': () => false });
}

export function setup() {
        const response = http.get(TARGET, { timeout: '5s' });

        if (response.status !== 200) {
                throw new Error(
                        `sample-server is not answering on ${BASE_URL} ` +
                        `(status ${response.status}, ${response.error || 'no transport error'}) — ` +
                        `start it with: cd testbed/sample-server && cargo run`,
                );
        }

        // Worth stating up front, because a run that hits the client's own limits
        // looks exactly like a server that fell over.
        console.log(
                `ramping past a ${MAX_CONNECTIONS}-connection ceiling; ` +
                `if this machine also runs the server, expect the top stages to measure both`,
        );
}

export function handleSummary(data) {
        return timingSummary(
                `Concurrency ramp — 50 → 1500 req/s against a ${MAX_CONNECTIONS}-connection ceiling`,
                data,
        );
}
