#!/usr/bin/env python3
"""D-20 release versions: one stream, channels are pointers.

  release_version.py next < TAGS        print the next MAJOR.MINOR.PATCH,
                                         from the release tags on stdin
  release_version.py stamp X.Y.Z         write X.Y.Z into the workspace
                                         Cargo.toml, every crate's internal
                                         dependencies and tauri.conf.json
                                         (CI checkout only, never committed)
  release_version.py --selftest

Every nightly build gets the next version, as Semantic Versioning has
it: MAJOR.MINOR come from the workspace version in Cargo.toml (the
maintainer bumps them), and PATCH is 0 for the first build of a
MAJOR.MINOR and one more than the last release tag's after that
(v0.1.0, v0.1.1, …). A version that isn't above every release tag is
refused, so versions only ever grow. A build promoted to beta or stable
keeps its version: nothing is rebuilt.

PATCH was once the nightly's run number, which never resets: the first
0.1 build came out as 0.1.18 (2026-10-23).
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
# An internal dependency: `{ version = "X.Y.Z", path = "../crate" }`, in
# either order. Its requirement must follow the stamped version, or Cargo
# can't resolve the workspace (the first nightly after 0.0.1 failed so).
PATH_DEP_VERSION = (
    re.compile(r'(version\s*=\s*")[^"]*("\s*,\s*path\s*=)'),
    re.compile(r'(path\s*=\s*"[^"]*"\s*,\s*version\s*=\s*")[^"]*(")'),
)


def base(root: Path) -> tuple[int, int]:
    match = WORKSPACE_VERSION.search((root / CARGO).read_text())
    if not match:
        raise SystemExit("Cargo.toml has no workspace version")
    return int(match.group(2)), int(match.group(3))


RELEASE_TAG = re.compile(r"v(\d+)\.(\d+)\.(\d+)")


def next_version(root: Path, tags: list[str]) -> str:
    """The next version after `tags` (git tag names; anything but
    vX.Y.Z, such as a channel's tag, is ignored)."""
    major, minor = base(root)
    released = [tuple(int(part) for part in match.groups()) for tag in tags if (match := RELEASE_TAG.fullmatch(tag.strip()))]
    patches = [patch for (x, y, patch) in released if (x, y) == (major, minor)]
    version = (major, minor, max(patches) + 1 if patches else 0)
    newest = max(released, default=None)
    if newest is not None and version <= newest:
        raise SystemExit(
            f"{'.'.join(map(str, version))} isn't above v{'.'.join(map(str, newest))}: "
            "raise the workspace version in Cargo.toml"
        )
    return ".".join(map(str, version))


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
    for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        text = manifest.read_text()
        for pattern in PATH_DEP_VERSION:
            text = pattern.sub(lambda m: f"{m.group(1)}{version}{m.group(2)}", text)
        manifest.write_text(text)
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
        (root / "crates/app").mkdir(parents=True)
        (root / "crates/app/Cargo.toml").write_text(
            '[package]\nname = "app"\nversion.workspace = true\n\n[dependencies]\n'
            'core = { version = "0.3.1", path = "../core" }\n'
            'link = { path = "../link", version = "0.3.1" }\n'
            'serde = { version = "1.0.200", features = ["derive"] }\n'
        )
        assert next_version(root, []) == "0.3.0", "the first build of a MAJOR.MINOR is .0"
        assert next_version(root, ["v0.2.17", "v0.2.9"]) == "0.3.0", "PATCH resets when MINOR goes up"
        assert next_version(root, ["v0.2.17", "v0.3.0", "v0.3.4\n", "v0.3.2"]) == "0.3.5"
        assert next_version(root, ["nightly-channel", "beta-channel", "v0.3.9-rc.1", "0.3.7", "v0.3.1"]) == "0.3.2", "only vX.Y.Z counts"
        for ahead in (["v0.4.0"], ["v1.0.0"], ["v0.3.1", "v0.10.2"]):
            try:
                next_version(root, ahead)
            except SystemExit:
                continue
            raise AssertionError(f"went below {ahead!r}")
        stamp(root, "0.3.57")
        assert 'version = "0.3.57"' in (root / CARGO).read_text()
        app = (root / "crates/app/Cargo.toml").read_text()
        assert 'core = { version = "0.3.57", path = "../core" }' in app, app
        assert 'link = { path = "../link", version = "0.3.57" }' in app, app
        assert 'serde = { version = "1.0.200"' in app, "a registry dependency is left alone"
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
    if args == ["next"]:
        print(next_version(ROOT, sys.stdin.read().split()))
        return 0
    if len(args) == 2 and args[0] == "stamp":
        stamp(ROOT, args[1])
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
