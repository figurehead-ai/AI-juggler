#!/usr/bin/env python3
"""AJ-E2010: AI Juggler's own helpers start with only what they need.

A process AI Juggler starts for itself goes through
`juggler_platform::process::helper` (re-exported as
`juggler_adapter::process`): an absolute program, the environment cleared
to the allowlist, the system PATH. The only other places that may call
`Command::new` are the ones listed in scripts/spawn.allow, each with its
reason: the user's own tools (an agent session, their editor, `gh`, a
vendor CLI) run in the user's login environment by design, and the app
relaunching itself is the app.

Test modules (`#[cfg(test)] mod tests`) and `src/bin/` fixtures are not
checked. Usage: spawn_lint.py [--selftest]
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts/spawn.allow"
SPAWN = re.compile(r"\bCommand::new\(")


def production(text: str) -> str:
    """`text` up to its test module."""
    m = re.search(r"#\[cfg\(test\)\]\s*\n\s*(?:pub\s+)?mod \w+", text)
    return text if m is None else text[: m.start()]


def sites(root: pathlib.Path) -> list[str]:
    found = []
    for path in sorted(root.glob("crates/*/src/**/*.rs")):
        if "/src/bin/" in str(path):
            continue
        text = production(path.read_text())
        for m in SPAWN.finditer(text):
            line = text.count("\n", 0, m.start()) + 1
            found.append(f"{path.relative_to(root)}:{line}")
    return found


def allowed() -> set[str]:
    return {
        line.split("#")[0].strip()
        for line in ALLOW.read_text().splitlines()
        if line.split("#")[0].strip()
    }


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        assert production("a\n#[cfg(test)]\nmod tests {\nCommand::new(x)\n}") == "a\n"
        assert SPAWN.search("std::process::Command::new(p)")
        assert not SPAWN.search("process::helper(p)")
        print("spawn_lint: selftest ok")
        return 0
    ok = allowed()
    problems = [site for site in sites(ROOT) if site.split(":")[0] not in ok]
    for site in problems:
        print(f"spawn_lint: {site}: a process started outside the helper seam — use juggler_adapter::process::helper, or list the file in scripts/spawn.allow with why it runs in the user's environment")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
