#!/usr/bin/env python3
"""0.5.2 RW1: the lab's Codex CLI threads whose last turn edits, for the
feed's diff stat (AJ-E3713 R6a) and the turn review (AJ-E3820) on a Codex
conversation. The Claude Code side is live: the lab stand-in's `lab: KIND`
turns (scripts/qa/lab_standin.sh, its header).

`seed` writes two threads in the lab's Codex App's folder, as Codex CLI
writes them in a terminal outside AI Juggler (a rollout under
`sessions/YYYY/MM/DD/` and its row in the thread store, `state_5.sqlite`,
the replica's own schema), each ended a minute ago:

- `cx-patch` in web: one `apply_patch` that updates src/api/client.py
  (+2 −1) and adds src/api/retry.py (+9): 2 files +11 −1; the reply says
  the tests pass.
- `cx-delete` in api: one `apply_patch` that deletes src/old_handlers.py
  and updates src/routes.py: a delete carries no line counts, so the post
  shows 2 files and no lines; the reply says the tests weren't run.
  Ended 2.5 hours before the first, so each is its own post (the feed rolls
  up one App's work within two hours into one).

The folder is the lab's Codex App's, read from the lab's Dev data
(`accounts`, read-only); every path written is inside the lab and the
lab's own repos are only named, never written. Nothing real is read.

  lab_turns.py seed --home /tmp/aij-lab-rw1 [--dry-run]
  lab_turns.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as dt
import importlib.util
import json
import sqlite3
import sys
import tempfile
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = "Library/Application Support/AI Juggler Dev"

THREADS = [
    {
        "id": "cx-patch", "project": "web", "ask": "Add a retry helper to the API client",
        "patch": ("*** Begin Patch\n*** Update File: {cwd}/src/api/client.py\n@@ def get(path):\n"
                  "-    return session.get(BASE + path)\n+    from .retry import retry\n"
                  "+    return retry(lambda: session.get(BASE + path))\n"
                  "*** Add File: {cwd}/src/api/retry.py\n+import time\n+\n+def retry(call, tries=3):\n"
                  "+    for n in range(tries):\n+        try:\n+            return call()\n+        except OSError:\n"
                  "+            time.sleep(0.1 * 2 ** n)\n+    return call()\n*** End Patch"),
        "reply": "I added a retry helper in retry.py and used it in client.py. The tests pass.",
    },
    {
        "id": "cx-delete", "project": "api", "ask": "Remove the old handlers module",
        "patch": ("*** Begin Patch\n*** Delete File: {cwd}/src/old_handlers.py\n"
                  "*** Update File: {cwd}/src/routes.py\n@@\n-from .old_handlers import routes\n"
                  "+from .handlers import routes\n*** End Patch"),
        "reply": "I removed the old handlers module and pointed routes.py at the new one. I have not run the tests.",
    },
]


def lab_only(path: Path, lab: Path) -> None:
    resolved = path.resolve()
    if not (str(resolved).startswith(("/tmp/", "/private/tmp/")) and "aij-lab" in str(resolved)
            and resolved.is_relative_to(lab.resolve())):
        sys.exit(f"lab_turns: {path} isn't inside the lab {lab}; refusing")


def replica():
    spec = importlib.util.spec_from_file_location("replica", ROOT / "scripts" / "replica.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def codex_folder(lab: Path) -> Path:
    db = lab / DATA / "juggler.sqlite"
    if not db.exists():
        sys.exit(f"lab_turns: no Dev data at {db} (lab_scenario.sh setup first)")
    # Read-only: through its shared memory while Dev runs on it, else (a
    # WAL database closed cleanly has none) as the unchanging file it is.
    rows = None
    for mode in ("mode=ro", "immutable=1"):
        try:
            con = sqlite3.connect(f"{db.as_uri()}?{mode}", uri=True)
            try:
                rows = con.execute("select state_dir from accounts where provider_id = 'codex'").fetchall()
                break
            finally:
                con.close()
        except sqlite3.OperationalError:
            continue
    if rows is None:
        sys.exit(f"lab_turns: {db} can't be read")
    if len(rows) != 1:
        sys.exit(f"lab_turns: the lab has {len(rows)} Codex Apps' folders; it needs one")
    return Path(rows[0][0])


def iso(at: dt.datetime) -> str:
    return at.strftime("%Y-%m-%dT%H:%M:%S.") + "%03dZ" % (at.microsecond // 1000)


def rollout(thread: dict, cwd: str, ended: dt.datetime) -> tuple[str, list[str], dt.datetime]:
    """A thread's rollout lines, as Codex CLI 0.16x writes one turn."""
    tid = str(uuid.uuid5(uuid.NAMESPACE_URL, "aij-lab-rw1/" + thread["id"]))
    at = ended - dt.timedelta(seconds=40)
    start = at
    lines = []

    def put(kind: str, payload: dict) -> None:
        nonlocal at
        lines.append(json.dumps({"timestamp": iso(at), "type": kind, "payload": payload}, separators=(",", ":")))
        at += dt.timedelta(seconds=4)

    put("session_meta", {"id": tid, "timestamp": iso(at), "cwd": cwd, "originator": "codex_cli_rs",
                         "cli_version": "0.163.0", "source": "cli", "model_provider": "openai",
                         "git": {"branch": "main"}, "history_mode": "paginated"})
    put("turn_context", {"cwd": cwd, "model": "gpt-5.1-codex"})
    put("event_msg", {"type": "task_started"})
    put("event_msg", {"type": "user_message", "message": thread["ask"]})
    put("response_item", {"type": "message", "role": "user", "content": [{"type": "input_text", "text": thread["ask"]}]})
    put("response_item", {"type": "custom_tool_call", "name": "apply_patch", "call_id": "call_" + thread["id"],
                          "input": thread["patch"].format(cwd=cwd), "status": "completed"})
    put("response_item", {"type": "custom_tool_call_output", "call_id": "call_" + thread["id"],
                          "output": "Success. Updated the following files."})
    put("response_item", {"type": "message", "role": "assistant",
                          "content": [{"type": "output_text", "text": thread["reply"]}]})
    put("event_msg", {"type": "agent_message", "message": thread["reply"]})
    put("event_msg", {"type": "task_complete", "last_agent_message": thread["reply"]})
    return tid, lines, start


def seed(lab: Path, dry_run: bool) -> list[str]:
    folder = codex_folder(lab)
    lab_only(folder, lab)
    writes = []
    ended = dt.datetime.now(dt.timezone.utc) - dt.timedelta(minutes=1)
    rep = replica() if not dry_run else None
    for n, thread in enumerate(THREADS):
        cwd = str((lab / "repos" / thread["project"]).resolve())
        lab_only(Path(cwd), lab)
        # The second one 2.5 h earlier: past the feed's same-verb window
        # (2 h), so each thread is its own post, not one rollup.
        done = ended - dt.timedelta(minutes=150 * n)
        tid, lines, start = rollout(thread, cwd, done)
        day = folder / "sessions" / f"{start:%Y}" / f"{start:%m}" / f"{start:%d}"
        path = day / f"rollout-{start:%Y-%m-%dT%H-%M-%S}-{tid}.jsonl"
        writes.append(f"{path} ({thread['id']}, {len(lines)} lines, in {thread['project']})")
        if dry_run:
            continue
        day.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(lines) + "\n")
        db = sqlite3.connect(folder / "state_5.sqlite")
        try:
            db.execute("pragma journal_mode=wal")
            db.execute(rep.CODEX_THREADS)
            made, last = int(start.timestamp()), int(done.timestamp())
            row = {"id": tid, "rollout_path": str(path), "created_at": made, "updated_at": last, "source": "cli",
                   "model_provider": "openai", "cwd": cwd, "title": thread["ask"],
                   "sandbox_policy": json.dumps({"type": "workspace-write"}), "approval_mode": "on-request",
                   "tokens_used": 24000, "has_user_event": 1, "git_branch": "main", "cli_version": "0.163.0",
                   "first_user_message": thread["ask"], "model": "gpt-5.1-codex", "created_at_ms": made * 1000,
                   "updated_at_ms": last * 1000, "preview": thread["ask"], "recency_at": last,
                   "recency_at_ms": last * 1000, "history_mode": "paginated", "originator": "codex_cli_rs"}
            db.execute(f"insert or replace into threads ({', '.join(row)}) values ({', '.join('?' * len(row))})",
                       list(row.values()))
            db.commit()
        finally:
            db.close()
        writes.append(f"{folder / 'state_5.sqlite'} (its row)")
    return writes


def selftest() -> None:
    with tempfile.TemporaryDirectory(prefix="aij-lab-turns-", dir="/tmp") as tmp:
        lab = Path(tmp)
        tid, lines, _ = rollout(THREADS[0], "/w", dt.datetime.now(dt.timezone.utc))
        kinds = [json.loads(line)["payload"].get("type") for line in lines]
        assert "user_message" in kinds and "custom_tool_call" in kinds and "task_complete" in kinds, kinds
        assert all('": ' not in line.split('"input"')[0] for line in lines), "compact JSON"
        for bad in (Path("/Users/someone/.codex"), lab.parent):
            try:
                lab_only(bad, lab)
            except SystemExit:
                continue
            raise AssertionError(f"{bad} wasn't refused")
    print("lab_turns selftest: ok")


def main() -> None:
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        return
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("seed")
    s.add_argument("--home", required=True)
    s.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    lab = Path(args.home)
    lab_only(lab, lab)
    for line in seed(lab, args.dry_run):
        print(("would write " if args.dry_run else "wrote ") + line)


if __name__ == "__main__":
    main()
