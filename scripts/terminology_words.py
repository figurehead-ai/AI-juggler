#!/usr/bin/env python3
"""AJ-E3201 (D-62): one word per thing, in what users read.

D-62's first decision: an **Account** is a subscription; an **App** is a
desktop app or one of its profiles ("Claude app · b-work"), or a CLI tool
on its folder ("Claude Code · Work"); a **Tool** is the product (Claude
Code, Codex); a **Project** is a code project; a **Session** is a chat in
an App. This lint flags the words those replaced, in the UI's copy
(string literals and JSX text in shipped `ui/src` code, through
`plain_words_lint`'s tokenizer, plus JSX text that wraps over lines) and
in the user guide's prose (`docs/guide/*.md`, without code, links'
targets or HTML tags):

- "account" for a folder or profile ("this account's folder");
- "profile" for an Account or a page of its own ("Which profile?",
  "Settings for this profile", "the profile's plan");
- "instance" (say session, or the app's profile);
- "place" for an App ("places in their usual order");
- "provider" for a Tool, and "harness" (say the tool, or the app).

Each rule keeps its true exceptions beside it, as a regex over the text
with the reason: English idioms ("in place", "in one place"), and the
company behind a model, which is an "AI provider". `terminology_lint.sh`
runs it after its own checks.

Usage: terminology_words.py [--selftest]
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import plain_words_lint as words  # noqa: E402  (shared tokenizer: the same "on screen")

ROOT = Path(__file__).resolve().parent.parent
# A word, not part of an identifier or a class name ("place-items",
# "data-instance", "instance_id").
W, E = r"(?<![\w-])", r"(?![\w-])"
FOLDERISH = r"(?:own\s+)?(?:isolated\s+|config(?:uration)?\s+|data\s+)?(?:folders?|profiles?|director(?:y|ies))\b"

# name -> (pattern, allowed-in-the-text regex or None, what to say instead)
RULES: dict[str, tuple[re.Pattern[str], re.Pattern[str] | None, str]] = {
    # A folder or a profile belongs to an App; an Account has neither.
    "account-as-folder": (
        re.compile(
            r"\b[Aa]ccount(?:s|['’]s|s['’])?\s+" + FOLDERISH
            + r"|\b[Aa]ccounts?\s+(?:gets?|has|have|keeps?)\s+(?:its|their)\s+" + FOLDERISH
        ),
        None,
        "an Account is the subscription; the folder or profile is its App's (\"this app's folder\")",
    ),
    # "profile" lives only inside an App's name or facts ("its profile
    # b-work", "Find this app's other profiles"); never the Account, a
    # choice of one, or a page.
    "profile-as-account": (
        re.compile(
            r"\b[Pp]rofile(?:['’]s)?\s+(?:plan|limits?|room|renewal|fee|subscription|sign-?in|runway|page|settings)\b"
            r"|\b(?:on|for)\s+(?:this|that|each|every|your)\s+profile\b"
            r"|\b[Pp]rofiles?\s+used\b|\b[Ww]hich\s+profile\b"
        ),
        None,
        "say account (the subscription) or app (\"Claude app · b-work\")",
    ),
    # D-25: on screen it is a session; D-62: an app's profile is "its profile".
    "instance": (re.compile(W + r"[Ii]nstances?" + E), None, "say session, or the app's profile"),
    # D-62 amends D-36: a place is an App. Idioms stay.
    "place": (
        re.compile(W + r"[Pp]laces?(?:['’]s)?" + E),
        re.compile(
            r"\bin\s+(?:one(?:\s+calm)?|its\s+own|the\s+same|the\s+right)\s+place\b"
            r"|\bin\s+place\b|\bno\s+place\s+for\b|\btakes?\s+place\b|\bkeeps?\s+its\s+place\b"
            r"|\b(?:a\s+few|several|many)\s+places\b",
            re.I,
        ),
        "say app (\"apps in their usual order\")",
    ),
    # D-25: the tool by name (Claude Code, Codex). The company behind a
    # model (Anthropic, OpenAI, a service's) is an "AI provider".
    "provider": (
        re.compile(W + r"[Pp]roviders?(?:['’]s?)?" + E),
        re.compile(r"\bAI\s+providers?(?:['’]s?)?" + E),
        "say the tool (Claude Code, Codex); the company behind a model is an \"AI provider\"",
    ),
    "harness": (re.compile(W + r"[Hh]arness(?:es)?" + E), None, "say the tool, or the app"),
}
# path -> [(rule, allowed regex, reason)]: whole-text exceptions in one file.
EXCEPTIONS: dict[str, list[tuple[str, re.Pattern[str], str]]] = {
    # cc-switch's own word: a "provider" there is an endpoint and key it
    # switches Claude Code or Codex between. The guide names it to say
    # what AI Juggler sees of it.
    "docs/guide/coming-from-other-tools.md": [
        ("provider", re.compile(r"^## cc-switch\n.*?(?=^## |\Z)", re.S | re.M), "cc-switch's own word, in its section"),
    ],
}
GUIDE = "docs/guide"

# JSX text that wraps: after a tag's `>`, up to the next tag or `{`, over
# more than one line (one line is `plain_words_lint`'s). Code never reads
# this way: a block holding `;`, `=`, `&&`, `||` or `?.` is code.
JSX_WRAPPED = re.compile(r"(?<![=\-\s])>([^<>{}]*\n[^<>{}]*)(?=[<{])")
CODEISH = re.compile(r"[;=]|&&|\|\||\?\.")
# The shared tokenizer reads a template nested in a template's `${…}`, or
# one after a regex literal, a little off: what it then hands over is
# code, not words, and is left alone.
MISREAD = re.compile(r"/\*|\*/|=>|;\s*\}|\bconst\s|\breturn\s")
# The guide's prose: no fenced code, inline code, link targets or tags.
FENCE = re.compile(r"^```.*?^```", re.S | re.M)
INLINE = re.compile(r"`[^`\n]*`|\]\([^)\n]*\)|<[^>\n]+>")


def without_comments(source: str) -> str:
    """The source with its comments blanked (lengths and lines kept)."""
    out = list(source)
    for match in words.TOKENS.finditer(source):
        if match.group(0).startswith(("//", "/*")):
            for i in range(match.start(), match.end()):
                if out[i] != "\n":
                    out[i] = " "
    return "".join(out)


def ui_texts(source: str, tsx: bool) -> list[tuple[int, str]]:
    found = []
    for offset, text in words.texts_at(source, False):
        text = text.split("${", 1)[0]  # an interpolation it couldn't close
        if MISREAD.search(text) or words.IDENTIFIER.match(text.strip()):
            continue
        found.append((offset, text))
    if tsx:
        plain = without_comments(source)
        for match in JSX_WRAPPED.finditer(plain):
            text = match.group(1)
            if re.search(r"[A-Za-z]{2}", text) and not CODEISH.search(text):
                found.append((match.start(1), text))
    return found


def blank(match: re.Match[str]) -> str:
    """A match's text as spaces, its line breaks kept (offsets hold)."""
    return re.sub(r"[^\n]", " ", match.group(0))


def guide_texts(source: str) -> list[tuple[int, str]]:
    """The guide page as prose: code, link targets and tags blanked."""
    return [(0, INLINE.sub(blank, FENCE.sub(blank, source)))]


def line_of(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def problems_in(rel: str, source: str, texts: list[tuple[int, str]]) -> list[str]:
    out = []
    seen = set()
    for offset, text in texts:
        for name, (pattern, allow, say) in RULES.items():
            # An allowed phrase covers the words inside it, no more.
            spans = [m.span() for m in allow.finditer(text)] if allow else []
            spans += [m.span() for rule, ok, _ in EXCEPTIONS.get(rel, []) if rule == name for m in ok.finditer(text)]
            for hit in pattern.finditer(text):
                if any(a <= hit.start() and hit.end() <= b for a, b in spans):
                    continue
                at = line_of(source, offset + hit.start())
                if (at, name) in seen:
                    continue
                seen.add((at, name))
                said = re.sub(r"\s+", " ", source.split("\n")[at - 1]).strip()
                out.append(f"TERMINOLOGY: {rel}:{at}: {said[:160]!r} — {name}: {say}")
    return out


def check(root: Path) -> list[str]:
    out: list[str] = []
    for path in sorted((root / "ui/src").rglob("*")):
        if path.suffix not in (".ts", ".tsx") or path.name.endswith(".d.ts"):
            continue
        rel = path.relative_to(root).as_posix()
        if words.exempt(rel):
            continue
        source = path.read_text(encoding="utf-8", errors="replace")
        out += problems_in(rel, source, ui_texts(source, path.suffix == ".tsx"))
    for path in sorted((root / GUIDE).glob("*.md")):
        rel = path.relative_to(root).as_posix()
        source = path.read_text(encoding="utf-8", errors="replace")
        out += problems_in(rel, source, guide_texts(source))
    return out


def selftest() -> int:
    bad_tsx = (
        '<p>Settings for this profile</p>;\n'
        'const a = "Which profile?";\n'
        'const b = `Opens ${name}\'s account folder`;\n'
        'const c = "Each new account gets its own isolated folder in here";\n'
        'const d = "Rows in order; places in their usual order.";\n'
        'const e = "Go to a place in the list";\n'
        '<p>No instances running</p>;\n'
        'const f = "The Provider\'s own transcript file";\n'
        "<div>\n  Desktop harnesses open their own window\n</div>;\n"
        'const g = "Profiles used here";\n'
    )
    good_tsx = (
        "// this account's folder, a comment\n"
        'const a = "instance"; const b = "aij:go-place"; const k = "place-items-center";\n'
        'const c = `${place.instance} · ${provider}`;\n'
        "const d = \"Put your own value in place of it.\";\n"
        'const e = "Your other Macs in one place.";\n'
        'const f = "Find this app\'s other profiles";\n'
        'const g = "It asks no AI provider for anything, at the AI providers\' prices.";\n'
        "const h = list.filter((p) => p.instance)\n  .map((x) => x);\n"
        "<p>Keep alive starts it in its own profile</p>;\n"
        'const t = `desktop-place-${place.app}${place.instance ? `-${place.instance}` : ""}`;\n'
        'record(`place:${id}:${rounded.map((n) => `${n.id}@${n.x}`).join(";")}`);\n'
    )
    bad_md = (
        "# Guide\n\nOpen the profile's page to see its plan.\n\n"
        "Each place has a header.\n\n"
        "Costs use each provider's prices.\n"
    )
    good_md = (
        "# Guide\n\nA session keeps its place in the list.\n\n"
        "Checked by [`a_claude_instance_test`](../x.rs).\n\n"
        "```\nprovider = x\n```\n\nIt asks no AI provider anything.\n"
    )
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "ui/src/components").mkdir(parents=True)
        (root / GUIDE).mkdir(parents=True)
        (root / "ui/src/components/Bad.tsx").write_text(bad_tsx)
        (root / "ui/src/components/Good.tsx").write_text(good_tsx)
        (root / "ui/src/components/Bad.test.tsx").write_text('const a = "Which profile?";\n')
        (root / GUIDE / "bad.md").write_text(bad_md)
        (root / GUIDE / "good.md").write_text(good_md)
        got = check(root)
        lines = {re.search(r"TERMINOLOGY: (\S+?:\d+):", p).group(1) for p in got}
        want = {f"ui/src/components/Bad.tsx:{n}" for n in (1, 2, 3, 4, 5, 6, 7, 8, 10, 12)}
        want |= {f"{GUIDE}/bad.md:{n}" for n in (3, 5, 7)}
        if lines != want:
            print("terminology-words selftest FAILED")
            print("  missed:", sorted(want - lines))
            print("  flagged wrongly:", sorted(lines - want))
            for problem in got:
                print("   ", problem)
            return 1
    print("terminology-words selftest: OK")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    problems = check(ROOT)
    for problem in problems:
        print(problem)
    if problems:
        print(f"terminology-words: {len(problems)} old word(s) in what users read")
        return 1
    print("terminology-words: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
