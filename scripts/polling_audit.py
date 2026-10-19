#!/usr/bin/env python3
"""AJ-E805 polling audit (SPEC: idle CPU stays near zero; no busy loops).

Every repeating timer (`setInterval`) or sleep shorter than one second in shipped code must say
why, in a `poll:` note on the same line or within the three lines above
it — what the wait serves and what bounds it (a user action, a retry
count, a deadline). Test code, fixtures and the fake harnesses are
exempt. Intervals of a second or more are listed, not judged.

Exit 1 when an unexplained sub-second wait is found.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UI = re.compile(r"\bsetInterval\(\s*[^,]+,\s*([0-9_]+)\s*\)")
RUST = re.compile(r"(?:sleep|interval)\(\s*(?:[\w:]+::)?Duration::from_(millis|secs)\(\s*([0-9_]+)\s*\)")
EXEMPT = ("/tests/", "/bin/fake_", ".test.", ".spec.", "/fixtures")


def shipped(path: Path) -> bool:
    text = str(path)
    return not any(part in text for part in EXEMPT)


def rust_test_start(lines: list[str]) -> int:
    """Index of the `#[cfg(test)]` module, so its waits are skipped."""
    for i, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]":
            return i
    return len(lines)


def explained(lines: list[str], i: int) -> bool:
    return any("poll:" in line for line in lines[max(0, i - 3) : i + 1])


def main() -> int:
    findings: list[tuple[str, int, int, bool]] = []
    for path in sorted((ROOT / "ui/src").rglob("*.ts*")):
        if not shipped(path):
            continue
        lines = path.read_text().splitlines()
        for i, line in enumerate(lines):
            for match in UI.finditer(line):
                ms = int(match.group(1).replace("_", ""))
                findings.append((str(path.relative_to(ROOT)), i + 1, ms, explained(lines, i)))
    for path in sorted(ROOT.glob("crates/*/src/**/*.rs")):
        if not shipped(path):
            continue
        lines = path.read_text().splitlines()
        end = rust_test_start(lines)
        for i, line in enumerate(lines[:end]):
            for match in RUST.finditer(line):
                n = int(match.group(2).replace("_", ""))
                ms = n if match.group(1) == "millis" else n * 1000
                findings.append((str(path.relative_to(ROOT)), i + 1, ms, explained(lines, i)))
    bad = [f for f in findings if f[2] < 1000 and not f[3]]
    for file, line, ms, ok in findings:
        if ms < 1000:
            mark = "ok  " if ok else "FAIL"
            print(f"{mark} {ms:>6} ms  {file}:{line}")
    slow = sum(1 for f in findings if f[2] >= 1000)
    print(f"polling audit: {len(findings)} timers/sleeps, {len(findings) - slow} under 1 s, {len(bad)} unexplained")
    if bad:
        print("Add a `poll:` note saying what each wait serves and what bounds it.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
