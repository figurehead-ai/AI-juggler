#!/usr/bin/env python3
"""AJ-E2801 R5: no system select, checkbox or radio in shipped UI code.

The shared controls (`ui/src/components/controls/`: Checkbox, Radio,
Picker, ChoiceLine, ChoiceOption, Segmented) draw every choice the same
way. This lint counts, per file of shipped UI code outside them, each raw
`<select>` and each `<input>` whose `type` is `checkbox` or `radio`. The
shared `<ChoiceOption type="checkbox">` is not an `<input>`, so it passes.

The allowlist `scripts/native_controls.allow` (`path kind count`) **only
shrinks**: a file over its count, or not listed, fails; a file under its
count fails until its line is lowered or removed.

Exempt: the shared controls themselves, tests (`*.test.*`, `*.spec.*`,
`/tests/`), generated types and the preview's data.

Usage: native_controls_lint.py [--print] [--selftest]
  --print     print today's counts in the allowlist's format
  --selftest  check the lint against a seeded tree
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "native_controls.allow"
SHARED = "ui/src/components/controls/"
EXEMPT = ("/tests/", ".test.", ".spec.", "/types/generated/", "/preview/", "/test/")
SELECT = re.compile(r"<select\b")
INPUT = re.compile(r"<input\b")
TYPE = re.compile(r"""\btype\s*=\s*(?:\{\s*)?["'`](checkbox|radio)["'`]""")


def tag_end(text: str, start: int) -> int:
    """Where the JSX tag opened at `start` ends: its `>` outside braces
    and quotes (an attribute's `(e) => …` doesn't end it)."""
    depth, quote, i = 0, "", start
    while i < len(text):
        c = text[i]
        if quote:
            if c == quote:
                quote = ""
        elif c in "\"'`":
            quote = c
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        elif c == ">" and depth == 0:
            return i
        i += 1
    return len(text)


def counts_in(text: str) -> dict[str, int]:
    out: dict[str, int] = {}
    if n := len(SELECT.findall(text)):
        out["select"] = n
    for m in INPUT.finditer(text):
        kind = TYPE.search(text, m.start(), tag_end(text, m.start()))
        if kind:
            out[kind.group(1)] = out.get(kind.group(1), 0) + 1
    return out


def scan(root: Path) -> dict[tuple[str, str], int]:
    found: dict[tuple[str, str], int] = {}
    for path in sorted((root / "ui" / "src").rglob("*")):
        if path.suffix not in (".tsx", ".ts", ".jsx", ".js") or not path.is_file():
            continue
        rel = path.relative_to(root).as_posix()
        if rel.startswith(SHARED) or any(part in f"/{rel}" for part in EXEMPT):
            continue
        for kind, n in counts_in(path.read_text(encoding="utf-8", errors="replace")).items():
            found[(rel, kind)] = n
    return found


def allowed(path: Path) -> dict[tuple[str, str], int]:
    out: dict[tuple[str, str], int] = {}
    if not path.exists():
        return out
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        file, kind, count = line.split()
        out[(file, kind)] = int(count)
    return out


def problems(found: dict[tuple[str, str], int], allow: dict[tuple[str, str], int]) -> list[str]:
    out = []
    for key, n in sorted(found.items()):
        cap = allow.get(key, 0)
        if n > cap:
            out.append(f"{key[0]}: {n} raw {key[1]}{'' if cap == 0 else f' (allowed {cap})'}: use the shared controls in {SHARED} (Checkbox, Radio, Picker, ChoiceOption)")
    for key, cap in sorted(allow.items()):
        n = found.get(key, 0)
        if n < cap:
            out.append(f"{key[0]}: {n} raw {key[1]}, the allowlist says {cap}: lower its line in scripts/native_controls.allow (it only shrinks)")
    return out


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / "ui" / "src"
        (src / "components" / "controls").mkdir(parents=True)
        (src / "screens").mkdir()
        (src / "components" / "controls" / "Toggle.tsx").write_text('<input type="checkbox" />')
        (src / "screens" / "Ok.tsx").write_text('<ChoiceOption type="checkbox" label="x" />\n<input type="text" onChange={(e) => go(e)} />')
        (src / "screens" / "Bad.tsx").write_text('<select value={v}>\n<input\n  onChange={(e) => x > 1}\n  type="radio" />\n<input type={"checkbox"} />')
        (src / "screens" / "bad.test.tsx").write_text('<select />')
        found = scan(root)
        assert found == {("ui/src/screens/Bad.tsx", "select"): 1, ("ui/src/screens/Bad.tsx", "radio"): 1, ("ui/src/screens/Bad.tsx", "checkbox"): 1}, found
        assert len(problems(found, {})) == 3
        allow = {("ui/src/screens/Bad.tsx", "select"): 1, ("ui/src/screens/Bad.tsx", "radio"): 1, ("ui/src/screens/Bad.tsx", "checkbox"): 1}
        assert problems(found, allow) == []
        # one fixed: the allowlist must shrink with it
        (src / "screens" / "Bad.tsx").write_text('<select value={v}>')
        assert any("lower its line" in p for p in problems(scan(root), allow))
    print("native-controls selftest: OK")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    found = scan(ROOT)
    if "--print" in argv:
        for (file, kind), n in sorted(found.items()):
            print(f"{file} {kind} {n}")
        return 0
    issues = problems(found, allowed(ALLOW))
    for issue in issues:
        print(f"native-controls: {issue}")
    if issues:
        return 1
    print(f"native-controls: OK ({sum(found.values())} allowed)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
