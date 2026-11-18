#!/usr/bin/env python3
"""D-127/D-128: the gate watch. What on this Mac has moved past what was
checked, so it is re-checked in the lab before the next release.

AI Juggler never gates a feature on a tool's version number (D-128). Two
things stand in for the old version lists, and both are data this reads:

- The fidelity register (crates/juggler-handover/data/fidelity-register.json):
  every decided cell records the versions of the run. A cell holds across
  patch and minor updates; a new MAJOR of either tool makes it due
  (`Register::due` in juggler-handover, the same rule).
- The shape checks: a feature that rewrites a tool's own files checks the
  data's shape at run time (the move: claude_move::shape). A mismatch hides
  that one option and AI Juggler keeps it, with the tool and its version,
  in <data>/compat/shape-mismatches.json; this lists what the installed
  app and the Dev build kept.

Installed versions are read read-only, as each adapter's spec says
(`version_probes`: a command run with `--version` in a clean environment,
or a desktop app's Info.plist). No tool is named here.

Usage: gate_watch.py            the report (exit 0: it reports, never gates)
       gate_watch.py --json     the same, as JSON
       gate_watch.py --selftest made-up register, probes and records
release_scope.py prints this report under its own.
"""
from __future__ import annotations

import json
import os
import plistlib
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

ROOT = Path(__file__).resolve().parent.parent
ADAPTERS = ROOT / "crates/juggler-adapters/src"
REGISTER = ROOT / "crates/juggler-handover/data/fidelity-register.json"
SUPPORT = Path.home() / "Library/Application Support"
# AI Juggler's data folders on a Mac: the installed app's and the Dev build's.
DATA_DIRS = [SUPPORT / "AI Juggler", SUPPORT / "AI Juggler Dev"]
SHAPE_FILE = "compat/shape-mismatches.json"
THROUGH_SERVICE = "+service"
# Where an installer puts a command when PATH (a GUI's) doesn't have it.
COMMAND_PLACES = [Path.home() / ".local/bin", Path("/opt/homebrew/bin"), Path("/usr/local/bin")]
NOT_TOOLS = {"fake", "fake-desktop"}
VERSION = re.compile(r"\d+(?:\.\d+)+")


@dataclass(frozen=True)
class Probe:
    tool: str
    kind: str  # command | app
    ref: str   # the command's name, or the bundle's path


@dataclass
class Report:
    installed: dict[str, str] = field(default_factory=dict)  # "claude-code" / "claude-code app" -> version
    due: list[str] = field(default_factory=list)
    shapes: list[str] = field(default_factory=list)
    unwatched: list[str] = field(default_factory=list)

    def lines(self) -> list[str]:
        out = ["gate watch (D-127/D-128): what moved past what was checked, on this Mac"]
        if self.installed:
            out.append("  installed  " + ", ".join(f"{k} {v}" for k, v in sorted(self.installed.items())))
        else:
            out.append("  installed  none of the tools is installed here: nothing to compare")
        out += [f"  DUE        {d}: re-run AJ-E808 for it in the lab" for d in self.due]
        out += [f"  SHAPE      {s}" for s in self.shapes]
        out += [f"  UNWATCHED  {u}" for u in self.unwatched]
        if not (self.due or self.shapes or self.unwatched):
            out.append("  nothing due: every decided route ran on the installed majors, and no shape check failed")
        return out


def major(version: str) -> int | None:
    match = re.match(r"\s*[vV]?(\d+)", version or "")
    return int(match.group(1)) if match else None


def side_tool(side: str) -> str:
    return side[: -len(THROUGH_SERVICE)] if side.endswith(THROUGH_SERVICE) else side


def probes(directory: Path = ADAPTERS) -> list[Probe]:
    """Every version probe the adapters' specs name, read from their sources."""
    found: list[Probe] = []
    for path in sorted(directory.glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        tool = re.search(r'pub const [A-Z_]+_ID: &str = "([a-z0-9-]+)";', text)
        if not tool or tool.group(1) in NOT_TOOLS:
            continue
        for match in re.finditer(r'VersionProbe::(Command|AppBundle)\s*\{\s*(program|path):\s*"([^"]+)"', text):
            variant, _, value = match.groups()
            found.append(Probe(tool.group(1), "command" if variant == "Command" else "app", value))
    return found


def clean_env() -> dict[str, str]:
    """The basics only, as scripts/qa/lab_env.sh keeps: never a session's
    markers (CLAUDE_CODE_*, CLAUDECODE, ANTHROPIC_*), no auto-update."""
    keep = {k: os.environ[k] for k in ("HOME", "PATH", "USER", "LOGNAME", "LANG", "TMPDIR") if k in os.environ}
    keep["DISABLE_AUTOUPDATER"] = "1"
    return keep


def read_version(probe: Probe) -> str | None:
    """The installed version, read only; None when it isn't installed."""
    if probe.kind == "app":
        plist = Path(probe.ref) / "Contents/Info.plist"
        try:
            with plist.open("rb") as handle:
                value = plistlib.load(handle).get("CFBundleShortVersionString")
        except (OSError, plistlib.InvalidFileException):
            return None
        return value if isinstance(value, str) and VERSION.search(value) else None
    path = os.environ.get("PATH", "") + os.pathsep + os.pathsep.join(str(p) for p in COMMAND_PLACES)
    binary = shutil.which(probe.ref, path=path)
    if not binary:
        return None
    try:
        done = subprocess.run([binary, "--version"], capture_output=True, text=True, timeout=15,
                              env=clean_env(), stdin=subprocess.DEVNULL, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    match = VERSION.search(done.stdout or "")
    return match.group(0) if match else None


def shape_records(data_dirs: list[Path]) -> list[str]:
    out: list[str] = []
    for data in data_dirs:
        path = data / SHAPE_FILE
        try:
            seen = json.loads(path.read_text(encoding="utf-8")).get("seen", [])
        except (OSError, ValueError, AttributeError):
            continue
        for entry in seen if isinstance(seen, list) else []:
            if not isinstance(entry, dict):
                continue
            where = "desktop app" if entry.get("app") else "tool folders"
            out.append(f"{entry.get('feature', '?')} on {entry.get('tool', '?')} ({where}) "
                       f"{entry.get('version') or 'of an unknown version'}: {entry.get('what', '?')} "
                       f"[kept by {data.name}]")
    return sorted(set(out))


def watch(register_path: Path = REGISTER, found: list[Probe] | None = None,
          read: Callable[[Probe], str | None] = read_version,
          data_dirs: list[Path] | None = None) -> Report:
    report = Report()
    found = probes() if found is None else found
    commands: dict[str, str] = {}
    for probe in found:
        version = read(probe)
        if version is None:
            continue
        if probe.kind == "command":
            commands[probe.tool] = version
            report.installed[probe.tool] = version
        else:
            report.installed[f"{probe.tool} app"] = version
    register = json.loads(register_path.read_text(encoding="utf-8"))
    watched = {probe.tool for probe in found if probe.kind == "command"}
    for pair in register.get("pairs", []):
        by_tool: dict[tuple[str, str, str], set[str]] = {}
        for cell in pair.get("cells", []):
            if cell.get("verdict") == "UNVALIDATED":
                continue
            for side, recorded in ((pair["from"], cell.get("source_version")),
                                   (pair["to"], cell.get("target_version"))):
                tool = side_tool(side)
                if tool not in watched:
                    note = f"{tool}: decided routes, and no adapter says how to read its version"
                    if note not in report.unwatched:
                        report.unwatched.append(note)
                    continue
                now = commands.get(tool)
                if recorded and now and major(now) != major(recorded):
                    by_tool.setdefault((tool, now, recorded), set()).add(cell.get("area", "?"))
        for (tool, now, recorded), areas in sorted(by_tool.items()):
            report.due.append(f"{pair['from']}→{pair['to']}: {tool} is {now}, the runs used {recorded} "
                              f"({len(areas)} area{'' if len(areas) == 1 else 's'})")
    report.shapes = shape_records(DATA_DIRS if data_dirs is None else data_dirs)
    return report


def selftest() -> int:
    import tempfile

    tmp = Path(tempfile.mkdtemp())
    register = tmp / "register.json"
    cell = lambda area, verdict, a, b: {"area": area, "verdict": verdict, "source_version": a, "target_version": b}
    register.write_text(json.dumps({"pairs": [
        {"from": "claude-code", "to": "claude-code", "cells": [
            cell("todos", "PASS", "2.1.310", "2.1.310"), cell("file-history", "UNVALIDATED", None, None)]},
        {"from": "claude-code", "to": "codex", "cells": [
            cell("todos", "PASS", "2.1.310", "0.163.0"), cell("subagents", "FAIL", "2.1.310", "0.163.0")]},
        {"from": "claude-code", "to": "claude-code+service", "cells": [cell("todos", "PASS", "1.0.0", "1.0.0")]},
        {"from": "zcode", "to": "zcode", "cells": [cell("todos", "PASS", "1.0", "1.0")]},
    ]}), encoding="utf-8")
    data = tmp / "AI Juggler Dev"
    (data / "compat").mkdir(parents=True)
    (data / SHAPE_FILE).write_text(json.dumps({"v": 1, "seen": [
        {"tool": "claude-code", "app": True, "version": "6.0.1", "feature": "move",
         "what": "a file beside the records AI Juggler doesn't know (sessions.db)", "at_ms": 1}]}), encoding="utf-8")
    made_up = [Probe("claude-code", "command", "claude"), Probe("claude-code", "app", "/Applications/Claude.app"),
               Probe("codex", "command", "codex")]
    versions = {"claude": "2.4.1", "/Applications/Claude.app": "6.0.1", "codex": "1.2.0"}
    report = watch(register, made_up, lambda probe: versions.get(probe.ref), [data])
    # A patch or minor update (2.1.310 → 2.4.1) is never due; a major
    # (0.163.0 → 1.2.0) is; a side through another service is its tool's
    # version (1.0.0 → 2.4.1).
    assert report.due == [
        "claude-code→codex: codex is 1.2.0, the runs used 0.163.0 (2 areas)",
        "claude-code→claude-code+service: claude-code is 2.4.1, the runs used 1.0.0 (1 area)",
    ], report.due
    assert report.installed == {"claude-code": "2.4.1", "claude-code app": "6.0.1", "codex": "1.2.0"}, report.installed
    assert report.shapes and "sessions.db" in report.shapes[0] and "6.0.1" in report.shapes[0], report.shapes
    assert report.unwatched == ["zcode: decided routes, and no adapter says how to read its version"], report.unwatched
    text = "\n".join(report.lines())
    assert "DUE        claude-code→codex" in text and "SHAPE      move on claude-code (desktop app) 6.0.1" in text, text
    # Nothing installed: nothing compared, nothing due.
    empty = watch(register, made_up, lambda probe: None, [tmp / "none"])
    assert not empty.due and not empty.shapes and "none of the tools is installed here" in "\n".join(empty.lines())
    # The real adapters name their probes, and every tool with a decided
    # route in the committed register has a command probe.
    real = probes()
    tools = {p.tool for p in real if p.kind == "command"}
    assert {"claude-code", "codex"} <= tools, real
    assert any(p.tool == "claude-code" and p.kind == "app" for p in real), real
    committed = json.loads(REGISTER.read_text(encoding="utf-8"))
    decided = {side_tool(side) for pair in committed["pairs"] for c in pair["cells"]
               if c["verdict"] != "UNVALIDATED" for side in (pair["from"], pair["to"])}
    assert decided <= tools, decided - tools
    assert major("v2.1.322 (Claude Code)") == 2 and major("5.11492.5") == 5 and major("") is None
    # A probe never hands a session's markers on.
    os.environ["CLAUDECODE"] = "1"
    try:
        env = clean_env()
        assert "CLAUDECODE" not in env and env["DISABLE_AUTOUPDATER"] == "1", env
    finally:
        del os.environ["CLAUDECODE"]
    print("gate-watch selftest: OK")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    report = watch()
    if args == ["--json"]:
        print(json.dumps({"installed": report.installed, "due": report.due, "shapes": report.shapes,
                          "unwatched": report.unwatched}, indent=2))
    else:
        print("\n".join(report.lines()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
