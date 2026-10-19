#!/usr/bin/env python3
"""AJ-E807 — milestone gate packets: an evidence review, not a status meeting.

  gate_packet.py build docs/qa/gates/<gate>.json   write the gate packet (Markdown + HTML)
  gate_packet.py check docs/qa/gate-<gate>.md      every link resolves; no PASS without evidence
  gate_packet.py signed docs/qa/gate-<gate>.md     exit 1 unless signed (date, commit, reviewer)

A criterion is PASS only when every evidence path resolves and every
owning task is DONE-EVIDENCED. A missing path is DEFERRED with the open
task IDs, or FAIL when the tasks claim done without proof — never PASS.
The packet also carries the review inputs: the gallery delta, the axe
summary, the measured performance table and the unresolved-items
register. Regenerating keeps an existing sign-off section as written.
"""
from __future__ import annotations

import html
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE = ROOT / "docs/qa/evidence"
SIGNOFF_HEADING = "## Sign-off"
STATUS = re.compile(r"^### (AJ-E\d+) .*?\| Status: ([A-Z-]+)([^|]*)\|", re.M)
OPEN_ROW = re.compile(r"\|\s*(PENDING|PARTIAL|FAIL|UNPROVEN|DEFERRED)\b[^|]*\|")


def task_status() -> dict[str, str]:
    out: dict[str, str] = {}
    for path in sorted((ROOT / "docs/tasks").glob("*.md")):
        for match in STATUS.finditer(path.read_text()):
            out[match.group(1)] = (match.group(2) + match.group(3)).strip()
    return out


def verdict(criterion: dict, statuses: dict[str, str]) -> tuple[str, str]:
    missing = [p for p in criterion["evidence"] if not (ROOT / p).exists()]
    open_tasks = [t for t in criterion["tasks"] if not statuses.get(t, "").startswith("DONE-EVIDENCED")]
    if missing and not open_tasks:
        return "FAIL", "tasks marked done but evidence missing: " + ", ".join(missing)
    if missing or open_tasks:
        why = []
        if open_tasks:
            why.append("open: " + ", ".join(f"{t} ({statuses.get(t, 'unknown').split(' (')[0]})" for t in open_tasks))
        if missing:
            why.append("no evidence yet: " + ", ".join(Path(p).parent.name or p for p in missing))
        return "DEFERRED", "; ".join(why)
    return "PASS", ""


def rel(path: str, out: Path) -> str:
    return os.path.relpath(ROOT / path, out.parent)


def section(path: Path, heading: str) -> str:
    """The body of `## heading` in a Markdown file (up to the next `## `)."""
    if not path.exists():
        return "_not available_"
    text = path.read_text()
    match = re.search(rf"^## {re.escape(heading)}\s*$(.*?)(?=^## |\Z)", text, re.M | re.S)
    return match.group(1).strip() if match else "_not available_"


def gallery_delta(since: str | None) -> str:
    gallery = "docs/qa/evidence/_gallery"
    total = len(list((ROOT / gallery).rglob("*.png")))
    if not since:
        return f"{total} captures. No earlier signed gate to compare with: review the whole gallery ([index](evidence/_gallery/index.html))."
    changed = subprocess.run(
        ["git", "diff", "--name-only", f"{since}..HEAD", "--", gallery],
        cwd=ROOT, capture_output=True, text=True, check=False,
    ).stdout.split()
    return f"{total} captures; {len(changed)} changed since {since[:8]}."


def unsigned_gates() -> list[str]:
    out = []
    for gate in sorted((ROOT / "docs/qa").glob("gate-*.md")):
        text = gate.read_text()
        if "## Sign-off" in text and re.search(r"^\|\s*Decision\s*\|\s*APPROVED\s*\|", text, re.M) is None:
            out.append(f"- Gate [{gate.name}]({gate.name}) is unsigned")
    return out


def unresolved(statuses: dict[str, str]) -> list[str]:
    rows = unsigned_gates()
    rows += [f"- {task} — {status}" for task, status in sorted(statuses.items()) if not status.startswith("DONE-EVIDENCED")]
    for manifest in sorted(EVIDENCE.glob("*/manifest.md")):
        hits = [line for line in manifest.read_text().splitlines() if line.startswith("|") and OPEN_ROW.search(line)]
        if hits:
            task = manifest.parent.name
            rows.append(f"- {task}: {len(hits)} manifest row(s) PENDING/PARTIAL/FAIL ([manifest](evidence/{task}/manifest.md))")
    return rows


def build(definition: Path) -> int:
    spec = json.loads(definition.read_text())
    out = ROOT / spec["output"]
    statuses = task_status()
    previous = out.read_text() if out.exists() else ""
    kept = previous[previous.index(SIGNOFF_HEADING):] if SIGNOFF_HEADING in previous else None
    head = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=False).stdout.strip()
    rows, results = [], []
    for criterion in spec["criteria"]:
        result, why = verdict(criterion, statuses)
        results.append((criterion, result, why))
        links = ", ".join(
            f"[{Path(p).parent.name if Path(p).name == 'manifest.md' else Path(p).name}]({rel(p, out)})"
            if (ROOT / p).exists() else f"`{p}` (missing)"
            for p in criterion["evidence"]
        )
        tasks = ", ".join(criterion["tasks"])
        rows.append(f"| {criterion['id']} | {criterion['criterion']} | {tasks} | {links} | **{result}**{' — ' + why if why else ''} |")
    counts = {v: sum(1 for _, r, _ in results if r == v) for v in ("PASS", "DEFERRED", "FAIL")}
    lines = [
        f"# {spec['title']}",
        "",
        f"Generated by `scripts/gate_packet.py build {definition.relative_to(ROOT)}` at commit `{head}` (`just gate-packet`).",
        "Do not edit by hand except the Sign-off section.",
        "",
        spec["scope"],
        "",
        f"**Criteria:** {counts['PASS']} PASS · {counts['DEFERRED']} DEFERRED · {counts['FAIL']} FAIL "
        f"of {len(results)}. The gate passes only when every criterion is PASS.",
        "",
        "## 1. Criteria",
        "",
        "| ID | Criterion | Tasks | Evidence | Verdict |",
        "|---|---|---|---|---|",
        *rows,
        "",
        "## 2. Review inputs",
        "",
        "### Visual gallery delta",
        "",
        gallery_delta(spec.get("previous_signed_sha")),
        "",
        "### Accessibility (AJ-E804)",
        "",
        "\n".join(
            line for line in (EVIDENCE / "AJ-E804/axe-summary.md").read_text().splitlines() if line.startswith("- ")
        ) if (EVIDENCE / "AJ-E804/axe-summary.md").exists() else "_no axe summary_",
        "",
        "### Performance, measured (AJ-E805)",
        "",
        section(EVIDENCE / "AJ-E805/manifest.md", "Measured budgets"),
        "",
        "### Unresolved items",
        "",
        *unresolved(statuses),
        "",
    ]
    lines.append(kept.rstrip() + "\n" if kept else "\n".join([
        SIGNOFF_HEADING,
        "",
        "Signing approves this packet at the commit named. An unsigned beta gate keeps the",
        "stable channel closed (`promote.yml` runs `scripts/gate_packet.py signed` before stable).",
        "",
        "| Field | Value |",
        "|---|---|",
        "| Decision | UNSIGNED |",
        "| Reviewer | |",
        "| Date | |",
        "| Commit | |",
        "",
    ]))
    out.write_text("\n".join(lines))
    write_html(spec, results, out)
    print(f"{out.relative_to(ROOT)}: {counts['PASS']} PASS, {counts['DEFERRED']} DEFERRED, {counts['FAIL']} FAIL")
    return 0


def write_html(spec: dict, results: list, out: Path) -> None:
    """The packet index as one page (the AJ-E807 1280x800 capture)."""
    target = EVIDENCE / "AJ-E807" / f"packet-{spec['gate']}.html"
    target.parent.mkdir(parents=True, exist_ok=True)
    rows = "".join(
        f"<tr><td>{html.escape(c['id'])}</td><td>{html.escape(c['criterion'])}</td>"
        f"<td class='v {r.lower()}'>{r}</td><td>{html.escape(w)}</td>"
        f"<td>{'<br>'.join(html.escape(p) for p in c['evidence'])}</td></tr>"
        for c, r, w in results
    )
    target.write_text(
        "<!doctype html><meta charset=utf-8><title>"
        + html.escape(spec["title"])
        + "</title><style>body{font:13px system-ui;margin:24px;color:#171c26}"
        "table{border-collapse:collapse;width:100%;table-layout:fixed}td,th{border:1px solid #d8dde8;"
        "padding:6px;vertical-align:top;overflow-wrap:anywhere}th{background:#f2f4f9;text-align:left}"
        ".v{font-weight:600}.pass{color:#065f46}.deferred{color:#a94e08}.fail{color:#b91c1c}"
        "col.id{width:44px}col.v{width:90px}</style>"
        f"<h1>{html.escape(spec['title'])}</h1><table><colgroup><col class=id><col><col class=v><col><col></colgroup>"
        "<tr><th>ID</th><th>Criterion</th><th>Verdict</th><th>Why</th><th>Evidence</th></tr>"
        + rows
        + "</table>\n"
    )


def check(gate: Path) -> int:
    text = gate.read_text()
    problems = []
    for target in re.findall(r"\]\(([^)#\s]+)(?:#[^)]*)?\)", text):
        if re.match(r"^[a-z]+://", target):
            continue
        if not (gate.parent / target).exists():
            problems.append(f"broken link: {target}")
    for line in text.splitlines():
        if line.startswith("| ") and "**PASS**" in line:
            cells = line.split("|")
            evidence = cells[4] if len(cells) > 5 else ""
            if "](" not in evidence or "(missing)" in evidence:
                problems.append(f"PASS without resolving evidence: {cells[1].strip()}")
    for problem in problems:
        print(f"{gate.relative_to(ROOT)}: {problem}")
    if not problems:
        print(f"{gate.relative_to(ROOT)}: every link resolves; no PASS without evidence")
    return 1 if problems else 0


def signed(gate: Path) -> int:
    text = gate.read_text() if gate.exists() else ""
    fields = dict(re.findall(r"^\|\s*(Decision|Reviewer|Date|Commit)\s*\|\s*([^|]*?)\s*\|", text, re.M))
    ok = (
        fields.get("Decision", "").upper() == "APPROVED"
        and fields.get("Reviewer", "") != ""
        and re.fullmatch(r"\d{4}-\d{2}-\d{2}", fields.get("Date", "")) is not None
        and re.fullmatch(r"[0-9a-f]{7,40}", fields.get("Commit", "")) is not None
    )
    print(f"{gate}: {'signed' if ok else 'UNSIGNED — needs Decision APPROVED, Reviewer, Date (YYYY-MM-DD) and Commit (sha)'}")
    return 0 if ok else 1


def main() -> int:
    if len(sys.argv) != 3 or sys.argv[1] not in {"build", "check", "signed"}:
        print(__doc__, file=sys.stderr)
        return 2
    command, path = sys.argv[1], Path(sys.argv[2])
    path = path if path.is_absolute() else ROOT / path
    return {"build": build, "check": check, "signed": signed}[command](path)


if __name__ == "__main__":
    sys.exit(main())
