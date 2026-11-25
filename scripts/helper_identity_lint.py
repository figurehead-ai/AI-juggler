#!/usr/bin/env python3
"""AJ-E3822: every helper's identifier follows the build's identity, and
every helper names itself.

The 0.5.0 Nightly's `juggler-apple-ai` was `ai.figurehead.juggler.apple-ai`
signed with the team's Developer ID; Dev and lab builds signed the same
identifier ad hoc. macOS's app-data protection then asked '"(null)" differs
from previously opened versions': one identifier, two signers, and a
helper plist with no display name. So:

1. No helper's full identifier is written out in code, scripts, plists or
   workflows (comments and test code aside): it is always the identity's
   prefix and the helper's suffix, at build time from
   scripts/helper_identity.sh (AIJ_EDITION) and at run time from
   `Edition::helper_identifier`. The helpers are tauri.bundle.json's
   externalBin (`juggler-<suffix>`), so a later one is covered too.
2. A script's `identifier=` is `$(helper_identifier …)` or
   `$(helper_identifier_for_app …)`, never a literal.
3. The prefixes agree: helper_identity.sh's release prefix is the main
   edition's identifier and its Dev prefix Dev's (edition.rs), as is the
   local helper's build.rs default.
4. Every helper Info.plist (crates/**/*Info.plist but the app's own) has
   CFBundleIdentifier `@HELPER_IDENTIFIER@` (written in at build time) and
   CFBundleName and CFBundleDisplayName "AI Juggler"; whatever embeds one
   (`__info_plist`) writes the identifier in (helper_plist, or build.rs's
   placeholder).
5. Every helper is signed through the identity somewhere in scripts/
   (`helper_identifier <suffix>`).

Usage: helper_identity_lint.py [--selftest]
"""
from __future__ import annotations

import json
import plistlib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_tests  # noqa: E402  (one test-code finder for every lint, D-94)

NAME = "AI Juggler"
PLACEHOLDER = "@HELPER_IDENTIFIER@"
APP_PLIST = "crates/juggler-app/Info.plist"
IDENTITY = "scripts/helper_identity.sh"
SELF = "scripts/helper_identity_lint.py"


def suffixes(root: Path) -> list[str]:
    """The helpers' identifier suffixes, from the bundle's helper list."""
    bundle = json.loads((root / "crates/juggler-app/tauri.bundle.json").read_text())
    names = [Path(b).name for b in bundle["bundle"]["externalBin"]]
    return [n.removeprefix("juggler-") for n in names]


def literal(helpers: list[str]) -> re.Pattern[str]:
    """A helper's full identifier written out, under any edition."""
    alts = "|".join(re.escape(h) for h in helpers)
    return re.compile(r"ai\.figurehead\.juggler(?:\.dev|\.nightly)?\.(?:" + alts + r")(?![\w-])")


def is_comment(line: str, kind: str) -> bool:
    stripped = line.lstrip()
    if kind == "rs":
        return stripped.startswith("//")
    return stripped.startswith("#")


def literal_sites(text: str, kind: str, pattern: re.Pattern[str]) -> list[int]:
    """Lines (1-based) that write a helper's identifier out, comments aside."""
    if kind == "rs":
        text = rust_tests.blank_tests(text)
    if kind == "plist":
        text = re.sub(r"<!--.*?-->", lambda m: re.sub(r"[^\n]", " ", m.group()), text, flags=re.S)
    return [
        i
        for i, line in enumerate(text.splitlines(), 1)
        if pattern.search(line) and not (kind != "plist" and is_comment(line, kind))
    ]


IDENT_ASSIGN = re.compile(r"^\s*(?:local\s+)?identifier=(.*)$")


def assignment_sites(text: str) -> list[int]:
    """A script's `identifier=` that isn't the identity's."""
    bad = []
    for i, line in enumerate(text.splitlines(), 1):
        m = IDENT_ASSIGN.match(line)
        if m and not re.match(r'"?\$\(helper_identifier(?:_for_app)? ', m.group(1)):
            bad.append(i)
    return bad


def plist_problems(data: bytes) -> list[str]:
    try:
        plist = plistlib.loads(data)
    except Exception as error:  # noqa: BLE001  (any parse failure is a finding)
        return [f"not a plist ({error})"]
    out = []
    if plist.get("CFBundleIdentifier") != PLACEHOLDER:
        out.append(f"CFBundleIdentifier is {plist.get('CFBundleIdentifier')!r}, not {PLACEHOLDER} (the identity's, written in at build time)")
    for key in ("CFBundleName", "CFBundleDisplayName"):
        if plist.get(key) != NAME:
            out.append(f"{key} is {plist.get(key)!r}, not {NAME!r} (a system prompt says \"(null)\")")
    return out


def prefix_problems(identity: str, edition: str, build_rs: str) -> list[str]:
    out = []
    main = re.search(r'pub const MAIN_IDENTIFIER: &str = "([^"]+)";', edition)
    dev = re.search(r'fn identifier\(self\)[^}]*?Self::Dev => "([^"]+)"', edition, re.S)
    release_prefix = re.search(r"^AIJ_RELEASE_PREFIX=(\S+)$", identity, re.M)
    dev_prefix = re.search(r"^AIJ_DEV_PREFIX=(\S+)$", identity, re.M)
    build_default = re.search(r'const DEV_PREFIX: &str = "([^"]+)";', build_rs)
    if not (main and dev):
        return ["edition.rs: can't read MAIN_IDENTIFIER and Dev's identifier"]
    if not (release_prefix and dev_prefix):
        return [f"{IDENTITY}: can't read AIJ_RELEASE_PREFIX and AIJ_DEV_PREFIX"]
    if release_prefix.group(1) != main.group(1):
        out.append(f"{IDENTITY}: AIJ_RELEASE_PREFIX {release_prefix.group(1)} isn't the main edition's {main.group(1)}")
    if dev_prefix.group(1) != dev.group(1):
        out.append(f"{IDENTITY}: AIJ_DEV_PREFIX {dev_prefix.group(1)} isn't Dev's {dev.group(1)}")
    if not build_default or build_default.group(1) != dev.group(1):
        out.append(f"crates/juggler-local-ai/build.rs: DEV_PREFIX isn't Dev's {dev.group(1)}")
    if not re.search(r"Self::Main \| Self::Nightly => MAIN_IDENTIFIER,\s*Self::Dev => self\.identifier\(\),", edition):
        out.append("edition.rs: helper_identifier no longer gives Main and Nightly AI Juggler's prefix and Dev its own")
    return out


def files(root: Path, pattern: str) -> list[Path]:
    return sorted(p for p in root.glob(pattern) if "/target/" not in p.as_posix() and "/node_modules/" not in p.as_posix())


def scan(root: Path) -> list[str]:
    helpers = suffixes(root)
    pattern = literal(helpers)
    tests = rust_tests.test_files(root)
    out: list[str] = []

    # 1. No helper identifier written out.
    sources: list[tuple[Path, str]] = []
    sources += [(p, "sh") for p in files(root, "scripts/**/*.sh")]
    sources += [(p, "sh") for p in files(root, "scripts/**/*.py") if p.relative_to(root).as_posix() != SELF]
    sources += [(p, "sh") for p in files(root, ".github/workflows/*.yml")]
    sources += [(root / "justfile", "sh")]
    sources += [(p, "plist") for p in files(root, "crates/**/*.plist")]
    sources += [(p, "plist") for p in files(root, "crates/**/*.entitlements")]
    for path in files(root, "crates/**/*.rs"):
        rel = path.relative_to(root).as_posix()
        if "/tests/" in rel or rel.endswith("_tests.rs") or rel in tests:
            continue
        sources.append((path, "rs"))
    for path, kind in sources:
        if not path.exists():
            continue
        for line in literal_sites(path.read_text(errors="replace"), kind, pattern):
            out.append(f"{path.relative_to(root)}:{line}: a helper's identifier written out; take it from the build's identity ({IDENTITY}, Edition::helper_identifier)")

    # 2. Scripts assign the identity's identifier.
    for path in files(root, "scripts/**/*.sh"):
        for line in assignment_sites(path.read_text()):
            out.append(f"{path.relative_to(root)}:{line}: identifier= must be $(helper_identifier …) or $(helper_identifier_for_app …)")

    # 3. The prefixes agree.
    out += prefix_problems(
        (root / IDENTITY).read_text(),
        (root / "crates/juggler-app/src/edition.rs").read_text(),
        (root / "crates/juggler-local-ai/build.rs").read_text(),
    )

    # 4. Every helper plist names itself and takes the identity's identifier.
    for path in files(root, "crates/**/*Info.plist"):
        if path.relative_to(root).as_posix() == APP_PLIST:
            continue
        for problem in plist_problems(path.read_bytes()):
            out.append(f"{path.relative_to(root)}: {problem}")
    for path in files(root, "scripts/**/*.sh") + files(root, "crates/**/build.rs"):
        text = path.read_text()
        if "__info_plist" not in text or path.relative_to(root).as_posix() == IDENTITY:
            continue
        if path.suffix == ".sh" and "helper_plist " not in text:
            out.append(f"{path.relative_to(root)}: embeds an Info.plist without helper_plist (the identity's identifier)")
        if path.suffix == ".rs" and PLACEHOLDER not in text:
            out.append(f"{path.relative_to(root)}: embeds an Info.plist without writing in {PLACEHOLDER}")

    # 5. Every helper is signed through the identity.
    scripts = "\n".join(p.read_text() for p in files(root, "scripts/**/*.sh"))
    for helper in helpers:
        if not re.search(r"helper_identifier(?:_for_app \S+)? " + re.escape(helper) + r"\b", scripts):
            out.append(f"juggler-{helper}: no script signs or checks it as the identity's (helper_identifier {helper})")
    return out


def selftest() -> None:
    pattern = literal(["agent", "apple-ai", "local-ai"])
    assert literal_sites('identifier=ai.figurehead.juggler.apple-ai\n', "sh", pattern) == [1]
    assert literal_sites('x\nid="ai.figurehead.juggler.dev.local-ai"\n', "sh", pattern) == [2]
    assert literal_sites('# signs it as ai.figurehead.juggler.apple-ai\n', "sh", pattern) == []
    assert literal_sites('ai.figurehead.juggler.dev.vault ai.figurehead.juggler.apple-aix\n', "sh", pattern) == []
    assert literal_sites('ai.figurehead.juggler.nightly.agent\n', "sh", pattern) == [1]
    assert literal_sites('const I: &str = "ai.figurehead.juggler.local-ai";\n', "rs", pattern) == [1]
    assert literal_sites('/// ai.figurehead.juggler.local-ai\nfn f() {}\n', "rs", pattern) == []
    assert literal_sites('fn f() {}\n#[cfg(test)]\nmod tests {\n    const I: &str = "ai.figurehead.juggler.local-ai";\n}\n', "rs", pattern) == []
    assert literal_sites('<!-- ai.figurehead.juggler.apple-ai -->\n<string>ai.figurehead.juggler.apple-ai</string>\n', "plist", pattern) == [2]

    assert assignment_sites('identifier=ai.figurehead.juggler.apple-ai\n') == [1]
    assert assignment_sites('identifier="$(helper_identifier apple-ai)"\n') == []
    assert assignment_sites('  if x; then identifier="$(helper_identifier_for_app "$app" local-ai)"; fi\n') == []
    assert assignment_sites('  identifier="$(helper_identifier_for_app "$app" local-ai)"\n') == []
    assert assignment_sites('  local identifier="ai.figurehead.juggler.x"\n') == [1]

    def plist(**keys: str) -> bytes:
        return plistlib.dumps(keys)

    assert plist_problems(plist(CFBundleIdentifier=PLACEHOLDER, CFBundleName=NAME, CFBundleDisplayName=NAME)) == []
    assert len(plist_problems(plist(CFBundleIdentifier="ai.figurehead.juggler.apple-ai", CFBundleName=NAME, CFBundleDisplayName=NAME))) == 1
    assert len(plist_problems(plist(CFBundleIdentifier=PLACEHOLDER, CFBundleName="juggler-apple-ai"))) == 2
    assert plist_problems(b"not a plist")[0].startswith("not a plist")

    edition = (
        'pub const MAIN_IDENTIFIER: &str = "ai.figurehead.juggler";\n'
        'pub fn identifier(self) -> &str { match self { Self::Main => MAIN_IDENTIFIER,\n'
        'Self::Dev => "ai.figurehead.juggler.dev", } }\n'
        'let prefix = match self { Self::Main | Self::Nightly => MAIN_IDENTIFIER,\n Self::Dev => self.identifier(), };\n'
    )
    identity = "AIJ_RELEASE_PREFIX=ai.figurehead.juggler\nAIJ_DEV_PREFIX=ai.figurehead.juggler.dev\n"
    build = 'const DEV_PREFIX: &str = "ai.figurehead.juggler.dev";\n'
    assert prefix_problems(identity, edition, build) == []
    assert len(prefix_problems(identity.replace(".dev\n", ".devx\n"), edition, build)) == 1
    assert len(prefix_problems(identity, edition, build.replace(".dev", ".lab"))) == 1
    assert len(prefix_problems(identity, edition.replace("self.identifier()", "MAIN_IDENTIFIER"), build)) == 1
    print("helper_identity_lint selftest: ok")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    found = scan(ROOT)
    for problem in found:
        print(f"helper_identity_lint: {problem}")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
