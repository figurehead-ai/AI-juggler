#!/usr/bin/env python3
"""Rust test code, found one way for every lint (D-94: one helper, not a
copy per lint).

Test code is any item under `#[cfg(test)]` or `#[cfg(all(test, …))]`,
after any other attributes: a `mod name { … }` or `mod name;` (with or
without `pub` or `pub(crate)`), a fn, impl, struct, use or const, a field,
anywhere in the file (code after it still counts). Braces are matched over
code only: a brace in a string, a char or a comment doesn't end it. A `mod name;` names a file that is test code whatever it is
called (`name.rs` or `name/mod.rs`, or a `#[path = "…"]`).

    blank_tests(text)       the text with its test modules blanked (each
                            character a space, newlines kept: offsets and
                            line numbers stay right)
    test_line_count(text)   how many lines its test modules take
    test_files(root)        repo-relative paths of out-of-line test modules

Usage: rust_tests.py --selftest
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

CFG_TEST = re.compile(r"#\[\s*cfg\s*\(\s*(?:test|all\s*\([^\]]*?\btest\b[^\]]*?\))\s*\)\s*\]")
ATTR = re.compile(r"\s*#\[(?:[^\[\]]|\[[^\]]*\])*\]")
PATH_ATTR = re.compile(r'#\[\s*path\s*=\s*"([^"]+)"\s*\]')
MOD = re.compile(r"\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(\w+)\s*([{;])")


def code_mask(text: str) -> list[bool]:
    """For each character, whether it is code (not a comment, string or char)."""
    mask = [True] * len(text)
    i, n = 0, len(text)

    def off(start: int, end: int) -> None:
        for k in range(start, min(end, n)):
            mask[k] = False

    while i < n:
        c = text[i]
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            off(i, end)
            i = end
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            off(i, j)
            i = j
        elif (raw := re.match(r'b?r(#*)"', text[i:i + 300])) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            close = '"' + raw.group(1)
            end = text.find(close, i + raw.end())
            end = n if end < 0 else end + len(close)
            off(i, end)
            i = end
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            off(i, j + 1)
            i = j + 1
        elif c == "'" and (char := re.match(r"'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'", text[i:i + 12])):
            off(i, i + char.end())
            i += char.end()
        else:
            i += 1
    return mask


def _modules(text: str) -> list[tuple[int, int, str, str | None]]:
    """(start, end, name, file) of each test module; `file` is the declared
    file for a `mod name;` (its `#[path]` when given), else None."""
    mask = code_mask(text)
    found: list[tuple[int, int, str, str | None]] = []
    pos = 0
    for attr in CFG_TEST.finditer(text):
        if attr.start() < pos or not mask[attr.start()]:
            continue
        index = attr.end()
        path = None
        while (more := ATTR.match(text, index)):
            if (given := PATH_ATTR.search(more.group(0))):
                path = given.group(1)
            index = more.end()
        decl = MOD.match(text, index)
        if decl and decl.group(2) == ";":
            found.append((attr.start(), decl.end(), decl.group(1), path or decl.group(1)))
            pos = decl.end()
            continue
        end = _item_end(text, mask, index)
        found.append((attr.start(), end, decl.group(1) if decl else "", None))
        pos = end
    return found


def _item_end(text: str, mask: list[bool], index: int) -> int:
    """Where the item starting at `index` ends: its `{ … }` block (after any
    signature), its `;`, or a `,` for a field or an arm, whichever comes
    first outside parentheses and brackets; never past the enclosing `}`."""
    nest, j = 0, index
    while j < len(text):
        c = text[j] if mask[j] else "\0"  # not "": "" is in every string
        if c in "([":
            nest += 1
        elif c in ")]":
            nest -= 1
        elif nest == 0 and c in ";,":
            return j + 1
        elif nest == 0 and c == "}":
            return j
        elif nest == 0 and c == "{":
            depth, j = 1, j + 1
            while j < len(text) and depth:
                if mask[j]:
                    depth += {"{": 1, "}": -1}.get(text[j], 0)
                j += 1
            return j
        j += 1
    return j


def blank_tests(text: str) -> str:
    """`text` with every test module blanked, newlines kept."""
    chars = list(text)
    for start, end, _, _ in _modules(text):
        for k in range(start, end):
            if chars[k] != "\n":
                chars[k] = " "
    return "".join(chars)


def test_line_count(text: str) -> int:
    """Lines a test module takes (from its `#[cfg(test)]` to its end)."""
    lines: set[int] = set()
    for start, end, _, _ in _modules(text):
        first = text.count("\n", 0, start)
        last = text.count("\n", 0, max(start, end - 1))
        lines.update(range(first, last + 1))
    return len(lines)


def test_files(root: Path) -> set[str]:
    """Repo-relative paths of the files that are `#[cfg(test)] mod name;`."""
    found: set[str] = set()
    crates = root / "crates"
    if not crates.exists():
        return found
    for path in crates.rglob("*.rs"):
        text = path.read_text(encoding="utf-8", errors="replace")
        if "cfg" not in text:
            continue
        beside = path.name in ("lib.rs", "main.rs", "mod.rs")
        base = path.parent if beside else path.parent / path.stem
        for _, _, name, file in _modules(text):
            if file is None:
                continue
            if file != name:  # a #[path]: relative to the declaring file's folder
                candidates = [path.parent / file]
            else:
                candidates = [base / f"{name}.rs", base / name / "mod.rs"]
            for candidate in candidates:
                try:
                    found.add(candidate.resolve().relative_to(root.resolve()).as_posix())
                except ValueError:
                    continue
    return found


def selftest() -> int:
    source = (
        "fn a() { let _ = \"{\"; }\n"                                    # 1 shipped
        "#[cfg(test)]\nmod tests { fn t() { let _ = '}'; } }\n"           # 2-3 test
        "fn b() {}\n"                                                     # 4 shipped
        "#[cfg(test)]\npub(crate) mod kit {\n    // }\n    fn k() {}\n}\n"  # 5-9 test
        "#[cfg(all(test, unix))]\n#[allow(dead_code)]\npub mod u { /* } */ }\n"  # 10-12 test
        "#[cfg(test)] mod inline { fn i() { let _ = r#\"}\"#; } }\n"     # 13 test
        "#[cfg(test)]\nmod fakes;\n"                                      # 14-15 test
        "#[cfg(test)]\n#[path = \"kit_file.rs\"]\nmod by_path;\n"         # 16-18 test
        "#[cfg(test)]\npub(crate) fn only_in_tests(x: [u8; 2]) -> u8 { x[0] }\n"  # 19-20 test (a fn)
        "// #[cfg(test)]\nmod commented {}\n"                             # 21-22 shipped
        "fn c<'a>(x: &'a str) -> &'a str { x }\n"                         # 23 shipped
        "struct S {\n    #[cfg(test)]\n    probe: u8,\n    kept: u8,\n}\n"  # 24-28: 25-26 test
        "#[cfg(test)]\nuse std::fmt;\nfn d() {}\n"                        # 29-30 test, 31 shipped
        "#[cfg(test)]\nconst T: &[&str] = &[\"(\", \"[ {\"];\nfn e() {}\n"  # 32-33 test, 34 shipped
    )
    cases = 0
    bad = 0

    def expect(ok: bool, what: str) -> None:
        nonlocal cases, bad
        cases += 1
        if not ok:
            bad += 1
            print(f"rust_tests selftest FAILED: {what}", file=sys.stderr)

    blanked = blank_tests(source)
    expect(blanked.count("\n") == source.count("\n"), "line numbers kept")
    for kept in ("fn a()", "fn b()", "mod commented", "fn c<'a>", "struct S", "kept: u8", "fn d()", "fn e()"):
        expect(kept in blanked, f"shipped code kept: {kept}")
    for gone in ("mod tests", "fn t()", "mod kit", "fn k()", "pub mod u", "mod inline", "mod fakes", "mod by_path", "only_in_tests", "probe", "use std::fmt", "const T"):
        expect(gone not in blanked, f"test code blanked: {gone}")
    expect(test_line_count(source) == 24, f"24 test lines, got {test_line_count(source)}")
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / "crates/x/src"
        (src / "a").mkdir(parents=True)
        (src / "lib.rs").write_text("#[cfg(test)]\nmod test_support;\n#[cfg(test)]\npub(crate) mod helpers;\nmod shipped;\n")
        (src / "a.rs").write_text("#[cfg(test)]\nmod fakes;\n#[cfg(test)]\n#[path = \"a_tests.rs\"]\nmod tests;\n")
        files = test_files(root)
        want = {
            "crates/x/src/test_support.rs", "crates/x/src/test_support/mod.rs",
            "crates/x/src/helpers.rs", "crates/x/src/helpers/mod.rs",
            "crates/x/src/a/fakes.rs", "crates/x/src/a/fakes/mod.rs",
            "crates/x/src/a_tests.rs",
        }
        expect(files == want, f"test files {sorted(files)}")
    if bad:
        return 1
    print(f"rust_tests selftest: OK ({cases} checks)")
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        sys.exit(selftest())
    print(__doc__)
    sys.exit(2)
