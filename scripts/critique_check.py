#!/usr/bin/env python3
"""AJ-E2814 (D-57): every gallery family's screens have a design critique
that was written for the screens as they are.

A family is a folder under `docs/qa/evidence/_gallery/`. Its captures are
the `.png` files git tracks in it (the index, so a local Playwright run
that rewrites the gallery without staging it changes nothing here). Their
fingerprint is a short sha256 over each capture's path and blob id: the
same captures give the same fingerprint in a shallow clone, a fresh clone
or a worktree, whatever the files' times say.

- A family with `critique.md` carries the fingerprint of the captures it
  was written for, on a line `<!-- captures: <fingerprint> -->`. When the
  captures change, the critique is out of date: look at the screens again,
  update the critique, then `--stamp <family>`.
- A family without a critique is listed in `scripts/critique.allow`
  (`family fingerprint`), which only shrinks: the families from before
  AJ-E2814, pinned as they were. Once one's screens change, it needs its
  critique (and leaves the list).

Usage:
  scripts/critique_check.py               check (exit 1 on a finding)
  scripts/critique_check.py --stamp FAM…  write FAM's current fingerprint
                                          into its critique.md
  scripts/critique_check.py --print-allow print today's uncritiqued
                                          families in the allowlist's format
  scripts/critique_check.py --html FILE   a critique as HTML, for
                                          scripts/gallery_index.sh
  scripts/critique_check.py --selftest
Messages name families only.
"""
from __future__ import annotations

import hashlib
import html
import re
import subprocess
import sys
import tempfile
from pathlib import Path

GALLERY = "docs/qa/evidence/_gallery"
CRITIQUE = "critique.md"
ALLOW = "scripts/critique.allow"
STAMP = re.compile(r"<!--\s*captures:\s*([0-9a-f]{12}|none)\s*-->")


def tracked(root: Path) -> dict[str, list[str]]:
    """Each family's tracked captures: `path blob` lines (one git call)."""
    out = subprocess.run(
        ["git", "ls-files", "-s", "--", GALLERY], cwd=root, check=True, capture_output=True, text=True
    ).stdout
    families: dict[str, list[str]] = {}
    for line in out.splitlines():
        meta, _, path = line.partition("\t")
        if not path.endswith(".png"):
            continue
        rest = path[len(GALLERY) + 1 :]
        if "/" not in rest:
            continue
        family = rest.split("/", 1)[0]
        families.setdefault(family, []).append(f"{rest} {meta.split()[1]}")
    return families


def fingerprint(lines: list[str]) -> str:
    """The captures' fingerprint: 12 hex digits, or "none" for none."""
    if not lines:
        return "none"
    return hashlib.sha256("\n".join(sorted(lines)).encode()).hexdigest()[:12]


def allowed(path: Path) -> dict[str, str]:
    """`family fingerprint` lines; # comments and blanks ignored."""
    if not path.exists():
        return {}
    out: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.split("#", 1)[0].strip()
        if line:
            family, _, value = line.partition(" ")
            out[family] = value.strip()
    return out


def critiques(root: Path) -> dict[str, str | None]:
    """Each family with a critique.md: the fingerprint it was stamped with."""
    out: dict[str, str | None] = {}
    base = root / GALLERY
    if not base.is_dir():
        return out
    for file in sorted(base.glob(f"*/{CRITIQUE}")):
        found = STAMP.search(file.read_text(encoding="utf-8"))
        out[file.parent.name] = found.group(1) if found else None
    return out


def check(root: Path) -> list[str]:
    captures = tracked(root)
    stamped = critiques(root)
    allow = allowed(root / ALLOW)
    problems: list[str] = []
    for family in sorted(set(captures) | set(stamped) | set(allow)):
        now = fingerprint(captures.get(family, []))
        if family in stamped:
            if family in allow:
                problems.append(f"{family}: has its critique — drop its line from {ALLOW} (it only shrinks)")
            if stamped[family] is None:
                problems.append(f"{family}: its critique has no captures line — run scripts/critique_check.py --stamp {family}")
            elif stamped[family] != now:
                problems.append(
                    f"{family}: its captures changed since its critique — review the screens, update the critique, "
                    f"then run scripts/critique_check.py --stamp {family}"
                )
        elif family in allow:
            if family not in captures:
                problems.append(f"{family}: no captures any more — drop its line from {ALLOW}")
            elif allow[family] != now:
                problems.append(f"{family}: its screens changed — write its {CRITIQUE} (D-57) and drop it from {ALLOW}")
        else:
            problems.append(f"{family}: no {CRITIQUE} beside its captures (D-57, AJ-E2814)")
    return problems


def stamp(root: Path, families: list[str]) -> list[str]:
    captures = tracked(root)
    problems = []
    for family in families:
        file = root / GALLERY / family / CRITIQUE
        if not file.exists():
            problems.append(f"{family}: no {CRITIQUE} to stamp")
            continue
        line = f"<!-- captures: {fingerprint(captures.get(family, []))} -->"
        text = file.read_text(encoding="utf-8")
        text = STAMP.sub(line, text, count=1) if STAMP.search(text) else f"{line}\n{text}"
        file.write_text(text, encoding="utf-8")
    return problems


def print_allow(root: Path) -> None:
    captures = tracked(root)
    stamped = critiques(root)
    for family in sorted(captures):
        if family not in stamped:
            print(f"{family} {fingerprint(captures[family])}")


def inline(text: str) -> str:
    """Escaped text, `code` as <code>."""
    parts = html.escape(text, quote=True).split("`")
    return "".join(f"<code>{part}</code>" if index % 2 else part for index, part in enumerate(parts))


def to_html(markdown: str) -> str:
    """A critique as HTML: headings, paragraphs, lists and tables only,
    every word escaped (the review page shows it beside the captures)."""
    out: list[str] = []
    rows: list[list[str]] = []
    items: list[str] = []

    def flush() -> None:
        if items:
            out.append("<ul>" + "".join(f"<li>{inline(item)}</li>" for item in items) + "</ul>")
            items.clear()
        if rows:
            head, *body = rows
            cells = lambda row, tag: "".join(f"<{tag}>{inline(cell)}</{tag}>" for cell in row)  # noqa: E731
            out.append(f"<table><tr>{cells(head, 'th')}</tr>" + "".join(f"<tr>{cells(row, 'td')}</tr>" for row in body) + "</table>")
            rows.clear()

    for raw in markdown.splitlines():
        line = raw.strip()
        if STAMP.fullmatch(line):
            continue
        if line.startswith("|"):
            if items:
                flush()
            cells = [cell.strip() for cell in line.strip("|").split("|")]
            if not all(re.fullmatch(r":?-+:?", cell) for cell in cells):
                rows.append(cells)
            continue
        if line.startswith("- "):
            if rows:
                flush()
            items.append(line[2:])
            continue
        flush()
        if line.startswith("#"):
            level = min(6, len(line) - len(line.lstrip("#")) + 2)
            out.append(f"<h{level}>{inline(line.lstrip('#').strip())}</h{level}>")
        elif line:
            out.append(f"<p>{inline(line)}</p>")
    flush()
    return "\n".join(out)


def selftest() -> int:
    failures = 0

    def expect(name: str, cond: bool) -> None:
        nonlocal failures
        if not cond:
            failures += 1
            print(f"selftest FAIL: {name}")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        git = lambda *args: subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)  # noqa: E731
        git("init", "-q")
        (root / "scripts").mkdir()
        old = root / GALLERY / "01-old"
        new = root / GALLERY / "02-new"
        old.mkdir(parents=True)
        new.mkdir(parents=True)
        (old / "a.png").write_bytes(b"one")
        (new / "b.png").write_bytes(b"two")
        (new / CRITIQUE).write_text("# 02-new\n", encoding="utf-8")
        git("add", "-A")
        expect("an unstamped critique fails", any("no captures line" in p for p in check(root)))
        expect("an unlisted family without a critique fails", any(p.startswith("01-old: no critique.md") for p in check(root)))
        stamp(root, ["02-new"])
        fp = fingerprint(tracked(root)["01-old"])
        (root / ALLOW).write_text(f"# pinned\n01-old {fp}\n", encoding="utf-8")
        expect("stamped and allowlisted passes", check(root) == [])
        (new / "b.png").write_bytes(b"changed")
        expect("an unstaged rewrite is ignored", check(root) == [])
        git("add", "-A")
        expect("changed captures flag the critique", any(p.startswith("02-new: its captures changed") for p in check(root)))
        stamp(root, ["02-new"])
        expect("restamped passes", check(root) == [])
        (old / "c.png").write_bytes(b"three")
        git("add", "-A")
        expect("an allowlisted family whose screens changed fails", any(p.startswith("01-old: its screens changed") for p in check(root)))
        (old / CRITIQUE).write_text("# 01-old\n", encoding="utf-8")
        stamp(root, ["01-old"])
        expect("a critiqued family still allowlisted is stale", any("drop its line" in p for p in check(root)))
        (root / ALLOW).write_text("09-gone abcdefabcdef\n", encoding="utf-8")
        expect("an allow line for a missing family is stale", any(p.startswith("09-gone: no captures") for p in check(root)))
        (root / ALLOW).write_text("", encoding="utf-8")
        expect("all critiqued passes", check(root) == [])
        expect("no captures fingerprint is 'none'", fingerprint([]) == "none")
    page = to_html("<!-- captures: none -->\n# A <b>\n\n| a | b |\n|---|---|\n| `x` | <i> |\n- one\ntext")
    expect("the stamp is left out of the page", "captures" not in page)
    expect("words are escaped", "<b>" not in page and "<i>" not in page and "&lt;b&gt;" in page)
    expect("a table, its separator dropped", page.count("<tr>") == 2 and "---" not in page)
    expect("code and lists", "<code>x</code>" in page and "<li>one</li>" in page and "<p>text</p>" in page)
    if failures:
        return 1
    print("critique-check selftest: OK")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    root = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True, text=True).stdout.strip())
    if argv[:1] == ["--stamp"]:
        problems = stamp(root, argv[1:])
        for problem in problems:
            print(problem)
        return 1 if problems else 0
    if argv[:1] == ["--html"] and len(argv) == 2:
        print(to_html(Path(argv[1]).read_text(encoding="utf-8")))
        return 0
    if argv[:1] == ["--print-allow"]:
        print_allow(root)
        return 0
    problems = check(root)
    for problem in problems:
        print(f"CRITIQUE: {problem}")
    if problems:
        return 1
    print("critique-check: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
