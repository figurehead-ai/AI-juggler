#!/usr/bin/env python3
"""The docs that quote task counts and ranges agree with the epic files.

The epic files (docs/tasks/E*.md, one `### AJ-E…` heading per task) and
the decision log are the truth; these places repeat them and went stale:

- docs/tasks/TASK-CATALOG.md: a row per task and "Total: N tasks."
- docs/tasks/README.md: the file map's "AJ-E101-E108" range per epic,
  "E1-E13 below", and "decisions D-01..D-25".
- docs/plan/EPICS.md: "(`docs/tasks/E1..E13`, N tasks".

Usage: scripts/check_task_counts.py          (fails listing each mismatch)
       scripts/check_task_counts.py --selftest
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TASK = re.compile(r"^### AJ-E(\d+)\b", re.M)


def epics(root: Path) -> dict[str, list[int]]:
    """Epic file name → its task numbers (101, 102, …)."""
    out: dict[str, list[int]] = {}
    for path in sorted((root / "docs/tasks").glob("E*.md")):
        if re.match(r"E\d+-", path.name):
            out[path.name] = [int(n) for n in TASK.findall(path.read_text())]
    return out


def epic_number(name: str) -> int:
    return int(re.match(r"E(\d+)-", name).group(1))


def check(root: Path) -> list[str]:
    problems: list[str] = []
    files = epics(root)
    tasks = sorted(n for ns in files.values() for n in ns)
    total = len(tasks)
    last_epic = max(epic_number(name) for name in files)

    catalog = (root / "docs/tasks/TASK-CATALOG.md").read_text()
    rows = sorted(int(n) for n in re.findall(r"^\| AJ-E(\d+) \|", catalog, re.M))
    if rows != tasks:
        missing = sorted(set(tasks) - set(rows))
        extra = sorted(set(rows) - set(tasks))
        problems.append(
            f"TASK-CATALOG.md rows differ from the epic files: missing {['AJ-E%d' % n for n in missing]}, extra {['AJ-E%d' % n for n in extra]}"
        )
    stated = re.search(r"^Total: (\d+) tasks\.", catalog, re.M)
    if not stated or int(stated.group(1)) != total:
        problems.append(f"TASK-CATALOG.md says Total: {stated.group(1) if stated else '?'} tasks; the epic files hold {total}")

    readme = (root / "docs/tasks/README.md").read_text()
    for name, numbers in files.items():
        row = re.search(rf"^\| \[{re.escape(name)}\].*\| AJ-E(\d+)-E(\d+) \|$", readme, re.M)
        want = (min(numbers), max(numbers)) if numbers else None
        if not row:
            problems.append(f"docs/tasks/README.md has no file-map row with a range for {name}")
        elif want and (int(row.group(1)), int(row.group(2))) != want:
            problems.append(
                f"docs/tasks/README.md: {name} is AJ-E{row.group(1)}-E{row.group(2)}; its tasks are AJ-E{want[0]}-E{want[1]}"
            )
    span = re.search(r"\(E1-E(\d+) below\)", readme)
    if not span or int(span.group(1)) != last_epic:
        problems.append(f"docs/tasks/README.md says E1-E{span.group(1) if span else '?'}; the epics run to E{last_epic}")
    log = (root / "docs/review/decision-log.md").read_text()
    last_decision = max(int(n) for n in re.findall(r"^### D-(\d+)\b", log, re.M))
    decisions = re.search(r"decisions D-01\.\.D-(\d+)", readme)
    if not decisions or int(decisions.group(1)) != last_decision:
        problems.append(
            f"docs/tasks/README.md says decisions D-01..D-{decisions.group(1) if decisions else '?'}; the log runs to D-{last_decision:02d}"
        )

    plan = (root / "docs/plan/EPICS.md").read_text()
    quoted = re.search(r"\(`docs/tasks/E1\.\.E(\d+)`, (\d+) tasks", plan)
    if not quoted or (int(quoted.group(1)), int(quoted.group(2))) != (last_epic, total):
        got = f"E1..E{quoted.group(1)}, {quoted.group(2)} tasks" if quoted else "no count"
        problems.append(f"docs/plan/EPICS.md says {got}; the epic files are E1..E{last_epic}, {total} tasks")
    return problems


def selftest() -> None:
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "docs/tasks").mkdir(parents=True)
        (root / "docs/plan").mkdir(parents=True)
        (root / "docs/review").mkdir(parents=True)
        (root / "docs/tasks/E1-a.md").write_text("### AJ-E101 One\n### AJ-E102 Two\n")
        (root / "docs/tasks/E2-b.md").write_text("### AJ-E201 Three\n")
        (root / "docs/review/decision-log.md").write_text("### D-01 x\n### D-02 y\n")

        def write(total: int, e1_range: str, span: int, decisions: int, plan: str) -> list[str]:
            (root / "docs/tasks/TASK-CATALOG.md").write_text(
                "| AJ-E101 | One |\n| AJ-E102 | Two |\n| AJ-E201 | Three |\n\n" f"Total: {total} tasks. Regenerate.\n"
            )
            (root / "docs/tasks/README.md").write_text(
                f"decisions D-01..D-{decisions:02d}. Do not.\n4. The epic file (E1-E{span} below).\n"
                f"| [E1-a.md](E1-a.md) | A | {e1_range} |\n| [E2-b.md](E2-b.md) | B | AJ-E201-E201 |\n"
            )
            (root / "docs/plan/EPICS.md").write_text(f"(`docs/tasks/{plan}`, evidence-gated)\n")
            return check(root)

        assert write(3, "AJ-E101-E102", 2, 2, "E1..E2`, 3 tasks") == [], "clean tree flagged"
        assert len(write(2, "AJ-E101-E102", 2, 2, "E1..E2`, 3 tasks")) == 1, "stale total missed"
        assert len(write(3, "AJ-E101-E101", 2, 2, "E1..E2`, 3 tasks")) == 1, "stale range missed"
        assert len(write(3, "AJ-E101-E102", 1, 2, "E1..E2`, 3 tasks")) == 1, "stale epic span missed"
        assert len(write(3, "AJ-E101-E102", 2, 1, "E1..E2`, 3 tasks")) == 1, "stale decision span missed"
        assert len(write(3, "AJ-E101-E102", 2, 2, "E1..E1`, 107 tasks")) == 1, "stale EPICS count missed"
    print("task-counts selftest: OK")


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        selftest()
        sys.exit(0)
    found = check(ROOT)
    for problem in found:
        print(f"TASK-COUNTS: {problem}")
    if found:
        sys.exit(1)
    print("task-counts: OK")
