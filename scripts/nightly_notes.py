#!/usr/bin/env python3
"""D-20 nightly release notes, from the commits since the last nightly.

  nightly_notes.py VERSION FROM_SHA TO_SHA   write docs/releases/VERSION.md
  nightly_notes.py --selftest

Nightly notes follow the same rules as every release note
(scripts/release_notes_check.sh): one plain first sentence, then New /
Improved / Fixed. Only user-facing commit types count (feat → New,
perf → Improved, fix → Fixed). Scopes, task IDs, PR numbers and paths
are stripped; a subject that still names a file is left out.
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
        ],
        "2026-10-20",
    )
    assert text.startswith("Nightly build from the latest code, 2026-10-20."), text
    assert "## New\n- Accessibility gate — axe on every gallery state" in text, text
    assert "## Improved\n- Process rows never blink" in text, text
    assert "## Fixed\n- The wizard's Esc returns focus to the row" in text, text
    assert "AJ-E" not in text and "#4" not in text and "crates/" not in text, text
    assert "maintainer rulings" not in text and "dependency" not in text, text
    assert text.index("## New") < text.index("## Improved") < text.index("## Fixed")
    assert "No user-facing changes" in notes(["docs: x (#1)"], "2026-10-20")
    changed = notes(["feat: x (#1)"], "2026-10-20", schema_changed=True)
    assert changed.rstrip().endswith(DOWNGRADE) and "## Known issues" in changed, changed
    assert "## Known issues" not in notes(["feat: x (#1)"], "2026-10-20")
    print("nightly-notes selftest: OK")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    if len(args) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    version, start, end = args
    log = subprocess.run(
        ["git", "log", "--format=%s", f"{start}..{end}"] if start else ["git", "log", "--format=%s", "-50", end],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout.splitlines()
    date = subprocess.run(
        ["git", "show", "-s", "--format=%cs", end], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout.strip()
    target = ROOT / "docs/releases" / f"{version}.md"
    target.parent.mkdir(parents=True, exist_ok=True)
    sys.path.insert(0, str(ROOT / "scripts"))
    from downgrade_note import schema_at  # noqa: E402

    before, now = (schema_at(start), schema_at(end)) if start else (None, None)
    changed = before is not None and now is not None and now > before
    target.write_text(notes(log, date, schema_changed=changed))
    print(target.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    sys.exit(main())
