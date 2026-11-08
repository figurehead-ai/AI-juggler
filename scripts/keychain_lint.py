#!/usr/bin/env python3
"""0.4.1: the system keychain is reached through one door.

A lab, a replica, the Floor harness or a test run (a `HOME` that isn't the
user's, or `AIJ_NO_KEYCHAIN=1`) must never touch a keychain: macOS then
starts SecurityAgent and offers to create or reset one. So every keychain
use goes through code that honours that mode:

- `security_framework` (the Keychain APIs) only in
  `crates/juggler-account/src/secrets.rs`, whose stores the app routes to
  private files when the keychain is off (`edition::keep_secrets`);
- the `security` tool (`/usr/bin/security`, `find-generic-password` and the
  like) only in `crates/juggler-platform/src/keychain.rs`
  (`keychain::item_exists`, which runs nothing when off).

Fails naming each other place. Test files are left out.

Usage: keychain_lint.py [--selftest]
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import rust_tests  # noqa: E402  (one test-code finder for every lint, D-94)

RULES = [
    (re.compile(r"\bsecurity_framework\b|\bSecItem\w*|\bSecKeychain\w*"), {"crates/juggler-account/src/secrets.rs"},
     "the Keychain APIs: only through juggler-account's stores (secrets.rs)"),
    (re.compile(r"/usr/bin/security\b|\b(find|add|delete)-(generic|internet)-password\b|\bdump-keychain\b"),
     {"crates/juggler-platform/src/keychain.rs"},
     "the security tool: only through juggler_platform::keychain"),
]


def code_lines(text: str) -> list[tuple[int, str]]:
    """Lines outside the test modules (rust_tests), comments stripped."""
    return [(number, line.split("//", 1)[0]) for number, line in enumerate(rust_tests.blank_tests(text).splitlines(), 1)]


def findings(root: pathlib.Path) -> list[str]:
    out = []
    tests = rust_tests.test_files(root)
    for path in sorted((root / "crates").rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        if "/tests/" in rel or "/target/" in rel or "/examples/" in rel or "/benches/" in rel or rel in tests:
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for number, code in code_lines(text):
            for pattern, allowed, what in RULES:
                if rel not in allowed and pattern.search(code):
                    out.append(f"{rel}:{number}: {what}")
    return out


def selftest() -> None:
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)
        bad = root / "crates/x/src/lib.rs"
        bad.parent.mkdir(parents=True)
        bad.write_text('fn a() { let _ = std::process::Command::new("/usr/bin/security"); }\n'
                       "// security_framework in a comment is fine\n"
                       "#[cfg(test)]\nmod tests {\n    fn t() { security_framework::x(); }\n}\n")
        ok = root / "crates/juggler-platform/src/keychain.rs"
        ok.parent.mkdir(parents=True)
        ok.write_text('const SECURITY: &str = "/usr/bin/security";\n')
        found = findings(root)
        assert found == ["crates/x/src/lib.rs:1: the security tool: only through juggler_platform::keychain"], found
    print("keychain-lint: selftest ok")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    found = findings(ROOT)
    for line in found:
        print(f"keychain-lint: {line}", file=sys.stderr)
    if not found:
        print("keychain-lint: OK (the keychain is reached through juggler-account's stores and juggler_platform::keychain only)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
