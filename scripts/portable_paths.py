#!/usr/bin/env python3
"""Every tracked path checks out on every OS the app ships to (D-52).

Windows refuses a path with any of < > : " | ? * or a backslash, a
control character, a name ending in a dot or a space, or a reserved
device name (CON, PRN, AUX, NUL, COM1-9, LPT1-9, with or without an
extension); and Windows and macOS fold case, so two paths that differ
only in case collide. build-windows' checkout failed on six evidence
files named `...attention:facts_given.json` (0.5's release PR, 2026-11-20).

    portable_paths.py              # check `git ls-files`; exit 1 on any
    portable_paths.py --selftest
"""
from __future__ import annotations

import re
import subprocess
import sys

BAD_CHARS = re.compile(r'[<>:"|?*\\\x00-\x1f]')
RESERVED = re.compile(r"^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(\..*)?$", re.IGNORECASE)


def problems(paths: list[str]) -> list[str]:
    out: list[str] = []
    folded: dict[str, str] = {}
    for path in paths:
        for part in path.split("/"):
            if BAD_CHARS.search(part):
                out.append(f"{path}: '{part}' holds a character Windows refuses")
            elif part.endswith((".", " ")):
                out.append(f"{path}: '{part}' ends in a dot or a space")
            elif RESERVED.match(part):
                out.append(f"{path}: '{part}' is a Windows device name")
        key = path.lower()
        if key in folded and folded[key] != path:
            out.append(f"{path}: differs only in case from {folded[key]}")
        folded.setdefault(key, path)
    return out


def selftest() -> None:
    assert problems(["docs/a.json", "src/Lib.rs", "x/y-z_1.md"]) == []
    assert problems(["e/candidates-v4-attention:facts_given.json"])
    for bad in ["a/b?.md", "a/b*.md", 'a/"q".md', "a/b|c", "a/b<c", "a\\b", "a/b.", "a/b ", "a/CON", "a/nul.txt", "a/COM1.log"]:
        assert problems([bad]), bad
    assert problems(["a/Readme.md", "a/README.md"])
    assert problems(["a/console.md", "a/null.md", "a/com10.md"]) == []
    print("portable-paths: selftest ok")


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        return 0
    paths = subprocess.run(
        ["git", "ls-files", "-z"], check=True, capture_output=True
    ).stdout.decode().split("\0")
    found = problems([p for p in paths if p])
    for line in found:
        print(f"portable-paths: {line}")
    if found:
        return 1
    print(f"portable-paths: OK ({len(paths) - 1} tracked paths)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
