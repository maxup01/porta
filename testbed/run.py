#!/usr/bin/env python3
"""Start the sample server, run the k6 suite against it, stop it again.

Three things make a harness like this flaky, and each is handled deliberately
rather than hopefully:

  readiness   Sleeping before k6 starts is slow locally, flaky under load, and
              silently wrong on a busy machine. This completes a real TLS
              handshake against the port in a loop until it succeeds.

  identity    `cargo run` spawns cargo, which spawns the binary. Killing the
              cargo process can leave the server orphaned and still holding the
              port, which breaks the *next* run rather than this one. So the
              build and the launch are separate steps and the PID held here is
              the server itself.

  cleanup     The server must die whether k6 passes, fails, or the run is
              interrupted. Everything after startup sits inside a try/finally,
              and the child is terminated, given a grace period, then killed.

Usage:
    ./run.py                    all scripts, debug build
    ./run.py smoke baseline     only those two
    ./run.py --release          optimised build, which is what you want for
                                any number you intend to quote
    ./run.py --keep-server      leave it running afterwards, to poke at by hand
"""

from __future__ import annotations

import argparse
import os
import shutil
import socket
import ssl
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# Must match the `#[http_server]` attribute in sample-server's main. The macro
# takes them as literals, so there is no way to override this at runtime — if it
# changes there, it changes here and in k6/config.js.
HOST = "127.0.0.1"
PORT = 8443

# Ordered cheapest first, so a broken server fails in a second rather than after
# two minutes of load generation.
DEFAULT_SCRIPTS = ["smoke", "baseline", "error-cost", "ramp", "payload"]

TESTBED = Path(__file__).resolve().parent
SERVER_DIR = TESTBED / "sample-server"
K6_DIR = TESTBED / "k6"

# How long to wait for the port to answer a TLS handshake before giving up. A
# debug binary on a cold page cache can take a couple of seconds to get going.
STARTUP_TIMEOUT_SECONDS = 20.0
POLL_INTERVAL_SECONDS = 0.1

# Time the server gets to exit on SIGTERM before being killed outright.
SHUTDOWN_GRACE_SECONDS = 5.0


def fail(message: str) -> None:
    print(f"\n  error: {message}\n", file=sys.stderr)
    sys.exit(1)


def check_prerequisites(scripts: list[str]) -> None:
    if shutil.which("cargo") is None:
        fail("cargo is not on PATH")

    if shutil.which("k6") is None:
        fail(
            "k6 is not on PATH. It is a single Go binary — install it from "
            "your package manager or k6.io. It is unrelated to Node, so there "
            "is nothing to npm install."
        )

    for name in scripts:
        script = K6_DIR / f"{name}.js"

        if not script.is_file():
            available = sorted(
                p.stem for p in K6_DIR.glob("*.js") if p.stem != "config"
            )
            fail(f"no such script: {script.name}. Available: {', '.join(available)}")


def port_is_occupied() -> bool:
    """True if something is already accepting connections on the port.

    Worth checking before starting rather than after: the macro hardcodes the
    port, so a leaked server from a previous run cannot be worked around, and
    the failure it produces otherwise is a bare 'Failed to bind address' panic
    from inside the child.
    """
    with socket.socket() as probe:
        probe.settimeout(0.5)
        return probe.connect_ex((HOST, PORT)) == 0


def build(release: bool) -> Path:
    profile = "release" if release else "debug"
    command = ["cargo", "build"] + (["--release"] if release else [])

    print(f"  building sample-server ({profile})")

    result = subprocess.run(command, cwd=SERVER_DIR)

    if result.returncode != 0:
        fail("build failed")

    binary = SERVER_DIR / "target" / profile / "sample-server"

    if not binary.is_file():
        fail(f"built, but no binary at {binary}")

    return binary


def wait_until_ready(process: subprocess.Popen, log_path: Path) -> None:
    """Poll until the server completes a TLS handshake, or give up.

    A plain TCP connect would return as soon as the listener is bound, which
    happens before rustls is ready to negotiate. Completing a handshake is the
    only signal that means what it looks like.
    """
    context = ssl.create_default_context()
    context.check_hostname = False
    context.verify_mode = ssl.CERT_NONE

    deadline = time.monotonic() + STARTUP_TIMEOUT_SECONDS

    while time.monotonic() < deadline:
        # A server that panicked on startup will never answer, and waiting the
        # full timeout to discover that wastes twenty seconds and hides the
        # reason.
        if process.poll() is not None:
            print(tail(log_path), file=sys.stderr)
            fail(f"server exited during startup with code {process.returncode}")

        try:
            with socket.create_connection((HOST, PORT), timeout=1.0) as raw:
                with context.wrap_socket(raw):
                    return
        except (ConnectionRefusedError, socket.timeout, ssl.SSLError, OSError):
            time.sleep(POLL_INTERVAL_SECONDS)

    print(tail(log_path), file=sys.stderr)
    fail(
        f"server did not answer on {HOST}:{PORT} within {STARTUP_TIMEOUT_SECONDS:.0f}s"
    )


def tail(path: Path, lines: int = 30) -> str:
    if not path.is_file():
        return ""

    content = path.read_text(errors="replace").splitlines()

    return "\n".join(
        ["", f"  last {lines} lines of {path.name}:", ""] + content[-lines:]
    )


def run_scripts(scripts: list[str]) -> list[str]:
    failed = []

    for name in scripts:
        print(f"\n{'─' * 70}\n  k6 run {name}.js\n{'─' * 70}\n")

        # cwd is the k6 directory so that relative imports of ./config.js and
        # relative output paths in handleSummary resolve as the scripts expect.
        result = subprocess.run(["k6", "run", f"{name}.js"], cwd=K6_DIR)

        if result.returncode != 0:
            failed.append(name)
            print(
                f"\n  {name}.js exited with code {result.returncode}", file=sys.stderr
            )

    return failed


def stop(process: subprocess.Popen) -> None:
    """Terminate, wait, then kill. Runs on every exit path."""
    if process.poll() is not None:
        return

    process.terminate()

    try:
        process.wait(timeout=SHUTDOWN_GRACE_SECONDS)
        print("  server stopped")
        return
    except subprocess.TimeoutExpired:
        pass

    print(f"  server ignored SIGTERM after {SHUTDOWN_GRACE_SECONDS:.0f}s, killing")
    process.kill()
    process.wait(timeout=SHUTDOWN_GRACE_SECONDS)


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "scripts", nargs="*", default=None, help="k6 scripts to run, without .js"
    )
    parser.add_argument("--release", action="store_true", help="build optimised")
    parser.add_argument(
        "--keep-server", action="store_true", help="leave the server running afterwards"
    )
    args = parser.parse_args()

    scripts = args.scripts or DEFAULT_SCRIPTS

    check_prerequisites(scripts)

    if port_is_occupied():
        fail(
            f"something is already listening on {HOST}:{PORT}. The port is baked "
            f"into the #[http_server] attribute, so it cannot be moved — most "
            f"likely a server leaked from an earlier run. Find it with: "
            f"lsof -ti:{PORT}"
        )

    binary = build(args.release)

    # A scratch file outside the repository. Server output is captured rather
    # than inherited, because its own logging — accept failures, a panicking
    # handler — would otherwise interleave with k6's progress bars and make both
    # unreadable. Nothing is written into the project.
    handle, log_name = tempfile.mkstemp(prefix="sample-server-", suffix=".log")
    os.close(handle)
    log_path = Path(log_name)

    print(f"  starting {binary.name}")

    with log_path.open("w") as log:
        process = subprocess.Popen(
            [str(binary)],
            stdout=log,
            stderr=subprocess.STDOUT,
            # Its own process group, so a Ctrl-C in this terminal reaches only
            # this script. The server is then stopped deliberately below rather
            # than racing a signal it also received.
            start_new_session=True,
        )

        try:
            wait_until_ready(process, log_path)
            print(f"  ready on https://{HOST}:{PORT}")

            failed = run_scripts(scripts)
        finally:
            if args.keep_server:
                print(
                    f"\n  leaving server running as pid {process.pid} (--keep-server)"
                )
                print(f"  stop it with: kill {process.pid}")
            else:
                print()
                stop(process)

    print(f"\n{'─' * 70}")

    if failed:
        # Kept on failure so there is something to read, removed otherwise —
        # a passing run should leave nothing behind.
        print(f"  {len(failed)} of {len(scripts)} failed: {', '.join(failed)}")
        print(f"  server log: {log_path}")
        return 1

    log_path.unlink(missing_ok=True)

    print(f"  {len(scripts)} of {len(scripts)} passed")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        # The finally in main() has already stopped the server by the time this
        # is reached; this only keeps the traceback off the screen.
        print("\n  interrupted", file=sys.stderr)
        sys.exit(130)
