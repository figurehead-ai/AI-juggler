#!/usr/bin/env python3
"""AJ-E3301: the 0.4 scenarios — one file each in docs/qa/scenarios/ —
drive every layer: L2 seeds the lab from `setup`, L3's journeys and L4's
hand walk follow `steps`, so a walk and its automation can't drift apart.

A scenario file (TOML, the subset below) says:

    id = "P1"                       # the plan's name for it
    title = "Writer and reviewer"
    epic = "E30"
    decisions = ["D-62.9", "D-44"]  # what it validates
    layers = ["L2", "L3", "L4"]
    exercises = ["App member", "Reviewer"]
    needs = ["lab"]                 # replica | lab | live-claude | live-codex | rc

    [setup]                         # the starting state, as data
    projects = ["web"]
    [[setup.apps]]                  # tool, account, profile, running,
                                    # signed_in (false: none), room_used (% of 5 h)
    [[setup.sessions]]              # id, app, project, state
    [[setup.pools]]                 # name, template, scope, sealed
    [[setup.members]]               # pool, kind, ref, role, level

    [[steps]]
    do = "New pool → Writer and reviewer"
    expect = "The wizard opens on its first step"
    measure = "click to wizard ≤ 100 ms"   # optional

Usage:
  scenario.py check                 every file parses and has what it needs
  scenario.py sheet ID [--out F]    the walk sheet for one scenario
  scenario.py seed ID               the setup as JSON (the lab seeder reads it)
  scenario.py list                  id, title, needs
  scenario.py --selftest
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCENARIOS = ROOT / "docs" / "qa" / "scenarios"
NEEDS = {"replica", "lab", "live-claude", "live-codex", "rc", "mock"}
LAYERS = {"L1", "L2", "L3", "L4", "L5"}
REQUIRED = ("id", "title", "epic", "layers", "needs", "steps")


# --- a TOML reader for the subset the scenario files use -----------------------

def _value(raw: str, where: str):
    raw = raw.strip()
    if raw.startswith('"'):
        if not raw.endswith('"') or len(raw) < 2:
            raise ValueError(f"{where}: unterminated string")
        return json.loads(raw)
    if raw.startswith("["):
        if not raw.endswith("]"):
            raise ValueError(f"{where}: unterminated list")
        inner = raw[1:-1].strip()
        if not inner:
            return []
        items, depth, cur, quoted = [], 0, "", False
        for ch in inner:
            if ch == '"' and (not cur.endswith("\\")):
                quoted = not quoted
            if ch == "," and not quoted and depth == 0:
                items.append(_value(cur, where))
                cur = ""
                continue
            cur += ch
        if cur.strip():
            items.append(_value(cur, where))
        return items
    if raw in ("true", "false"):
        return raw == "true"
    if re.fullmatch(r"-?\d+", raw):
        return int(raw)
    raise ValueError(f"{where}: can't read {raw!r} (strings need quotes)")


def _strip_comment(line: str) -> str:
    quoted = False
    for at, ch in enumerate(line):
        if ch == '"' and (at == 0 or line[at - 1] != "\\"):
            quoted = not quoted
        if ch == "#" and not quoted:
            return line[:at]
    return line


def parse(text: str, name: str = "<scenario>") -> dict:
    try:
        import tomllib  # Python 3.11+
        return tomllib.loads(text)
    except ModuleNotFoundError:
        pass
    data: dict = {}
    table = data
    for number, line in enumerate(text.splitlines(), 1):
        where = f"{name}:{number}"
        line = _strip_comment(line).strip()
        if not line:
            continue
        if line.startswith("[["):
            path = line.strip("[] ").split(".")
            parent = data
            for key in path[:-1]:
                parent = parent.setdefault(key, {})
            table = {}
            parent.setdefault(path[-1], []).append(table)
        elif line.startswith("["):
            path = line.strip("[] ").split(".")
            table = data
            for key in path:
                table = table.setdefault(key, {})
        else:
            if "=" not in line:
                raise ValueError(f"{where}: expected key = value")
            key, raw = line.split("=", 1)
            table[key.strip()] = _value(raw, where)
    return data


# --- checks ----------------------------------------------------------------------

def problems(scenario: dict, name: str) -> list[str]:
    out = [f"{name}: no `{key}`" for key in REQUIRED if key not in scenario]
    for need in scenario.get("needs", []):
        if need not in NEEDS:
            out.append(f"{name}: unknown need {need!r}")
    for layer in scenario.get("layers", []):
        if layer not in LAYERS:
            out.append(f"{name}: unknown layer {layer!r}")
    for index, step in enumerate(scenario.get("steps", []), 1):
        for key in ("do", "expect"):
            if not str(step.get(key, "")).strip():
                out.append(f"{name}: step {index} has no `{key}`")
    if scenario.get("id") and not name.startswith(str(scenario["id"]) + "-"):
        out.append(f"{name}: the file should start with its id ({scenario['id']}-…)")
    return out


def load_all() -> dict[str, dict]:
    found = {}
    for path in sorted(SCENARIOS.glob("*.toml")):
        found[path.name] = parse(path.read_text(), path.name)
    return found


def by_id(wanted: str) -> dict:
    for scenario in load_all().values():
        if scenario.get("id") == wanted:
            return scenario
    raise SystemExit(f"scenario {wanted!r} not found in {SCENARIOS}")


# --- the walk sheet -------------------------------------------------------------

def sheet(scenario: dict) -> str:
    rows = [
        f"# {scenario['id']} {scenario['title']} — walk sheet",
        "",
        f"Generated from `docs/qa/scenarios/` by `scripts/qa/scenario.py sheet {scenario['id']}`: "
        "the steps and expectations are the automation's own.",
        "",
        f"- **Validates:** {', '.join(scenario.get('decisions', [])) or '—'}",
        f"- **Exercises:** {', '.join(scenario.get('exercises', [])) or '—'}",
        f"- **Runs on:** {', '.join(scenario.get('needs', []))}; layers {', '.join(scenario.get('layers', []))}",
        "- **Build:** _(version and commit walked)_ · **Walked by / on:** _(who, date)_",
        "",
        "Pass: every step does what it promised; nothing under 4; feedback ≤ 100 ms; nothing cut off or overlapping. "
        "A failure is a finding on its task, and the whole scenario is walked again from the start.",
        "",
        "| # | Do | Expect | Seen | ms (feedback / done) | Works | Speed | Clarity | Layout | Captures (light · dark) |",
        "|---|---|---|---|---|---|---|---|---|---|",
    ]
    for index, step in enumerate(scenario["steps"], 1):
        expect = step["expect"] + (f" _(measure: {step['measure']})_" if step.get("measure") else "")
        rows.append(f"| {index} | {step['do']} | {expect} |  |  |  |  |  |  |  |")
    rows += ["", "## Findings", "", "_(each with its task, fix commit and the re-walk's date)_", ""]
    return "\n".join(rows)


# --- entry ----------------------------------------------------------------------

def selftest() -> None:
    sample = parse(
        '''
id = "Z1"  # a comment
title = "A \\"quoted\\" title"
layers = ["L2", "L4"]
needs = ["lab"]
count = 3
sealed = false
[setup]
projects = ["web", "api"]
[[setup.pools]]
name = "Team # not a comment"
[[setup.pools]]
name = "Two"
[[steps]]
do = "a"
expect = "b"
''',
        "Z1-sample.toml",
    )
    assert sample["title"] == 'A "quoted" title', sample["title"]
    assert sample["layers"] == ["L2", "L4"] and sample["count"] == 3 and sample["sealed"] is False
    assert [pool["name"] for pool in sample["setup"]["pools"]] == ["Team # not a comment", "Two"]
    assert sample["setup"]["projects"] == ["web", "api"]
    sample.update({"epic": "E30"})
    assert problems(sample, "Z1-sample.toml") == [], problems(sample, "Z1-sample.toml")
    assert problems({"id": "Z2", "steps": [{"do": "x"}]}, "Z2-x.toml")
    assert "| 1 | a | b |" in sheet({**sample, "id": "Z1"})
    for bad in ('x = nope', 'x = "open', 'x = [1, 2'):
        try:
            parse(bad)
        except ValueError:
            continue
        raise AssertionError(f"accepted {bad!r}")
    print("scenario selftest: ok")


def main(argv: list[str]) -> int:
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    if argv[0] == "--selftest":
        selftest()
        return 0
    if argv[0] == "check":
        all_ = load_all()
        found = [problem for name, scenario in all_.items() for problem in problems(scenario, name)]
        ids = [scenario.get("id") for scenario in all_.values()]
        found += [f"id {dup} used twice" for dup in sorted({i for i in ids if ids.count(i) > 1})]
        for problem in found:
            print(f"scenario: {problem}", file=sys.stderr)
        print(f"scenario: {len(all_)} file(s), {sum(len(s.get('steps', [])) for s in all_.values())} steps" + (" — problems above" if found else ", OK"))
        return 1 if found else 0
    if argv[0] == "list":
        for scenario in sorted(load_all().values(), key=lambda s: str(s.get("id"))):
            print(f"{scenario['id']:4} {scenario['title']}  [{', '.join(scenario['needs'])}]")
        return 0
    if argv[0] == "sheet" and len(argv) >= 2:
        text = sheet(by_id(argv[1]))
        if "--out" in argv:
            Path(argv[argv.index("--out") + 1]).write_text(text)
        else:
            print(text)
        return 0
    if argv[0] == "seed" and len(argv) == 2:
        print(json.dumps(by_id(argv[1]).get("setup", {}), indent=2))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
