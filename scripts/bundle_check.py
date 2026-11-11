#!/usr/bin/env python3
"""Every helper a release ships beside the app is in the built .app.

The helpers are Tauri's `externalBin` entries: the app's own
`tauri.conf.json` and the release's `tauri.bundle.json` (the one list the
release candidate build and the release workflow both take). Each must be
an executable in the .app's Contents/MacOS. A release candidate without
one is not the release (rc4 and rc5 had no juggler-agent: no hook
reports, no pool tools).

Usage: bundle_check.py "<path>/AI Juggler Dev.app"
       bundle_check.py --selftest
"""
from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONFIGS = (ROOT / "crates/juggler-app/tauri.conf.json", ROOT / "crates/juggler-app/tauri.bundle.json")


def helpers(configs: tuple[Path, ...] = CONFIGS) -> list[str]:
    """The helpers' file names, from every config's bundle.externalBin."""
    names: list[str] = []
    for config in configs:
        if not config.exists():
            continue
        for entry in json.loads(config.read_text()).get("bundle", {}).get("externalBin", []):
            name = Path(entry).name
            if name not in names:
                names.append(name)
    return names


def missing(app: Path, names: list[str]) -> list[str]:
    folder = app / "Contents" / "MacOS"
    return [name for name in names if not os.access(folder / name, os.X_OK)]


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        config = root / "bundle.json"
        config.write_text(json.dumps({"bundle": {"externalBin": ["binaries/juggler-agent"]}}))
        names = helpers((config, root / "absent.json"))
        assert names == ["juggler-agent"], names
        app = root / "X.app"
        (app / "Contents/MacOS").mkdir(parents=True)
        assert missing(app, names) == ["juggler-agent"], "a missing helper fails"
        helper = app / "Contents/MacOS/juggler-agent"
        helper.write_text("#!/bin/sh\n")
        assert missing(app, names) == ["juggler-agent"], "a helper must be executable"
        helper.chmod(0o755)
        assert missing(app, names) == [], "present and executable passes"
    print("bundle-check selftest: OK")
    return 0


def main(argv: list[str]) -> int:
    if argv == ["--selftest"]:
        return selftest()
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    app = Path(argv[0])
    names = helpers()
    if not names:
        print("bundle-check: no externalBin listed; nothing to check")
        return 0
    gone = missing(app, names)
    if gone:
        print(f"bundle-check: {app.name} lacks {', '.join(gone)} in Contents/MacOS (listed in externalBin)", file=sys.stderr)
        return 1
    print(f"bundle-check: OK ({', '.join(names)} in {app.name})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
