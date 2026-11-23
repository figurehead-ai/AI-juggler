#!/usr/bin/env python3
"""Prints the page for a running read-only test bridge (docs/qa/DEV-BRIDGE.md).

Reads the bridge's 0600 token file in AI Juggler Dev's data folder (or the
folder given as the only argument) and prints the Vite page with the port
in the query and the token in the fragment, which the browser never sends
to the dev server.

`--query` prints the token in the query instead (`&token=`), for a browser
that drops a URL's fragment on navigation (the walk kit's in-app pane,
AJ-E2314 L1). The page reads either and removes the token from the address
at once; the query form reaches only the local dev server, on this Mac.
"""
import json
import os
import pathlib
import stat
import sys

FOLDER = pathlib.Path.home() / "Library" / "Application Support" / "AI Juggler Dev"


def main() -> int:
    args = [arg for arg in sys.argv[1:] if arg != "--query"]
    in_query = len(args) != len(sys.argv) - 1
    folder = pathlib.Path(args[0]) if args else FOLDER
    path = folder / "dev-bridge.json"
    try:
        info = path.stat()
    except FileNotFoundError:
        print(f"no {path}: start AI Juggler Dev with `just dev-bridge` first", file=sys.stderr)
        return 1
    if info.st_uid != os.getuid() or stat.S_IMODE(info.st_mode) != 0o600:
        print(f"{path} isn't owner-only (0600) and ours: not using it", file=sys.stderr)
        return 1
    data = json.loads(path.read_text())
    origin = data["origin"]
    port = int(data["port"])
    token = data["token"]
    if len(token) != 64 or any(c not in "0123456789abcdef" for c in token):
        print(f"{path} holds no valid token", file=sys.stderr)
        return 1
    if in_query:
        print(f"{origin}/?devBridge={port}&token={token}")
    else:
        print(f"{origin}/?devBridge={port}#token={token}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
