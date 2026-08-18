# testbed

A real downstream consumer of `porta`, plus the load tests that run against it.

Excluded from the workspace and from the published crate. Nothing here ships.

## Why it exists

Two things the workspace tests cannot reach.

**Macro expansion in someone else's crate.** `#[http_server]` emits absolute `::porta::…` paths into the *caller's* crate. Inside the workspace those paths resolve for reasons that do not apply downstream, so a broken re-export compiles fine in `cargo test` and fails for the first real user. `sample-server` depends on the published crate by path and imports nothing else, which is the only arrangement that proves the plumbing works.

**The accept loop.** Certificate generation, the listener, the 512-connection semaphore and the TLS handshake are emitted into `main` and exist nowhere as callable code. A k6 run over a real socket is the only thing that exercises them.

Everything else — the read loop, the size ceiling, the request deadline, dispatch — lives in the `server` crate and is unit-tested there over an in-memory pipe. Those tests are faster, more precise, and point at a line number when they fail. Nothing here duplicates them.

## Running

```bash
./run.py                    # all five scripts, debug build
./run.py smoke baseline     # a subset
./run.py --release          # optimised, for numbers you intend to quote
./run.py --keep-server      # leave the server up afterwards to poke at
```

The script builds `sample-server`, launches it, waits for the port to complete a TLS handshake, runs each k6 script in turn, then stops it. The server dies whether the run passes, fails, or is interrupted.

Three prerequisites:

- **`cargo`** — builds `sample-server`.
- **`k6`** on `PATH` — a single Go binary, unrelated to Node. There is no `package.json` here and nothing to install with npm. On Arch it is AUR-only (`k6-bin`), or drop the release tarball's binary into `/usr/local/bin`.
- **Python 3.9+** — Standard library only; nothing to pip install.

`run.py` checks for `cargo` and `k6` before doing anything and names whichever is missing. Python checks itself by failing to start.

## Layout

```
testbed/
├── sample-server/       nine routes across all five verbs
│   ├── Cargo.toml       depends on porta by path, and serde
│   └── src/main.rs
├── k6/
│   ├── config.js        shared URL, TLS options, route table, summary format
│   ├── smoke.js         one pass over every route, asserts each status
│   ├── baseline.js      the floor: 1 VU, cheapest route, no contention
│   ├── ramp.js          50 → 1500 req/s, past the connection ceiling
│   ├── error-cost.js    what a 404 costs relative to a 200
│   └── payload.js       1 KB → 1000 KB bodies
└── run.py
```

## What each script answers

**`smoke.js`** — does it work end to end? One VU, one iteration. Checks every route's status over real TLS, plus the framing that only exists on the wire: `Connection: close`, the `Date` header, a 204 arriving with no body and no content headers, route specificity keeping `/users/me` and `/users/42` apart, and the 413 and 400 rejection paths.

**`baseline.js`** — what does one request cost with nothing competing? The floor every other number is read against. Its most useful output is not the total but the split between `http_req_tls_handshaking` and `http_req_waiting`: a connection serves exactly one request, so every request pays a full handshake. If that dwarfs time-to-first-byte, keep-alive is worth more than any dispatch optimisation.

**`ramp.js`** — what happens past 512 concurrent connections? Either latency climbs while clients queue in the accept backlog, or the backlog fills and the kernel starts refusing. Failures are counted separately as refused versus timed out, because those describe different modes.

**`error-cost.js`** — is a miss more expensive than a hit? A 200 takes one route-table lock; a 404 takes six, because `path_exists` probes every method and never short-circuits. A 405 takes two or six depending on where the serving verb sits in a hardcoded array. This measures whether that shows up at the wire.

**`payload.js`** — does the read loop scale linearly with body size? Four sizes as concurrent cohorts. Read the µs-per-kilobyte column: flat means linear, rising means something is superlinear.

## Reading the numbers

**Every request includes a full TLS handshake.** The server has no keep-alive, so this is not an artefact of the test — it is how the server actually behaves. It also means throughput figures are mostly a handshake benchmark. Compare against `baseline.js` from the same session before concluding anything about dispatch.

**Numbers are machine-specific.** A p95 from this laptop and a p95 from a CI runner are not comparable. Measure, change, re-measure, same machine, same sitting.

**Above a few hundred connections you may be measuring k6.** It competes with the server for the same cores. Treat the shape of the curve as the finding rather than the absolute values, or run the client elsewhere.

**Only `smoke.js` has meaningful thresholds.** The rest report rather than pass or fail — `ramp.js` in particular is *expected* to degrade at the top, and a threshold there would mark a successful experiment as a failed test.

## Things that will drift

Three constants are duplicated between the server and the tests, and nothing enforces agreement:

| Value | Defined in | Used by |
|---|---|---|
| `127.0.0.1:8443` | the `#[http_server]` attribute | `k6/config.js`, `run.py` |
| 512 connections | the generated accept loop | `ramp.js` |
| 1 MiB request ceiling | `server::Limits` | `payload.js`, `smoke.js` |

The port is the awkward one: the macro takes it as a literal, so it cannot be overridden at runtime and a leaked server from a previous run cannot be worked around. `run.py` checks for that before building and tells you to run `lsof -ti:8443`.

`error-cost.js` additionally depends on *which* verbs serve which paths in `sample-server`, since its lock counts are derived from that. Its `setup()` verifies every cohort's status before measuring, so a changed route fails loudly instead of producing confidently wrong ratios.

## Not here yet

Baselines are not recorded. Runs print to the terminal and leave nothing behind — the server log goes to a temp file, kept only if something failed. When the Python analysis arrives, `handleSummary` in each script wants to write JSON somewhere so that runs can be compared over time. Worth doing before you need it: the first comparison needs data from runs you had no reason to keep.
