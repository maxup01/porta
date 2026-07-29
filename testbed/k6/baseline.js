// One virtual user, one route, for a fixed duration. The floor.
//
// This measures what a single request costs when nothing is competing for
// anything: no concurrency, no contention on the route-table mutex, no queueing
// at the connection semaphore. Every other number is only interpretable next to
// it — a payload test reporting 40 ms means nothing until you know whether the
// floor is 2 ms or 38 ms.
//
// Single VU is the design, not a shortcut. Add concurrency and you are measuring
// contention, which is a different question and belongs in the ramp test.
//
//   k6 run baseline.js
//
// The most useful output is not the headline duration but the split between
// `http_req_tls_handshaking` and `http_req_waiting`, printed in the summary
// below. Since a connection serves exactly one request, every request pays a
// full handshake — if that dwarfs time-to-first-byte, then dispatch is not the
// thing worth optimising, keep-alive is.

import http from 'k6/http';
import { check } from 'k6';
import { Trend } from 'k6/metrics';
import { BASE_URL, BASE_OPTIONS, timingSummary } from './config.js';

// `/` takes no path parameters, no query string and no body, and returns a short
// string. It is the cheapest route the sample server has, which is what makes it
// the floor rather than a sample of typical work.
const TARGET = `${BASE_URL}/`;

export const options = {
  ...BASE_OPTIONS,
  vus: 1,
  duration: '30s',

  // No latency thresholds. A number tight enough to be meaningful on this machine
  // would fail on another, and the comparison that matters is against a run taken
  // on the same hardware in the same sitting. Correctness thresholds only.
  thresholds: {
    http_req_failed: ['rate==0.0'],
    checks: ['rate==1.0'],
  },
};

// k6 already separates handshake from server time in its built-in sub-metrics.
// These two trends exist so the ratio appears as a single number in the summary
// rather than something you compute by eye from two rows.
const handshakeShare = new Trend('handshake_share_percent');
const serverShare = new Trend('server_share_percent');

export default function () {
  const response = http.get(TARGET);

  check(response, {
    'floor route answers 200': (r) => r.status === 200,
  });

  const total = response.timings.duration;

  // Guard against a zero total, which happens on the occasional sub-microsecond
  // loopback response and would otherwise produce Infinity in the trend.
  if (total > 0) {
    handshakeShare.add((response.timings.tls_handshaking / total) * 100);
    serverShare.add((response.timings.waiting / total) * 100);
  }
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
}

// k6 only honours `handleSummary` when the entry script exports it, so this
// delegates rather than importing the formatter as the hook directly. The shape
// of the report lives in config.js, shared with every other scenario.
export function handleSummary(data) {
  return timingSummary('Floor — 1 VU, GET /, no concurrency', data);
}
