#!/usr/bin/env python3
"""0.5.2 G56: a test never runs a program it put in an `.app` by hand.

A program written into `<name>.app/Contents/MacOS/` and run from there makes
macOS check the whole bundle first. A bundle without an `Info.plist` and a
seal over its resources fails that check ("code has no resources but
signature indicates they must be present", -67056), and macOS then shows
"“<name>” is damaged and can't be opened" on whoever's screen, in the
middle of their work (a unit test's stand-in `Claude.app`, a copy of
`bash`, raised it twice during the 0.5.2 RW1 walks).

The one way a test makes a stand-in app is the shared fixture
`crates/juggler-supervisor/tests/fixtures/stand_in_app.rs` (`stand_in_app::
make`): an `Info.plist`, the program, an ad-hoc signature over the whole
bundle, checked with `codesign --verify --strict` before anything runs.

This lint fails on a function (test code included: that is the point) that
names a `Contents/MacOS` path in a string, writes a file (`fs::copy`,
`fs::write`, `File::create`, `fs::hard_link`, `stand_in::write`) and runs
a program by a path it holds (`Command::new(<not a literal>)`), without
`stand_in_app::`.

Usage: stand_in_app_lint.py [--selftest]
"""
from __future__ import annotations

import pathlib
import re
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import rust_tests  # noqa: E402  (one code/comment/string finder for every lint, D-94)

FIXTURE = "crates/juggler-supervisor/tests/fixtures/stand_in_app.rs"
FN = re.compile(r"\bfn\s+(\w+)")
WRITE = re.compile(r"\bfs::(?:copy|write|hard_link)\s*\(|\bFile::create\s*\(|\bstand_in::write\s*\(")
# In the code view a string literal is blanked, so a literal argument reads
# as spaces up to the `)`.
RUN = re.compile(r"\bCommand::new\s*\(\s*[^\s)]")
HELPER = "stand_in_app::"


def strings(text: str, mask: list[bool]) -> list[tuple[int, int]]:
    """The spans of string literals (not comments) in `text`."""
    spans: list[tuple[int, int]] = []
    i, n = 0, len(text)
    while i < n:
        if mask[i]:
            i += 1
            continue
        j = i
        while j < n and not mask[j]:
            j += 1
        # A run that isn't code is comments, strings and chars side by side;
        # only the strings count.
        k = i
        while k < j:
            if text.startswith("//", k):
                end = text.find("\n", k)
                k = j if end < 0 or end > j else end
            elif text.startswith("/*", k):
                end = text.find("*/", k)
                k = j if end < 0 else min(j, end + 2)
            elif text[k] in "\"rb":
                spans.append((k, j))
                break
            else:
                k += 1
        i = j
    return spans


def bodies(text: str, mask: list[bool]) -> list[tuple[str, int, int]]:
    """(name, start, end) of every fn body `{ … }` in code."""
    found: list[tuple[str, int, int]] = []
    for match in FN.finditer(text):
        if not mask[match.start()]:
            continue
        nest, j = 0, match.end()
        while j < len(text):
            c = text[j] if mask[j] else "\0"
            if c in "([<":
                nest += 1
            elif c in ")]>":
                nest = max(0, nest - 1)
            elif c == ";" and nest == 0:
                break  # a declaration without a body
            elif c == "{" and nest == 0:
                depth, k = 1, j + 1
                while k < len(text) and depth:
                    if mask[k]:
                        depth += {"{": 1, "}": -1}.get(text[k], 0)
                    k += 1
                found.append((match.group(1), j, k))
                break
            j += 1
    return found


def file_findings(rel: str, text: str) -> list[str]:
    if "Contents/MacOS" not in text or rel == FIXTURE:
        return []
    mask = rust_tests.code_mask(text)
    literal = [span for span in strings(text, mask) if "Contents/MacOS" in text[span[0]:span[1]]]
    code = "".join(c if mask[i] else " " for i, c in enumerate(text))
    hits: list[tuple[str, int, int]] = []
    for name, start, end in bodies(text, mask):
        named = any(start <= s < end for s, _ in literal)
        if not named or HELPER in code[start:end]:
            continue
        if WRITE.search(code, start, end) and RUN.search(code, start, end):
            hits.append((name, start, end))
    # The innermost function that does it all is the one named.
    inner = [h for h in hits if not any(o != h and h[1] <= o[1] and o[2] <= h[2] for o in hits)]
    return [
        f"{rel}:{text.count(chr(10), 0, start) + 1}: fn {name} writes a program into an .app "
        f"and runs it without stand_in_app::make ({FIXTURE}): macOS calls such a bundle damaged "
        "and shows a dialog (G56)"
        for name, start, _ in inner
    ]


def findings(root: pathlib.Path) -> list[str]:
    out: list[str] = []
    for path in sorted((root / "crates").rglob("*.rs")):
        if "/target/" in path.as_posix():
            continue
        rel = path.relative_to(root).as_posix()
        out.extend(file_findings(rel, path.read_text(encoding="utf-8", errors="replace")))
    return out


def selftest() -> None:
    bad_bash = (
        "#[test]\nfn launches() {\n"
        "    let bundle = root.join(\"Claude.app/Contents/MacOS\");\n"
        "    let exe = bundle.join(\"Claude\");\n"
        "    std::fs::copy(\"/bin/bash\", &exe).unwrap();\n"
        "    let child = std::process::Command::new(&exe).spawn();\n}\n"
    )
    bad_stand_in = (
        "impl Busy {\n    fn start(dir: &Path) -> Self {\n"
        "        let app = dir.join(\"Claude.app/Contents/MacOS\");\n"
        "        stand_in::write(&app.join(\"Claude\"), b\"x\");\n"
        "        Self { app: Command::new(&app).spawn().unwrap() }\n    }\n}\n"
    )
    good = (
        "#[test]\nfn launches() {\n"
        "    let exe = stand_in_app::make(&root.join(\"Claude.app\"), \"Claude\", \"/bin/bash\", None);\n"
        "    // Contents/MacOS\n"
        "    std::fs::copy(\"/bin/bash\", &other).unwrap();\n"
        "    let child = std::process::Command::new(&exe).spawn();\n}\n"
    )
    data_only = (
        "#[test]\nfn reads_a_path() {\n"
        "    let exe = PathBuf::from(\"/Applications/Claude.app/Contents/MacOS/Claude\");\n"
        "    std::fs::write(dir.join(\"note\"), \"x\").unwrap();\n"
        "    let out = Command::new(\"/usr/bin/true\").status();\n}\n"
    )
    commented = (
        "// fs::copy into Claude.app/Contents/MacOS then Command::new(&exe)\n"
        "fn quiet() {\n    // \"Contents/MacOS\"\n"
        "    std::fs::copy(a, &b).unwrap();\n    Command::new(&b).spawn();\n}\n"
    )
    nested = (
        "mod tests {\n    fn outer() {\n        fn inner() {\n"
        "            let p = d.join(\"X.app/Contents/MacOS/X\");\n"
        "            std::fs::copy(\"/bin/sh\", &p).unwrap();\n"
        "            Command::new(&p).status();\n        }\n    }\n}\n"
    )
    checks = [
        (file_findings("a.rs", bad_bash), 1, "fn launches"),
        (file_findings("b.rs", bad_stand_in), 1, "fn start"),
        (file_findings("c.rs", good), 0, ""),
        (file_findings("d.rs", data_only), 0, ""),
        (file_findings("e.rs", commented), 0, ""),
        (file_findings("f.rs", nested), 1, "fn inner"),
        (file_findings(FIXTURE, bad_bash), 0, ""),
    ]
    for got, count, word in checks:
        assert len(got) == count, got
        assert all(word in line for line in got), got
    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)
        (root / "crates/x/src").mkdir(parents=True)
        (root / "crates/x/src/lib.rs").write_text(bad_bash)
        (root / "crates/x/src/ok.rs").write_text(good)
        got = findings(root)
        assert got == [line for line in got if line.startswith("crates/x/src/lib.rs:2:")] and len(got) == 1, got
    print("stand_in_app_lint selftest: ok")


def main() -> int:
    if "--selftest" in sys.argv[1:]:
        selftest()
        return 0
    found = findings(ROOT)
    for line in found:
        print(line, file=sys.stderr)
    if found:
        return 1
    print("stand_in_app_lint: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
