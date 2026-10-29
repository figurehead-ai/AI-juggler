#!/usr/bin/env python3
"""D-20: what ships in a release, so a night with nothing new builds nothing.

  release_ships.py newest < TAGS        print the newest release tag (vX.Y.Z)
                                         from the tags on stdin, or nothing
  release_ships.py filter < PATHS       print the paths (one per line, as
                                         `git diff --name-only` lists them)
                                         that change what ships
  release_ships.py explain PATH...      print each path's rule and reason
  release_ships.py --selftest

The scheduled nightly compares main with the newest release tag and skips
when no changed file ships (`.github/workflows/nightly.yml`). Twice a build
identical to the one before took the number the next real release needed:
v0.1.5 (2026-10-26) changed only an integration test, v0.1.7 (2026-10-27)
only release notes. A run started by hand always builds.

RULES below is the only list. The first rule a path matches decides; a
path no rule matches ships, so a file nobody classified builds rather than
being skipped. The self-test fails until every tracked file matches a rule,
and checks the rules against the tree: the app includes no file that
doesn't ship, every script the build workflows run ships, and every in-source
`tests.rs` is compiled for tests only.
"""
from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RELEASE_TAG = re.compile(r"v(\d+)\.(\d+)\.(\d+)")

# The workflows that build, sign and publish a release.
BUILD_WORKFLOWS = (".github/workflows/build-macos.yml", ".github/workflows/nightly.yml")

# (pattern, ships, why). `**` crosses folders, `*` doesn't; first match wins.
RULES: tuple[tuple[str, bool, str], ...] = (
    # Tests and their data inside the folders that ship.
    ("crates/*/tests/**", False, "integration tests and fixtures: only `cargo test` builds them"),
    ("crates/*/examples/**", False, "examples: only `cargo run --example` builds them"),
    ("crates/*/benches/**", False, "benchmarks: only `cargo bench` builds them"),
    ("crates/*/src/**/tests.rs", False, "unit tests, declared `#[cfg(test)] mod tests;` (the self-test checks)"),
    ("ui/tests/**", False, "Playwright specs, visual baselines and the bundle checks"),
    ("ui/src/**/*.test.ts", False, "Vitest tests"),
    ("ui/src/**/*.test.tsx", False, "Vitest tests"),
    ("ui/src/test/**", False, "helpers only the tests import (the self-test checks)"),
    ("ui/playwright*.config.ts", False, "Playwright's configuration"),
    ("ui/vitest.config.ts", False, "Vitest's configuration"),
    ("**/*.md", False, "documentation: the app includes none (the self-test checks)"),
    # The app.
    ("crates/**", True, "the app: Rust code, build scripts, the Tauri configuration, capabilities, icons, Info.plist, entitlements"),
    ("ui/**", True, "the interface `pnpm build` bundles, with its packages and lockfile"),
    ("Cargo.toml", True, "the workspace: its version (MAJOR.MINOR), members and dependencies"),
    ("Cargo.lock", True, "the exact dependencies the build compiles"),
    ("rust-toolchain.toml", True, "the compiler that builds the app"),
    # How it's built, signed and published.
    (".github/workflows/build-macos.yml", True, "builds and signs the app"),
    (".github/workflows/nightly.yml", True, "plans and publishes the build, and its update feeds"),
    ("scripts/release_version.py", True, "stamps the version into the build"),
    ("scripts/release_ships.py", True, "decides whether the nightly builds"),
    ("scripts/install_tauri_cli.sh", True, "the pinned Tauri CLI that builds and signs the updater archive"),
    ("scripts/edition_icon.swift", True, "draws AI Juggler Nightly's icon"),
    ("scripts/sign_macos.sh", True, "signs, notarizes and staples the app; makes the updater archive"),
    ("scripts/release_seams_check.sh", True, "checks the signed app before it is published"),
    ("scripts/apple_retry.py", True, "retries Apple's transient failures while signing"),
    ("scripts/updater_sig_check.py", True, "checks the updater signature before it is published"),
    ("scripts/nightly_notes.py", True, "writes the release notes the feed carries"),
    ("scripts/downgrade_note.py", True, "adds the downgrade note to those notes"),
    ("scripts/release_notes_check.sh", True, "checks those notes before the build"),
    # Everything else is for people, CI and the gates.
    ("docs/**", False, "documentation, including docs/releases (a version's notes, picked up by its build)"),
    (".github/**", False, "CI, the other workflows, issue and PR templates, Dependabot"),
    ("scripts/**", False, "gates and developer tools"),
    ("fuzz/**", False, "fuzz targets: never part of the app"),
    ("stats/**", False, "the usage-statistics schema: the app has juggler-stats' generated copy, which a gate keeps equal"),
    ("justfile", False, "developer tasks"),
    ("deny.toml", False, "cargo-deny's policy (a CI gate)"),
    ("rustfmt.toml", False, "formatting"),
    (".editorconfig", False, "editor settings"),
    (".gitignore", False, "git"),
    ("LICENSE", False, "the licence: the app bundle doesn't carry it"),
)


def _compile(pattern: str) -> re.Pattern[str]:
    out, i = "", 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out, i = out + "(?:.*/)?", i + 3
        elif pattern.startswith("**", i):
            out, i = out + ".*", i + 2
        elif pattern[i] == "*":
            out, i = out + "[^/]*", i + 1
        else:
            out, i = out + re.escape(pattern[i]), i + 1
    return re.compile(out + r"\Z")


_COMPILED = tuple((_compile(pattern), pattern, ships, why) for pattern, ships, why in RULES)


def rule(path: str) -> tuple[str, bool, str] | None:
    """The first rule `path` (relative to the repository, `/`-separated)
    matches, or None."""
    for regex, pattern, ships, why in _COMPILED:
        if regex.match(path):
            return pattern, ships, why
    return None


def ships(path: str) -> bool:
    found = rule(path)
    return True if found is None else found[1]


def newest(tags: list[str]) -> str:
    """The newest vX.Y.Z among `tags` (anything else is ignored), or ''."""
    released = [(tuple(int(part) for part in match.groups()), match.group(0)) for tag in tags if (match := RELEASE_TAG.fullmatch(tag.strip()))]
    return max(released)[1] if released else ""


# Guards: the rules against the tree as it is.

INCLUDE = re.compile(r'include_(?:str|bytes)!\(\s*"([^"]+)"')
IMPORT = re.compile(r"""(?:\bfrom\s+|\bimport\s*\(\s*|\bimport\s+)["'](\.{1,2}/[^"'?]+)""")
MOD_TESTS = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?mod tests;", re.M)
CFG_TEST_MOD_TESTS = re.compile(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod tests;")
UI_SUFFIXES =("", ".ts", ".tsx", ".js", ".json", "/index.ts", "/index.tsx")


def _resolve(base: Path, target: str, root: Path) -> str | None:
    path = (base / target).resolve()
    try:
        return path.relative_to(root.resolve()).as_posix()
    except ValueError:
        return None


def problems(root: Path, files: list[str]) -> list[str]:
    """What's wrong with RULES for the tracked `files` under `root`."""
    found = []
    tracked = set(files)
    for path in files:
        if rule(path) is None:
            found.append(f"{path}: no rule says whether it ships (add one to RULES)")
    for path in files:
        if not ships(path):
            continue
        if path.startswith("crates/") and path.endswith(".rs"):
            for target in INCLUDE.findall((root / path).read_text(errors="replace")):
                included = _resolve((root / path).parent, target, root)
                if included is not None and not ships(included):
                    found.append(f"{path} includes {included}, which RULES says doesn't ship")
        if path.startswith("ui/src/") and path.endswith((".ts", ".tsx")):
            for target in IMPORT.findall((root / path).read_text(errors="replace")):
                for suffix in UI_SUFFIXES:
                    imported = _resolve((root / path).parent, target + suffix, root)
                    if imported in tracked:
                        if not ships(imported):
                            found.append(f"{path} imports {imported}, which RULES says doesn't ship")
                        break
    scripts = {Path(p).name: p for p in files if p.startswith("scripts/") and p.count("/") == 1}
    for workflow in BUILD_WORKFLOWS:
        if workflow not in tracked:
            continue
        queue = [workflow]
        seen = set(queue)
        while queue:
            text = (root / queue.pop()).read_text(errors="replace")
            for name, script in scripts.items():
                if script not in seen and re.search(rf"(?<![\w.-]){re.escape(name)}(?![\w-])", text):
                    seen.add(script)
                    queue.append(script)
                    if not ships(script):
                        found.append(f"{workflow} runs {script}, which RULES says doesn't ship")
    for path in files:
        if not (path.startswith("crates/") and path.endswith("/tests.rs") and "/src/" in path):
            continue
        folder = Path(path).parent
        parents = [folder.with_suffix(".rs"), folder / "mod.rs"] if folder.name != "src" else [folder / "lib.rs", folder / "main.rs"]
        parent = next((p for p in parents if p.as_posix() in tracked), None)
        text = (root / parent).read_text() if parent else ""
        declared = len(MOD_TESTS.findall(text))
        if declared == 0 or declared != len(CFG_TEST_MOD_TESTS.findall(text)):
            found.append(f"{path} isn't declared `#[cfg(test)] mod tests;`, so it may ship")
    return found


def selftest() -> int:
    cases = {
        "crates/juggler-app/src/main.rs": True,
        "crates/juggler-app/tauri.conf.json": True,
        "crates/juggler-app/icons/icon.png": True,
        "crates/juggler-link/proto/link.proto": True,
        "crates/juggler-app/tests/second_launch.rs": False,  # v0.1.5
        "crates/juggler-core/tests/fixtures/a/b.json": False,
        "crates/juggler-handover/examples/pack.rs": False,
        "crates/juggler-app/src/features/handover/tests.rs": False,
        "crates/juggler-app/src/features/handover/tests_extra.rs": True,
        "crates/juggler-app/icons/NOTICE.md": False,
        "ui/src/App.tsx": True,
        "ui/src/lib/trust-claims.json": True,
        "ui/src/app-keys.test.tsx": False,
        "ui/src/lib/x.test.ts": False,
        "ui/src/test/cpuTime.ts": False,
        "ui/src/testing.ts": True,
        "ui/tests/visual/__baselines__/home.png": False,
        "ui/playwright.regression.config.ts": False,
        "ui/vite.config.ts": True,
        "ui/package.json": True,
        "ui/pnpm-lock.yaml": True,
        "Cargo.lock": True,
        "Cargo.toml": True,
        "rust-toolchain.toml": True,
        "docs/releases/0.1.6.md": False,  # v0.1.7
        "CHANGELOG.md": False,
        "README.md": False,
        ".github/workflows/nightly.yml": True,
        ".github/workflows/build-macos.yml": True,
        ".github/workflows/ci.yml": False,
        ".github/PULL_REQUEST_TEMPLATE.md": False,
        "scripts/sign_macos.sh": True,
        "scripts/visual_gate.sh": False,
        "fuzz/fuzz_targets/link.rs": False,
        "stats/schema.v1.json": False,
        "justfile": False,
        "something/new.txt": True,  # unclassified: builds
    }
    for path, expected in cases.items():
        assert ships(path) is expected, f"{path}: ships={ships(path)}, expected {expected}"
    assert rule("something/new.txt") is None

    assert newest(["nightly-channel", "beta/v0.1.6", "v0.1.9", "v0.1.10\n", "v0.2.0-rc.1", "0.3.0"]) == "v0.1.10"
    assert newest(["stable-channel"]) == ""

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        tree = {
            ".github/workflows/nightly.yml": "run: python3 scripts/release_version.py next\n",
            ".github/workflows/build-macos.yml": "run: ./scripts/sign_macos.sh\n",
            "scripts/release_version.py": "",
            "scripts/sign_macos.sh": "python3 \"$here/apple_retry.py\"\n",
            "scripts/apple_retry.py": "",
            "scripts/visual_gate.sh": "",
            "crates/a/src/lib.rs": 'const A: &str = include_str!("data.json");\n#[cfg(test)]\nmod tests;\n',
            "crates/a/src/data.json": "{}",
            "crates/a/src/tests.rs": "",
            "ui/src/main.tsx": 'import { App } from "./App";\nimport "./styles.css";\n',
            "ui/src/App.tsx": "",
            "ui/src/styles.css": "",
            "docs/x.md": "",
        }
        for path, text in tree.items():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text(text)
        assert problems(root, sorted(tree)) == [], problems(root, sorted(tree))

        def broken(path: str, text: str, expect: str) -> None:
            files = sorted({*tree, path})
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text(text)
            try:
                found = problems(root, files)
                assert any(expect in line for line in found), (path, found)
            finally:
                if path in tree:
                    (root / path).write_text(tree[path])
                else:
                    (root / path).unlink()

        broken("crates/a/src/lib.rs", 'const A: &str = include_str!("../../../docs/x.md");\n#[cfg(test)]\nmod tests;\n', "includes docs/x.md")
        (root / "ui/src/test").mkdir()
        (root / "ui/src/test/cpu.ts").write_text("")
        tree["ui/src/test/cpu.ts"] = ""
        broken("ui/src/main.tsx", 'import { cpu } from "./test/cpu";\n', "imports ui/src/test/cpu.ts")
        broken("scripts/sign_macos.sh", "./visual_gate.sh\n", "runs scripts/visual_gate.sh")
        broken("crates/a/src/lib.rs", "mod tests;\n", "isn't declared")
        broken("crates/a/src/lib.rs", "#[cfg(test)]\nmod tests;\n#[cfg(feature = \"x\")]\nmod tests;\n", "isn't declared")
        broken("assets/logo.svg", "", "assets/logo.svg: no rule")

    try:
        files = subprocess.run(["git", "ls-files"], cwd=ROOT, check=True, capture_output=True, text=True).stdout.split("\n")
    except (OSError, subprocess.CalledProcessError):
        files = []
    files = [path for path in files if path and (ROOT / path).is_file()]
    if files:
        found = problems(ROOT, files)
        assert not found, "RULES don't fit the repository:\n  " + "\n  ".join(found)
        print(f"release-ships selftest: OK ({len(files)} tracked files classified)")
    else:
        print("release-ships selftest: OK (not in a git checkout: the tree wasn't checked)")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    if args == ["newest"]:
        print(newest(sys.stdin.read().split()))
        return 0
    if args == ["filter"]:
        for line in sys.stdin.read().splitlines():
            if line.strip() and ships(line.strip()):
                print(line.strip())
        return 0
    if len(args) >= 2 and args[0] == "explain":
        for path in args[1:]:
            found = rule(path)
            print(f"{path}: " + (f"{'ships' if found[1] else 'does not ship'} ({found[0]}: {found[2]})" if found else "ships (no rule; add one)"))
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
