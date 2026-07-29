// How the cost of a request scales with the size of its body.
//
// The read loop pulls 4 KB at a time, so body size decides how many iterations it
// runs: 1 KB is one read, 1000 KB is two hundred and fifty. On top of that, three
// separate passes touch every byte before a handler sees it:
//
//   1. `data` grows by extend_from_slice, reallocating and copying as it doubles
//   2. `String::from_utf8` validates the whole buffer
//   3. `extract_request_body` copies the body out into a fresh String
//   4. `serde_json::from_str` allocates the deserialized value on top
//
// If that pipeline is linear, latency tracks size and cost per kilobyte stays
// flat. If cost per kilobyte climbs with size, something is superlinear —
// reallocation thrash being the likeliest candidate — and that is the finding.
//
// The `duplex` tests in the server crate already prove the multi-chunk path is
// correct. This asks what it costs over a real socket, where TLS record framing
// adds its own per-chunk work.
//
//   k6 run payload.js

import http from 'k6/http';
import { Trend } from 'k6/metrics';
import { BASE_URL, BASE_OPTIONS, JSON_PARAMS, timingSummary } from './config.js';

// The server refuses anything over 1 MiB total, headers included. The largest
// body here leaves roughly 24 KB of headroom, so this sweep stays entirely on the
// accepted side of the ceiling — the 413 path is smoke.js's job.
const SIZES_KB = [1, 64, 512, 1000];

// Deliberately low. A 1000 KB body at 20 req/s is 20 MB/s, which loopback handles
// without becoming the bottleneck; raise it and the largest cohort starts
// measuring the network instead of the server.
const RATE_PER_SIZE = 20;
const DURATION = '30s';

// Built once at init rather than per iteration. k6 runs module scope once per VU,
// so this allocates a handful of times in total instead of thousands — otherwise
// the client spends more time building megabyte strings than the server spends
// reading them.
const BODIES = {};

for (const kb of SIZES_KB) {
        // `{"name":"…","active":true}` is 28 bytes of structure around the padding, and
        // the padding lands in a String field so serde has to allocate it. Subtracting
        // the overhead keeps the wire size honest.
        const padding = 'x'.repeat(kb * 1024 - 28);

        BODIES[kb] = JSON.stringify({ name: padding, active: true });
}

// Two trends per size, because they answer different questions. `sending` is how
// long the client spent writing — mostly network and TLS framing. `waiting` is
// time to first byte, which is where the read loop, the copies and the
// deserialization live. Only the second is about the server.
const uploadTime = {};
const serverTime = {};

for (const kb of SIZES_KB) {
        uploadTime[kb] = new Trend(`upload_${kb}kb_ms`);
        serverTime[kb] = new Trend(`server_${kb}kb_ms`);
}

// One scenario per size, running concurrently so no cohort gets a warmer server
// than another. Named `size_1`, `size_64` and so on to match the exec functions.
const scenarios = {};

for (const kb of SIZES_KB) {
        scenarios[`size_${kb}`] = {
                executor: 'constant-arrival-rate',
                rate: RATE_PER_SIZE,
                timeUnit: '1s',
                duration: DURATION,
                preAllocatedVUs: 20,
                maxVUs: 100,
                exec: `size_${kb}`,
                tags: { size_kb: String(kb) },
        };
}

export const options = {
        ...BASE_OPTIONS,
        scenarios,

        // Correctness only. Latency is expected to grow with size — that is the
        // measurement, not a regression.
        thresholds: {
                http_req_failed: ['rate==0.0'],
        },
};

function measure(kb) {
        const response = http.post(`${BASE_URL}/users`, BODIES[kb], {
                ...JSON_PARAMS,
                timeout: '30s',
                tags: { size_kb: String(kb) },
        });

        // A 413 here means a size crossed the ceiling — most likely because the limit
        // changed in `server::Limits` and this file did not follow.
        if (response.status === 413) {
                console.error(
                        `${kb} KB was refused with 413 — the request exceeds the server's ceiling, ` +
                        `so SIZES_KB no longer fits under it`,
                );
                return;
        }

        if (response.status !== 201) {
                console.error(`${kb} KB: expected 201, got ${response.status}`);
                return;
        }

        uploadTime[kb].add(response.timings.sending);
        serverTime[kb].add(response.timings.waiting);
}

// k6 resolves `exec` by name at parse time, so these cannot be generated in a
// loop — each scenario needs a real exported binding.
export function size_1() {
        measure(1);
}

export function size_64() {
        measure(64);
}

export function size_512() {
        measure(512);
}

export function size_1000() {
        measure(1000);
}

export function setup() {
        // Confirm both ends of the sweep before spending two minutes on it: the
        // smallest proves the server is up, the largest proves the ceiling still
        // accommodates it.
        for (const kb of [SIZES_KB[0], SIZES_KB[SIZES_KB.length - 1]]) {
                const response = http.post(`${BASE_URL}/users`, BODIES[kb], {
                        ...JSON_PARAMS,
                        timeout: '10s',
                });

                if (response.status !== 201) {
                        throw new Error(
                                `${kb} KB body returned ${response.status} rather than 201 ` +
                                `(${response.error || 'no transport error'}) — either the server is not ` +
                                `running, or this size no longer fits under its request ceiling`,
                        );
                }
        }
}

export function handleSummary(data) {
        const report = timingSummary(
                `Payload sweep — ${SIZES_KB.join(', ')} KB at ${RATE_PER_SIZE} req/s each`,
                data,
        );

        const p95 = (name) => data.metrics[name]?.values?.['p(95)'];
        const smallest = SIZES_KB[0];
        const baselineServer = p95(`server_${smallest}kb_ms`);

        const lines = [
                '',
                '  p95 by body size',
                '',
                '    size      upload    server    vs 1 KB   µs per KB',
        ];

        for (const kb of SIZES_KB) {
                const upload = p95(`upload_${kb}kb_ms`);
                const server = p95(`server_${kb}kb_ms`);

                // Cost per kilobyte is the number that matters. Flat means the pipeline is
                // linear in body size; rising means something scales worse than the data.
                const perKb = Number.isFinite(server) ? (server * 1000) / kb : undefined;
                const ratio =
                        Number.isFinite(server) && baselineServer > 0 ? server / baselineServer : undefined;

                const fmt = (value, digits = 3) =>
                        Number.isFinite(value) ? value.toFixed(digits).padStart(8) : '     n/a';

                lines.push(
                        `    ${String(kb + ' KB').padEnd(8)}` +
                        `${fmt(upload)}  ${fmt(server)}  ` +
                        `${Number.isFinite(ratio) ? (ratio.toFixed(1) + '×').padStart(8) : '     n/a'}  ` +
                        `${fmt(perKb, 1)}`,
                );
        }

        lines.push(
                '',
                '  Read the last column, not the middle ones. Flat µs per KB means the read',
                '  loop and its three copies scale linearly with the body. Rising means',
                '  something is superlinear and worth profiling — start with the Vec growth',
                '  in read_request, which reallocates as it doubles.',
                '',
        );

        return { stdout: report.stdout + lines.join('\n') };
}
