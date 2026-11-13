#!/usr/bin/env python3
"""D-20 release versions: one stream, channels are pointers.

  release_version.py next < TAGS        print the next MAJOR.MINOR.PATCH,
                                         from the release tags on stdin
  release_version.py stamp X.Y.Z         write X.Y.Z into the workspace
                                         Cargo.toml, every crate's internal
                                         dependencies, tauri.conf.json and
                                         the Claude Code plugin's plugin.json
                                         (CI checkout only, never committed)
  release_version.py --selftest

`semver(text)` parses a strict Semantic Versioning 2.0.0 version into a
key that sorts by SemVer precedence (other scripts compare with it).

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
# AJ-E1905: the plugin is released with the app and says which version.
PLUGIN = Path("plugins/claude-code/ai-juggler/.claude-plugin/plugin.json")
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

# semver.org's own pattern: no leading zeros, no "v", all three numbers.
SEMVER = re.compile(
    r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?"
    r"(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"
)


def semver(text: str) -> tuple:
    """A strict SemVer version as a key in precedence order (spec item
    11): the numbers numerically; a pre-release below its release; its
    identifiers numeric below alphanumeric, a shorter set below a longer
    one; build metadata ignored. Raises ValueError for anything else."""
    match = SEMVER.fullmatch(text) if isinstance(text, str) else None
    if match is None:
        raise ValueError(f"not a SemVer version: {text!r}")
    major, minor, patch, pre, _build = match.groups()
    if pre is None:
        rank: tuple = (1,)
    else:
        rank = (0, tuple((0, int(part), "") if part.isdigit() else (1, 0, part) for part in pre.split(".")))
    return (int(major), int(minor), int(patch), rank)


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
    plugin = root / PLUGIN
    if plugin.exists():
        manifest = json.loads(plugin.read_text())
        manifest["version"] = version
        plugin.write_text(json.dumps(manifest, indent=2) + "\n")


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / TAURI).parent.mkdir(parents=True)
        (root / CARGO).write_text('[workspace]\nmembers = []\n\n[workspace.package]\nversion = "0.3.1"\nedition = "2024"\n')
        (root / TAURI).write_text(json.dumps({"productName": "AI Juggler", "version": "0.3.1"}))
        (root / PLUGIN).parent.mkdir(parents=True)
        (root / PLUGIN).write_text(json.dumps({"name": "ai-juggler", "version": "0.3.1"}))
        (root / "crates/app").mkdir(parents=True)
        (root / "crates/app/Cargo.toml").write_text(
            '[package]\nname = "app"\nversion.workspace = true\n\n[dependencies]\n'
            'core = { version = "0.3.1", path = "../core" }\n'
            'link = { path = "../link", version = "0.3.1" }\n'
            'serde = { version = "1.0.200", features = ["derive"] }\n'
        )
        ordered = ["0.9.0", "0.10.0-alpha", "0.10.0-alpha.1", "0.10.0-alpha.beta", "0.10.0-beta.2", "0.10.0-beta.11", "0.10.0-rc.1", "0.10.0", "1.0.0"]
        assert sorted(reversed(ordered), key=semver) == ordered, "SemVer precedence, not text order"
        assert semver("0.4.1+build.7") == semver("0.4.1"), "build metadata doesn't count"
        for bad in ("v0.4.1", "0.4", "01.2.3", "0.4.1-", "0.4.1-01", " 0.4.1", "", None):
            try:
                semver(bad)  # type: ignore[arg-type]
            except ValueError:
                continue
            raise AssertionError(f"accepted {bad!r}")
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
        assert json.loads((root / PLUGIN).read_text()) == {"name": "ai-juggler", "version": "0.3.57"}, "the plugin says the release it came with"
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
