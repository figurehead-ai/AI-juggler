#!/usr/bin/env python3
"""AJ-E3831 harness v14: records lab turns for the feed line's sets.

Each scenario gets a fresh copy of one of `projects.py`'s made-up
projects (a git repository with one commit), then its turns run headless
on the sanctioned Lab CLI App (`claude -p`, CLAUDE_CONFIG_DIR = AI Juggler
Dev's lab account folder, ADR-0027 7b), the earlier turns first, each later
one resumed in the same session. The last turn is the one the set measures.

    python3 record.py <scenarios.json> <out-dir> [--jobs 3] [--only ID,ID]

<out-dir>/<id>/meta.json records the session, its transcript's path, each
turn's end (epoch ms) and the result; nothing is printed of the turns'
words (a held-out set is run by people who never read it). Tools: file
edits and local commands only (python3, git, ls…); no network tool, no
package installer. DISABLE_AUTOUPDATER=1.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import projects  # noqa: E402

LAB = Path.home() / "Library/Application Support/AI Juggler Dev/accounts/claude-code/claude-code-361780292000"
ALLOWED = [
    "Read", "Edit", "MultiEdit", "Write", "Glob", "Grep", "TodoWrite",
    "Bash(python3:*)", "Bash(git:*)", "Bash(ls:*)", "Bash(cat:*)", "Bash(mkdir:*)",
    "Bash(rm:*)", "Bash(mv:*)", "Bash(cp:*)", "Bash(head:*)", "Bash(tail:*)",
    "Bash(grep:*)", "Bash(wc:*)", "Bash(diff:*)", "Bash(find:*)", "Bash(sed:*)",
]
DENIED = [
    "WebFetch", "WebSearch", "Bash(pip:*)", "Bash(pip3:*)", "Bash(python3 -m pip:*)",
    "Bash(curl:*)", "Bash(wget:*)", "Bash(brew:*)", "Bash(npm:*)", "Bash(npx:*)",
]


def env() -> dict[str, str]:
    e = {k: v for k, v in os.environ.items() if not k.startswith(("CLAUDE", "ANTHROPIC"))}
    e.update(CLAUDE_CONFIG_DIR=str(LAB), DISABLE_AUTOUPDATER="1", CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC="1")
    return e


def run_turn(cwd: Path, request: str, sc: dict, session: str | None) -> dict:
    cmd = ["claude", "-p", request, "--output-format", "json", "--model", sc.get("model", "sonnet"),
           "--permission-mode", "acceptEdits", "--allowedTools", *ALLOWED, "--disallowedTools", *DENIED,
           "--max-turns", str(sc.get("max_turns", 40))]
    if sc.get("nudge"):
        cmd += ["--append-system-prompt", sc["nudge"]]
    if session:
        cmd += ["--resume", session]
    began = int(time.time() * 1000)
    p = subprocess.run(cmd, cwd=cwd, env=env(), capture_output=True, text=True, timeout=1800)
    ended = int(time.time() * 1000)
    try:
        out = json.loads(p.stdout.strip().splitlines()[-1])
    except (json.JSONDecodeError, IndexError):
        out = {"is_error": True, "subtype": "no_json", "stderr_tail": p.stderr[-400:]}
    return {"began_ms": began, "ended_ms": ended, "session": out.get("session_id"), "subtype": out.get("subtype"),
            "is_error": out.get("is_error"), "num_turns": out.get("num_turns"), "exit": p.returncode}


def transcript_of(cwd: Path, session: str) -> Path | None:
    mangled = str(cwd.resolve()).replace("/", "-").replace(".", "-").replace("_", "-")
    f = LAB / "projects" / mangled / f"{session}.jsonl"
    if f.is_file():
        return f
    hits = list((LAB / "projects").glob(f"*/{session}.jsonl"))
    return hits[0] if hits else None


def record(sc: dict, out: Path) -> str:
    d = out / sc["id"]
    if (d / "meta.json").is_file():
        return f"{sc['id']}: kept"
    cwd = d / sc["project"]
    projects.make(sc["project"], cwd, sc.get("plant"))
    git = ["git", "-c", "user.name=Lab", "-c", "user.email=lab@example.invalid"]
    subprocess.run(["git", "init", "-q"], cwd=cwd, check=True)
    subprocess.run([*git, "add", "-A"], cwd=cwd, check=True)
    subprocess.run([*git, "commit", "-qm", "start"], cwd=cwd, check=True)
    turns, session = [], None
    for req in [*sc.get("prior", []), sc["request"]]:
        t = run_turn(cwd, req, sc, session)
        session = t["session"] or session
        turns.append(t)
        if not session:
            break
    meta = {"id": sc["id"], "project": sc["project"], "cwd": str(cwd), "session": session, "turns": turns,
            "transcript": str(transcript_of(cwd, session)) if session else None}
    (d / "meta.json").write_text(json.dumps(meta, indent=1))
    last = turns[-1]
    return f"{sc['id']}: {last['subtype']} turns={last['num_turns']} {'transcript' if meta['transcript'] else 'NO TRANSCRIPT'}"


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    scenarios = json.loads(Path(sys.argv[1]).read_text())
    out = Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    jobs = int(sys.argv[sys.argv.index("--jobs") + 1]) if "--jobs" in sys.argv else 3
    only = set(sys.argv[sys.argv.index("--only") + 1].split(",")) if "--only" in sys.argv else None
    todo = [s for s in scenarios if not only or s["id"] in only]
    with ThreadPoolExecutor(jobs) as pool:
        for line in pool.map(lambda s: record(s, out), todo):
            print(line, flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
