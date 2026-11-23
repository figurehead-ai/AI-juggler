#!/usr/bin/env python3
"""AJ-E1550 (PAT-SURF §5.12 and PAT-COLOR §5.13, DESIGN-SYSTEM; VALIDATION
N3): calm surfaces, and colour that always means one thing.

Separation is by tone, never by bright lines (the maintainer, 2026-11-23:
"Only time you can ever use a bright border is on highlight or mouse over
or to really really bring attention to the user"). This lint counts, per
file of shipped UI code (`ui/src`, `.ts`/`.tsx`), each class that draws a
line in a bright colour outside a focus or hover state:

- `bright-line`: a border, ring, inset ring, outline or divider utility
  (`border-*`, `border-x/y/t/r/b/l-*`, `ring-*`, `inset-ring-*`,
  `outline-*`, `divide-*`) in the accent (`accent`, `accent-hi`,
  `accent-fill`, `accent-soft`) or a bright neutral (`selected-ring`,
  `muted`, `ink`, `ink-2`), at any opacity, whose variants name no focus
  or hover state (`focus-visible:`, `focus-within:`, `focus:`, `hover:`,
  `group-hover/x:`, `has-[:focus-visible]:`, …). Not a line, so not
  counted: a spinner's arc (its class string has `animate-spin`) and a
  line drawn in its own fill's colour (`border-accent bg-accent`, a
  switch that is on). `faint` isn't bright here: it is the boundary an
  unchecked check, radio or mark needs (WCAG 1.4.11, 3:1).
- `sheen`: the retired `.sheen` top-light edge (ELV-2).
- `grayscale` (PAT-COLOR): a grayscale or saturate filter (`grayscale`,
  `saturate-*` utilities; `grayscale(`/`saturate(` in a style or in
  `ui/src/*.css`): App icons are always in full colour.
- `raw-hue` (PAT-COLOR): a Tailwind palette colour (`text-red-500`,
  `bg-emerald-400`…): status colour only through the semantic tokens
  (`danger`, `warning`, `success`, `live`, `accent`), each a soft and a
  strong step.
- `identity` (PAT-COLOR): an identity colour (`appColor(`,
  `accountColor(`) outside the identity marks' own components (Avatar,
  ProfileBadge, the charts' series colours, `lib/marks`): the eight App
  and Account colours are for small identity marks only, never status;
  each other use is listed and reviewed.

What stays allowed without a count: the PAT-SURF tokens (`border-subtle`,
`hairline`, `frame`, `selected-edge`, `focus`, `attention`), any colour
behind a focus or hover variant, status colours (a level's own line is
attention), and the shared controls' CSS in `styles.css` (not scanned).

Its allowlist `scripts/design_tokens.allow` (`path kind count`) **only
shrinks**: a file over its count, or not listed, fails; a file under its
count fails until its line is lowered or removed (`--print` gives today's
counts). Exempt: tests (`*.test.*`, `*.spec.*`, `/tests/`), the preview's
data, generated types, and `components/controls/selected.ts`, the one
home of the selected and chosen looks (its `CHOSEN_CARD` is the rich
mode's highlight of a picked card, D-126). Comments are skipped.

The rest of N3 (raw hex, raw sizes, radii, shadows, z-index) joins this
file as more kinds when it is built.

Usage: design_tokens_lint.py [--print] [--selftest]
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "design_tokens.allow"
EXEMPT = ("/tests/", ".test.", ".spec.", "/types/generated/", "/preview/", "/test/", "/components/controls/selected.ts")
# The identity marks' own homes (PAT-COLOR): where App and Account colours belong.
IDENTITY_HOMES = ("ui/src/lib/marks.ts", "ui/src/lib/mark-hues.ts", "ui/src/components/Avatar.tsx", "ui/src/components/ProfileBadge.tsx", "ui/src/components/charts/colors.ts")
GRAY_UTILITY = re.compile(r"^(?:grayscale|saturate-.+)$")
GRAY_STYLE = re.compile(r"\b(?:grayscale|saturate)\(")
RAW_HUE = re.compile(r"^-?(?:text|bg|border(?:-[xytrbl])?|ring|inset-ring|fill|stroke|from|to|via|outline|decoration|divide|shadow|accent|caret)-(?:red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|slate|gray|zinc|neutral|stone)-\d{2,3}(?:/\d+)?$")
IDENTITY = re.compile(r"\b(?:appColor|accountColor)\(")
COMMENT = re.compile(r"/\*[\s\S]*?\*/|(?<![:\\])//[^\n]*")
# A class string: a quoted literal (one line, no interpolation needed).
LITERAL = re.compile(r""""[^"\n]*"|'[^'\n]*'|`[^`]*`""")
# Class tokens: split where a class string can't continue.
SPLIT = re.compile(r"""[\s"'`{}+,;=()?]+""")
BRIGHT = re.compile(
    r"(?:border(?:-[xytrbl])?|inset-ring|ring|outline|divide(?:-[xy])?)-"
    r"(accent-hi|accent-fill|accent-soft|accent|selected-ring|muted|ink-2|ink)"
    r"(?:/\d+|/\[[^\]]*\])?"
)
STATE = re.compile(r"focus|hover")
WHY = {
    "bright-line": "a bright line outside focus or hover; separate by tone (surface steps), use border-subtle/hairline where tone can't, selected = SELECTED (a fill), Suggested = a tag, attention = ring-attention",
    "sheen": "the retired .sheen edge (ELV-2); a surface step separates it",
    "grayscale": "a grayscale or saturate filter; App icons are always in full colour, idle is said by the status dot and words (PAT-COLOR)",
    "raw-hue": "a raw palette colour; status colours only through the semantic tokens (danger, warning, success, live, accent) (PAT-COLOR)",
    "identity": "an App or Account colour outside an identity mark; identity colours only on small marks (dots, avatars, chart series), never status (PAT-COLOR)",
}


def split_variants(token: str) -> tuple[list[str], str]:
    """`hover:has-[:focus]:border-accent/40` → (['hover', 'has-[:focus]'], 'border-accent/40')."""
    parts, depth, last = [], 0, 0
    for i, c in enumerate(token):
        if c == "[":
            depth += 1
        elif c == "]":
            depth -= 1
        elif c == ":" and depth == 0:
            parts.append(token[last:i])
            last = i + 1
    return parts, token[last:]


def counts_in(text: str, identity_home: bool = False) -> dict[str, int]:
    out: dict[str, int] = {}
    code = COMMENT.sub("", text)
    if not identity_home and (n := len(IDENTITY.findall(code))):
        out["identity"] = n
    for literal in LITERAL.findall(code):
        if n := len(GRAY_STYLE.findall(literal)):
            out["grayscale"] = out.get("grayscale", 0) + n
        if "animate-spin" in literal:
            continue
        tokens = [t.strip("!").rstrip(".") for t in SPLIT.split(literal)]
        fills = {split_variants(t)[1] for t in tokens if t.startswith("bg-") or ":bg-" in t}
        for token in tokens:
            if token == "sheen":
                out["sheen"] = out.get("sheen", 0) + 1
                continue
            variants, utility = split_variants(token)
            if GRAY_UTILITY.match(utility):
                out["grayscale"] = out.get("grayscale", 0) + 1
                continue
            if RAW_HUE.match(utility):
                out["raw-hue"] = out.get("raw-hue", 0) + 1
                continue
            bright = BRIGHT.fullmatch(utility)
            if not bright or any(STATE.search(v) for v in variants):
                continue
            if f"bg-{bright.group(1)}" in fills:
                continue
            out["bright-line"] = out.get("bright-line", 0) + 1
    return out


def scan(root: Path) -> dict[tuple[str, str], int]:
    found: dict[tuple[str, str], int] = {}
    for path in sorted((root / "ui" / "src").rglob("*")):
        if path.suffix not in (".tsx", ".ts", ".css") or not path.is_file():
            continue
        rel = path.relative_to(root).as_posix()
        if any(part in f"/{rel}" for part in EXEMPT):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if path.suffix == ".css":
            if n := len(GRAY_STYLE.findall(re.sub(r"/\*[\s\S]*?\*/", "", text))):
                found[(rel, "grayscale")] = n
            continue
        for kind, n in counts_in(text, rel in IDENTITY_HOMES).items():
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
            out.append(f"{key[0]}: {n} {key[1]}{'' if cap == 0 else f' (allowed {cap})'}: {WHY[key[1]]} (DESIGN-SYSTEM §5.12-5.13)")
    for key, cap in sorted(allow.items()):
        n = found.get(key, 0)
        if n < cap:
            out.append(f"{key[0]}: {n} {key[1]}, the allowlist says {cap}: lower its line in scripts/design_tokens.allow (it only shrinks)")
    return out


def selftest() -> int:
    assert split_variants("hover:has-[:focus-visible]:border-accent/40") == (["hover", "has-[:focus-visible]"], "border-accent/40")
    assert counts_in('className="border border-hairline bg-surface"') == {}
    assert counts_in('className="rounded inset-ring-1 inset-ring-accent/70"') == {"bright-line": 1}
    assert counts_in('"focus-visible:ring-2 focus-visible:ring-accent/60 hover:border-accent/40"') == {}
    assert counts_in('"group-hover/card:inset-ring-accent has-[:focus-visible]:outline-ink-2"') == {}
    assert counts_in('`${on ? "border-selected-ring" : "border-ink-2"} ring-accent-soft divide-muted/30`') == {"bright-line": 4}
    assert counts_in('"inset-ring-[1.5px] inset-ring-faint" "border-2 border-accent border-t-transparent animate-spin"') == {}
    assert counts_in('(on ? "border-accent bg-accent" : "border-muted/60 bg-surface-3")') == {"bright-line": 1}
    assert counts_in('"aria-pressed:inset-ring-selected-ring border-x-accent"') == {"bright-line": 2}
    assert counts_in('"border-subtle ring-focus ring-attention inset-ring-selected-edge border-frame border-warning/50"') == {}
    assert counts_in('"text-accent bg-accent-soft border-transparent"') == {}
    assert counts_in('/* border-accent */ // ring-accent\nconst a = "sheen rounded-card";') == {"sheen": 1}
    # PAT-COLOR
    assert counts_in('"size-6 grayscale opacity-60" "saturate-50"') == {"grayscale": 2}
    assert counts_in('style={{ filter: "saturate(0.25)" }}') == {"grayscale": 1}
    assert counts_in('"text-red-500 bg-emerald-400/20 hover:border-amber-300"') == {"raw-hue": 3}
    assert counts_in('"text-danger bg-success-soft bg-live/60 text-warning"') == {}
    assert counts_in('const c = appColor(app); const d = accountColor(a);') == {"identity": 2}
    assert counts_in('const c = appColor(app);', identity_home=True) == {}
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / "ui" / "src"
        (src / "screens").mkdir(parents=True)
        (src / "screens" / "Bad.tsx").write_text('<div className="border border-accent/40" />\n<p className="sheen" />')
        (src / "screens" / "bad.test.tsx").write_text('<div className="border-accent" />')
        (src / "screens" / "Ok.tsx").write_text('<div className="bg-surface focus-visible:ring-accent" />')
        (src / "app.css").write_text("/* no grayscale( here */ .x { filter: grayscale(1); }")
        found = scan(root)
        assert found == {("ui/src/screens/Bad.tsx", "bright-line"): 1, ("ui/src/screens/Bad.tsx", "sheen"): 1, ("ui/src/app.css", "grayscale"): 1}, found
        (src / "app.css").write_text(".x { color: red; }")
        found = scan(root)
        assert len(problems(found, {})) == 2
        allow = {("ui/src/screens/Bad.tsx", "bright-line"): 1, ("ui/src/screens/Bad.tsx", "sheen"): 1}
        assert problems(found, allow) == []
        (src / "screens" / "Bad.tsx").write_text('<div className="border border-accent/40" />')
        assert any("lower its line" in p for p in problems(scan(root), allow))
        assert any("2 bright-line" in p for p in problems({("ui/src/screens/Bad.tsx", "bright-line"): 2}, allow))
    print("design-tokens selftest: OK")
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
        print(f"design-tokens: {issue}")
    if issues:
        return 1
    print(f"design-tokens: OK ({sum(found.values())} allowed, the list only shrinks)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
