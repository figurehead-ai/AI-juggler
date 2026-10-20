#!/usr/bin/env python3
"""Retry a signing/notarization command when Apple's services blip.

  apple_retry.py [--attempts N] -- COMMAND [ARG...]
  apple_retry.py --selftest

Signing a release talks to three Apple services: the timestamp server
(codesign --timestamp), the notary service (notarytool) and the ticket
service (stapler). Any of them can fail for a few minutes; the 2026-10-20
nightly lost its Nightly edition to "A timestamp was expected but was not
found" while the main edition, signed minutes earlier, was fine.

This runs COMMAND, streaming its output as usual. If it fails AND the
output matches one of Apple's known transient failures, it waits and runs
it again (up to N attempts, 30 s → 60 s → 120 s apart). Any other failure
— a wrong password, a rejected notarization, a compile error — fails at
once, exactly as before: nothing real is ever retried away.

APPLE_RETRY_DELAY_S overrides the first wait (the self-test uses 0).
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# Apple-side, temporary: the service didn't answer or answered 5xx.
TRANSIENT = [
    r"A timestamp was expected but was not found",
    r"timestamp service is not available",
    r"The timestamp server",
    r"unable to build chain to self-signed root.*timestamp",
    r"HTTP status code: 5\d\d",
    r"\b(502|503|504) (Bad Gateway|Service Unavailable|Gateway Time-?out)",
    r"The network connection was lost",
    r"The request timed out",
    r"Could not connect to the server",
    r"A server with the specified hostname could not be found",
    r"NSURLErrorDomain",
    # stapler: the ticket isn't in Apple's CDN yet / CloudKit hiccup.
    r"The staple and validate action failed! Error 6[58]",
    r"CloudKit query .* failed",
]
PATTERN = re.compile("|".join(TRANSIENT), re.IGNORECASE)


def transient(output: str) -> str | None:
    match = PATTERN.search(output)
    return match.group(0) if match else None


def run(command: list[str], attempts: int, first_delay: float) -> int:
    delay = first_delay
    for attempt in range(1, attempts + 1):
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        seen: list[str] = []
        assert process.stdout is not None
        for line in process.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()
            seen.append(line)
        code = process.wait()
        if code == 0:
            return 0
        reason = transient("".join(seen))
        if reason is None:
            return code  # a real failure: never retried
        if attempt == attempts:
            print(f"::error::Apple's service still failing after {attempts} attempts ({reason}).", flush=True)
            return code
        print(f"::warning::Apple's service blipped ({reason}); attempt {attempt + 1} of {attempts} in {delay:.0f} s.", flush=True)
        time.sleep(delay)
        delay *= 2
    return 1


def selftest() -> int:
    os.environ["APPLE_RETRY_DELAY_S"] = "0"
    with tempfile.TemporaryDirectory() as tmp:
        counter = Path(tmp) / "n"

        def stub(failures: int, message: str) -> list[str]:
            # Fails `failures` times printing `message`, then succeeds.
            return [
                sys.executable,
                "-c",
                "import pathlib,sys\n"
                f"p=pathlib.Path({str(counter)!r}); n=int(p.read_text()) if p.exists() else 0; p.write_text(str(n+1))\n"
                f"if n < {failures}: print({message!r}); sys.exit(1)\n"
                "print('signed')",
            ]

        def calls() -> int:
            value = int(counter.read_text()) if counter.exists() else 0
            counter.unlink(missing_ok=True)
            return value

        cases = [
            ("timestamp blip, then fine", stub(1, "x.dmg: A timestamp was expected but was not found."), 4, 0, 2),
            ("notary 503 twice, then fine", stub(2, "Error: HTTP status code: 503. Service Unavailable"), 4, 0, 3),
            ("stapler not yet propagated", stub(1, "The staple and validate action failed! Error 65."), 4, 0, 2),
            ("a real failure is not retried", stub(5, "Error: invalid credentials. Username or password is incorrect."), 4, 1, 1),
            ("a persistent blip gives up", stub(9, "A timestamp was expected but was not found"), 3, 1, 3),
        ]
        failed = 0
        for name, command, attempts, want_code, want_calls in cases:
            code = run(command, attempts, 0)
            got_calls = calls()
            ok = (code == 0) == (want_code == 0) and got_calls == want_calls
            print(f"{'ok  ' if ok else 'FAIL'} {name}: exit {code}, {got_calls} run(s)")
            failed += not ok
    print("apple-retry selftest: " + ("OK" if failed == 0 else f"{failed} FAILED"))
    return 1 if failed else 0


def main(argv: list[str]) -> int:
    if argv[:1] == ["--selftest"]:
        return selftest()
    attempts = 4
    if argv[:1] == ["--attempts"]:
        attempts = int(argv[1])
        argv = argv[2:]
    if argv[:1] != ["--"] or len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    return run(argv[1:], attempts, float(os.environ.get("APPLE_RETRY_DELAY_S", "30")))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
