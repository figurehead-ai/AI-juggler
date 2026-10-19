#!/usr/bin/env python3
"""D-20 release versions: one stream, channels are pointers.

  release_version.py next RUN_NUMBER     print MAJOR.MINOR.RUN_NUMBER
  release_version.py stamp X.Y.Z         write X.Y.Z into the workspace
                                         Cargo.toml and tauri.conf.json
                                         (CI checkout only, never committed)
  release_version.py --selftest

Every nightly build gets the next version: MAJOR.MINOR come from the
workspace version in Cargo.toml (the maintainer bumps them), PATCH is
the nightly workflow's run number, so versions only ever grow. A build
promoted to beta or stable keeps its version: nothing is rebuilt.
"""
from __future__ import annotations

import json
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = Path("Cargo.toml")
TAURI = Path("crates/juggler-app/tauri.conf.json")
WORKSPACE_VERSION = re.compile(r'^(version\s*=\s*")(\d+)\.(\d+)\.(\d+)(")', re.M)


def base(root: Path) -> tuple[int, int]:
    match = WORKSPACE_VERSION.search((root / CARGO).read_text())
    if not match:
        raise SystemExit("Cargo.toml has no workspace version")
    return int(match.group(2)), int(match.group(3))


def next_version(root: Path, run_number: int) -> str:
    if run_number < 1:
        raise SystemExit("the run number must be positive")
    major, minor = base(root)
    return f"{major}.{minor}.{run_number}"


def stamp(root: Path, version: str) -> None:
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise SystemExit(f"not a release version: {version!r}")
    cargo = root / CARGO
    text, count = WORKSPACE_VERSION.subn(
        lambda m: f"{m.group(1)}{version}{m.group(5)}", cargo.read_text(), count=1
    )
    if count != 1:
        raise SystemExit("Cargo.toml has no workspace version")
    cargo.write_text(text)
    tauri = root / TAURI
    conf = json.loads(tauri.read_text())
    conf["version"] = version
    tauri.write_text(json.dumps(conf, indent=2) + "\n")


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / TAURI).parent.mkdir(parents=True)
        (root / CARGO).write_text('[workspace]\nmembers = []\n\n[workspace.package]\nversion = "0.3.1"\nedition = "2024"\n')
        (root / TAURI).write_text(json.dumps({"productName": "AI Juggler", "version": "0.3.1"}))
        assert next_version(root, 57) == "0.3.57"
        stamp(root, "0.3.57")
        assert 'version = "0.3.57"' in (root / CARGO).read_text()
        assert json.loads((root / TAURI).read_text())["version"] == "0.3.57"
        assert json.loads((root / TAURI).read_text())["productName"] == "AI Juggler"
        for bad in ("0.3", "v0.3.1", "0.3.1-beta"):
            try:
                stamp(root, bad)
            except SystemExit:
                continue
            raise AssertionError(f"accepted {bad!r}")
    print("release-version selftest: OK")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    if len(args) == 2 and args[0] == "next":
        print(next_version(ROOT, int(args[1])))
        return 0
    if len(args) == 2 and args[0] == "stamp":
        stamp(ROOT, args[1])
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
