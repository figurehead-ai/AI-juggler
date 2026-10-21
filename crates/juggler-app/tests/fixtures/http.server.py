"""A dev-server stand-in for the process tests: `python3 -m http.server`
that never outlives the test that started it.

    python3 http.server.py PORT --bind 127.0.0.1 --follow PID [--setsid]

It listens on PORT like `python3 -m http.server` (its command line still
reads "http.server", so the app sees a dev server), in one process with
one thread: Linux lists a process's threads as processes too (sysinfo's
tasks), so a serving thread read as a second dev server there. It
follows PID, the test process, not its parent: tests kill its parent shell
on purpose to make it an orphan. When PID is gone (the test ended, failed
or was killed) it exits. `--setsid` moves it to its own session first (the
escaped-helper fixture). scripts/leaked_fixtures.sh finds any left behind
by this file's path.
"""

import argparse
import http.server
import os


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("port", type=int)
    parser.add_argument("--bind", default="127.0.0.1")
    parser.add_argument("--follow", type=int, required=True)
    parser.add_argument("--setsid", action="store_true")
    args = parser.parse_args()
    if args.setsid:
        os.setsid()
    server = http.server.HTTPServer((args.bind, args.port), http.server.SimpleHTTPRequestHandler)
    # Serve and watch PID on this one thread: each wait for a request
    # gives up after half a second, then PID is checked again.
    server.timeout = 0.5
    while alive(args.follow):
        server.handle_request()


if __name__ == "__main__":
    main()
