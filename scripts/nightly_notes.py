#!/usr/bin/env python3
"""D-20 nightly release notes, from the commits since the last nightly.

  nightly_notes.py VERSION FROM_SHA TO_SHA   write docs/releases/VERSION.md
  nightly_notes.py --selftest

Nightly notes follow the same rules as every release note
(scripts/release_notes_check.sh): one plain first sentence, then New /
Improved / Fixed. Only user-facing commit types count (feat → New,
perf → Improved, fix → Fixed). Scopes, task IDs, PR numbers and paths
are stripped; a subject that still names a file is left out. Notes
already written for VERSION (on main, by hand: 0.1.0's) are kept.
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SECTIONS = {"feat": "## New", "perf": "## Improved", "fix": "## Fixed"}
SUBJECT = re.compile(r"^(?P<type>[a-z]+)(\([^)]*\))?!?:\s*(?P<text>.+)$")
FORBIDDEN = re.compile(r"crates/|ui/src|docs/|\.rs\b|\.tsx?\b")


def clean(text: str) -> str:
    text = re.sub(r"\s*\(#\d+\)", "", text)  # (#41)
    # "review of #82", "reviews of #63, #70, #72": PR numbers anywhere.
    text = re.sub(r"\s*(?:\bof\s+)?#\d+(?:\s*(?:,|and)\s*#\d+)*", "", text)
    text = re.sub(r"\bAJ-E\d+[\w/,-]*\s*", "", text)  # task IDs
    text = re.sub(r"\s{2,}", " ", text).strip(" —-;,")
    return text[:1].upper() + text[1:] if text else text


DOWNGRADE = (
    "- **Going back to an earlier nightly** after this one: this build upgrades AI Juggler "
    "Nightly's data, which earlier builds can't open. Settings › About & updates › Go back to "
    "an earlier version puts the data from before the upgrade back."
)


def notes(subjects: list[str], date: str, schema_changed: bool = False) -> str:
    grouped: dict[str, list[str]] = {heading: [] for heading in SECTIONS.values()}
    for subject in subjects:
        match = SUBJECT.match(subject.strip())
        if not match or match.group("type") not in SECTIONS:
            continue
        text = clean(match.group("text"))
        if not text or FORBIDDEN.search(text):
            continue
        grouped[SECTIONS[match.group("type")]].append(f"- {text}")
    out = [f"Nightly build from the latest code, {date}. Expect rough edges."]
    if not any(grouped.values()):
        out += ["", "No user-facing changes since the last nightly build."]
    for heading in ("## New", "## Improved", "## Fixed"):
        if grouped[heading]:
            out += ["", heading, *grouped[heading]]
    if schema_changed:
        out += ["", "## Known issues", DOWNGRADE]
    return "\n".join(out) + "\n"


def selftest() -> int:
    text = notes(
        [
            "feat(e8/AJ-E804): accessibility gate — axe on every gallery state (#37)",
            "fix(ui): the wizard's Esc returns focus to the row (#42)",
            "perf(supervisor): process rows never blink (#36)",
            "docs: maintainer rulings (#40)",
            "build(deps): bring every dependency current (#41)",
            "fix: touch crates/juggler-app/src/lib.rs",
            "fix(e10/AJ-E1010): review of #82 — the launch dialog on every platform",
            "fix(e10/D-25): reviews of #63, #70, #72 — focus, keys after A",
        ],
        "2026-10-20",
    )
    assert text.startswith("Nightly build from the latest code, 2026-10-20."), text
    assert "## New\n- Accessibility gate — axe on every gallery state" in text, text
    assert "## Improved\n- Process rows never blink" in text, text
    assert "## Fixed\n- The wizard's Esc returns focus to the row" in text, text
    assert "AJ-E" not in text and not re.search(r"#\d", text) and "crates/" not in text, text
    assert "- Review — the launch dialog on every platform" in text, text
    assert "- Reviews — focus, keys after A" in text, text
    assert "maintainer rulings" not in text and "dependency" not in text, text
    assert text.index("## New") < text.index("## Improved") < text.index("## Fixed")
    assert "No user-facing changes" in notes(["docs: x (#1)"], "2026-10-20")
    changed = notes(["feat: x (#1)"], "2026-10-20", schema_changed=True)
    assert changed.rstrip().endswith(DOWNGRADE) and "## Known issues" in changed, changed
    assert "## Known issues" not in notes(["feat: x (#1)"], "2026-10-20")
    tags = ["v0.0.9", "v0.0.17", "v0.0.15", "beta/v0.0.15", "v0.1.32"]
    assert earlier_version("0.1.32", tags) == "0.0.17", earlier_version("0.1.32", tags)
    assert earlier_version("0.0.9", tags) is None
    print("nightly-notes selftest: OK")
    return 0


def earlier_version(version: str, tags: list[str]) -> str | None:
    """The newest `vX.Y.Z` tag below `version`: the version users come
    from when no previous build is named (release_notes_check.sh's default,
    so the notes and their check compare the same schemas)."""
    def key(text: str) -> tuple[int, ...]:
        return tuple(int(part) for part in text.split("."))
    known = [t[1:] for t in tags if re.fullmatch(r"v\d+\.\d+\.\d+", t)]
    lower = [t for t in known if key(t) < key(version)]
    return max(lower, key=key) if lower else None


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    if len(args) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    version, start, end = args
    target = ROOT / "docs/releases" / f"{version}.md"
    if target.exists():
        print(target.relative_to(ROOT), "(written by hand, kept)")
        return 0
    log = subprocess.run(
        ["git", "log", "--format=%s", f"{start}..{end}"] if start else ["git", "log", "--format=%s", "-50", end],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout.splitlines()
    date = subprocess.run(
        ["git", "show", "-s", "--format=%cs", end], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout.strip()
    target.parent.mkdir(parents=True, exist_ok=True)
    sys.path.insert(0, str(ROOT / "scripts"))
    from downgrade_note import schema_at  # noqa: E402

    # The schema users come from: the previous build, else the newest
    # earlier version (as the notes check reads it).
    tags = subprocess.run(["git", "tag", "-l", "v[0-9]*"], cwd=ROOT, capture_output=True, text=True).stdout.split()
    earlier = earlier_version(version, tags)
    base = start or (f"v{earlier}" if earlier else "")
    before, now = (schema_at(base), schema_at(end)) if base else (None, None)
    changed = before is not None and now is not None and now > before
    target.write_text(notes(log, date, schema_changed=changed))
    print(target.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    sys.exit(main())
