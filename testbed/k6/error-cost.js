// What a miss costs compared to a hit.
//
// Dispatch is not symmetric. Counting mutex acquisitions per request, against the
// nine routes the sample server registers:
//
//   200  GET /users/42        one lookup in GET_ROUTES                        1
//   405  POST /users/42       lookup misses, then path_exists probes GET
//                             and stops on the first match                    2
//   404  GET /nowhere         lookup misses, then path_exists probes all
//                             five tables and never short-circuits            6
//
// Each of those is a global `Mutex<HashMap>`, held while the lookup runs a linear
// filter plus min_by_key over the whole table for that method. So a miss is not
// just more work, it is more contention — and it costs the client nothing extra.
// Behind a reverse proxy, 404s arrive constantly from scanners and stale links.
// If they are six times more expensive to answer, a burst of junk traffic
// degrades legitimate requests disproportionately.
//
// This measures whether that asymmetry is real at the wire, or whether the
// handshake dominates so heavily that six lock acquisitions disappear into noise.
// Either answer is useful; the second one rules out a whole line of optimisation.
//
//   k6 run error-cost.js

import http from 'k6/http';
import { Trend } from 'k6/metrics';
import { BASE_URL, BASE_OPTIONS, timingSummary } from './config.js';

// One rate for every cohort. Comparing latencies only means something if each
// cohort is offered the same load, so this is deliberately modest — well under
// the connection ceiling, so nothing here is measuring queueing.
const RATE_PER_COHORT = 60;
const DURATION = '30s';

// The cohorts, chosen for their lock arithmetic rather than their variety.
const COHORTS = {
        // Baseline: one acquisition.
        hit: { method: 'GET', path: '/users/42', status: 200, locks: 1 },

        // Served by GET, which `path_exists` probes first, so it short-circuits
        // immediately. The cheap end of the 405 range.
        methodMissEarly: { method: 'POST', path: '/users/42', status: 405, locks: 2 },

        // Served only by DELETE, the last method `path_exists` probes, so every table
        // is locked before the answer is found. The expensive end of the same status —
        // two requests that both return 405 can differ threefold in cost purely
        // because of where a verb sits in a hardcoded array.
        methodMissLate: { method: 'GET', path: '/sessions/1', status: 405, locks: 6 },

        // Nothing serves it, so no probe short-circuits. Worst case.
        pathMiss: { method: 'GET', path: '/nowhere', status: 404, locks: 6 },
};

// Time to first byte, per cohort. Deliberately `waiting` rather than `duration`:
// total duration is dominated by the TLS handshake, which is identical across
// cohorts and would bury the difference being measured.
const serverTime = {};

for (const name of Object.keys(COHORTS)) {
        serverTime[name] = new Trend(`ttfb_${name}_ms`);
}

// Each cohort is its own scenario so k6 paces them independently and they run
// concurrently against the same server — which is the point. Running them in
// sequence would compare a warm server against a cold one.
const scenarios = {};

for (const [name, _] of Object.entries(COHORTS)) {
        scenarios[name] = {
                executor: 'constant-arrival-rate',
                rate: RATE_PER_COHORT,
                timeUnit: '1s',
                duration: DURATION,
                preAllocatedVUs: 40,
                maxVUs: 200,
                exec: name,
                tags: { cohort: name },
        };
}

export const options = {
        ...BASE_OPTIONS,
        scenarios,

        // Correctness only. There is no expected latency here — the numbers are the
        // output, not something to pass or fail against.
        thresholds: {
                http_req_failed: ['rate==0.0'],
        },
};

function measure(name) {
        const cohort = COHORTS[name];
        const body = cohort.method === 'POST' ? '{}' : null;

        const response = http.request(cohort.method, `${BASE_URL}${cohort.path}`, body, {
                timeout: '10s',
                // Tagged so the built-in metrics can also be filtered per cohort, not just
                // the custom trends below.
                tags: { cohort: name },
        });

        // A wrong status means the route assumptions above are stale — the lock counts
        // in the comments are derived from which verbs serve which path, so a changed
        // route silently invalidates the whole comparison.
        if (response.status !== cohort.status) {
                console.error(
                        `${name}: expected ${cohort.status} from ${cohort.method} ${cohort.path}, ` +
                        `got ${response.status}. The cohort no longer measures what it claims to.`,
                );
                return;
        }

        serverTime[name].add(response.timings.waiting);
}

// One exported function per scenario, named to match the `exec` fields above.
export function hit() {
        measure('hit');
}

export function methodMissEarly() {
        measure('methodMissEarly');
}

export function methodMissLate() {
        measure('methodMissLate');
}

export function pathMiss() {
        measure('pathMiss');
}

export function setup() {
        // Verify every cohort resolves to the status its lock count assumes, before
        // spending a minute measuring the wrong thing.
        for (const [name, cohort] of Object.entries(COHORTS)) {
                const response = http.request(
                        cohort.method,
                        `${BASE_URL}${cohort.path}`,
                        cohort.method === 'POST' ? '{}' : null,
                        { timeout: '5s' },
                );

                if (response.status !== cohort.status) {
                        throw new Error(
                                `cohort "${name}" expects ${cohort.status} from ${cohort.method} ${cohort.path} ` +
                                `but got ${response.status} — either the server is not running, or the ` +
                                `sample routes changed and the lock arithmetic in this file is stale`,
                        );
                }
        }
}

export function handleSummary(data) {
        const report = timingSummary(
                `Error-path cost — ${RATE_PER_COHORT} req/s per cohort, ${DURATION} each`,
                data,
        );

        // The comparison, spelled out. Reading four trends and dividing them by eye is
        // exactly the step that gets skipped.
        const ttfb = (name) => data.metrics[`ttfb_${name}_ms`]?.values?.['p(95)'];
        const baseline = ttfb('hit');

        const lines = ['', '  p95 time to first byte, relative to a hit', ''];

        for (const [name, cohort] of Object.entries(COHORTS)) {
                const value = ttfb(name);
                const ratio =
                        Number.isFinite(value) && baseline > 0 ? `${(value / baseline).toFixed(2)}×` : '   n/a';

                lines.push(
                        `    ${name.padEnd(18)} ${String(cohort.status)}  ` +
                        `${Number.isFinite(value) ? value.toFixed(3).padStart(7) : '    n/a'} ms  ` +
                        `${ratio.padStart(6)}   (${cohort.locks} lock${cohort.locks === 1 ? '' : 's'})`,
                );
        }

        lines.push(
                '',
                '  If the ratios track the lock counts, the route-table mutex is the cost and',
                '  an RwLock or a single combined lookup would remove it. If they are all ~1×,',
                '  the handshake dominates and dispatch is not worth optimising.',
                '',
        );

        return { stdout: report.stdout + lines.join('\n') };
}
