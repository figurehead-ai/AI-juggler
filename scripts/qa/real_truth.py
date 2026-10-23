#!/usr/bin/env python3
"""Ground truth for a walk on a real setup: what AI Juggler should show.

Read only. It opens nothing but:

- each Claude config folder's `projects/**/*.jsonl` (the default
  `~/.claude` and every `~/Library/Application Support/ClaudeCodeProfiles/*`);
- each Codex home's `sessions/**/rollout-*.jsonl` (`~/.codex`), and the
  `rollout-*.jsonl.zst` Codex compresses once a thread goes quiet (read
  with Python 3.14's `compression.zstd`, else `zstd -dc`; never skipped);
- the `.git` file of a worktree folder that still exists (to find its repo);
- `ps` and `lsof -d cwd` for the running `claude` / `codex` processes.

From the transcripts it keeps only names, mtimes and these fields: `cwd`,
`gitBranch`, `sessionId`, `timestamp`, `type`, `uuid`, `message.id`,
`requestId`, `message.usage`, and Codex's `session_meta` (id, cwd,
timestamp), `turn_context.model` and `token_count` (`info.total_token_usage`,
`info.last_token_usage`, `rate_limits`). No
message text, no credential files, no Keychain, no environment of other
processes.

What it prints (redacted: Account A…, Project 1…; `--key FILE` writes the
mapping to a private file, never commit it):

- projects, with agent worktrees (`<repo>/.claude/worktrees/<name>`,
  removed ones too) folded into their repo, and conversations per account;
- live sessions: each running CLI's account, project and app;
- tokens per account in the last 5 hours and 7 days (rolling windows,
  deduplicated by message id; the per-line sum shows the double count);
- Codex limits from the newest `rate_limits`: used %, window, reset, plan.

Usage: scripts/qa/real_truth.py [--json] [--key FILE] [--live-minutes N]
                                [--claude-dir DIR]... [--codex-dir DIR]...
                                [--plain-rollouts-only]   (what a reader that skips .jsonl.zst sees)
       scripts/qa/real_truth.py --codex-history   (rate_limits cadence and conflicts)
       scripts/qa/real_truth.py --compare OUT.json   (after the backend
           harness crates/juggler-app/tests/qa_real_readonly.rs wrote OUT.json)
       scripts/qa/real_truth.py --selftest
"""
from __future__ import annotations

import argparse
import calendar
import json
import os
import pwd
import re
import shutil
import subprocess
import sys
import time
from collections import defaultdict
from pathlib import Path

HOME = Path.home()
PROFILES = HOME / "Library/Application Support/ClaudeCodeProfiles"
WINDOWS = (("5h", 5 * 3600), ("7d", 7 * 86400))
ZSTD_COMMANDS = ("zstd", "/opt/homebrew/bin/zstd", "/usr/local/bin/zstd")
CODEX_CLASSES = (("input", "input_tokens"), ("cached_input", "cached_input_tokens"),
                 ("output", "output_tokens"), ("reasoning", "reasoning_output_tokens"))
WORKTREE = re.compile(r"^(?P<repo>.+?)/\.(?:claude|codex)/worktrees/[^/]+(?:/.*)?$")


# ---------------------------------------------------------------------------
# Small pure helpers (covered by --selftest)
# ---------------------------------------------------------------------------


def iso_to_epoch(raw: str | None) -> float | None:
    """`2026-10-21T19:18:59.391Z` → epoch seconds (UTC); None if not that shape."""
    if not isinstance(raw, str) or len(raw) < 19:
        return None
    try:
        return float(calendar.timegm(time.strptime(raw[:19], "%Y-%m-%dT%H:%M:%S")))
    except ValueError:
        return None


def encode_cwd(cwd: str) -> str:
    """Claude Code's project-folder name for a cwd: every non-alphanumeric → '-'."""
    return re.sub(r"[^A-Za-z0-9]", "-", cwd)


def project_of(cwd: str, git_file=None) -> tuple[str, str]:
    """(project root, how) for a cwd. Agent worktrees fold into their repo
    by path (works for removed ones); an existing linked worktree elsewhere
    folds by its `.git` file (`gitdir: <repo>/.git/worktrees/<name>`)."""
    m = WORKTREE.match(cwd)
    if m:
        return m.group("repo"), "worktree" if os.path.isdir(cwd) else "removed worktree"
    text = git_file(cwd) if git_file else _read_git_file(cwd)
    if text:
        g = re.match(r"gitdir:\s*(.+?)/\.git/worktrees/[^/\s]+\s*$", text.strip())
        if g:
            return g.group(1), "worktree"
    return cwd, "folder" if os.path.isdir(cwd) else "missing folder"


def _read_git_file(cwd: str) -> str | None:
    path = Path(cwd) / ".git"
    try:
        if path.is_file() and path.stat().st_size < 4096:
            return path.read_text(errors="replace")
    except OSError:
        pass
    return None


def letters(n: int) -> str:
    """0 → A, 25 → Z, 26 → AA."""
    s = ""
    n += 1
    while n:
        n, r = divmod(n - 1, 26)
        s = chr(65 + r) + s
    return s


# ---------------------------------------------------------------------------
# Readers
# ---------------------------------------------------------------------------


def claude_dirs(extra: list[str]) -> list[Path]:
    dirs = [HOME / ".claude"] if (HOME / ".claude/projects").is_dir() else []
    if PROFILES.is_dir():
        dirs += sorted(p for p in PROFILES.iterdir() if (p / "projects").is_dir())
    return dirs + [Path(d) for d in extra]


def codex_dirs(extra: list[str]) -> list[Path]:
    dirs = [HOME / ".codex"] if (HOME / ".codex/sessions").is_dir() else []
    return dirs + [Path(d) for d in extra]


def read_claude_conversation(path: Path) -> dict:
    """One top-level transcript → metadata and usage records (no content)."""
    conv = {"session": path.stem, "session_id": None, "mtime": path.stat().st_mtime, "cwd": None, "branch": None,
            "first": None, "last": None, "messages": 0, "usage": {}, "usage_lines": []}
    with path.open("rb") as fh:
        for raw in fh:
            try:
                line = json.loads(raw)
            except ValueError:
                continue
            if not isinstance(line, dict):
                continue
            if conv["cwd"] is None and isinstance(line.get("cwd"), str):
                conv["cwd"] = line["cwd"]
            if conv["session_id"] is None and isinstance(line.get("sessionId"), str):
                conv["session_id"] = line["sessionId"]
            if conv["branch"] is None and isinstance(line.get("gitBranch"), str):
                conv["branch"] = line["gitBranch"]
            ts = iso_to_epoch(line.get("timestamp"))
            if line.get("type") in ("user", "assistant") and ts is not None:
                conv["messages"] += 1
                conv["first"] = ts if conv["first"] is None else min(conv["first"], ts)
                conv["last"] = ts if conv["last"] is None else max(conv["last"], ts)
            msg = line.get("message")
            usage = msg.get("usage") if isinstance(msg, dict) else None
            if isinstance(usage, dict) and ts is not None:
                tokens = usage_tokens(usage)
                conv["usage"][(msg.get("id"), line.get("requestId"))] = (ts, tokens)
                conv["usage_lines"].append((ts, tokens))
    return conv


def usage_tokens(usage: dict) -> dict:
    def num(key):
        v = usage.get(key)
        return v if isinstance(v, int) else 0
    return {"input": num("input_tokens"), "output": num("output_tokens"),
            "cache_write": num("cache_creation_input_tokens"), "cache_read": num("cache_read_input_tokens")}


def rollout_files(sessions: Path, plain_only: bool = False) -> list[Path]:
    """A Codex home's rollouts: `rollout-*.jsonl` and, unless `plain_only`, the
    `rollout-*.jsonl.zst` Codex compresses quiet threads into (a compressed
    file keeps the plain one's mtime). A thread caught mid-compression, with
    both files, is read once, from the plain one."""
    plain = set(sessions.rglob("rollout-*.jsonl"))
    packed = set() if plain_only else {p for p in sessions.rglob("rollout-*.jsonl.zst") if p.with_suffix("") not in plain}
    return sorted(plain | packed)


def current_rollout(path: Path) -> Path:
    """`path`, or its `.zst` if Codex compressed it since it was listed."""
    packed = path.with_name(path.name + ".zst")
    return packed if path.suffix != ".zst" and not path.exists() and packed.exists() else path


def rollout_lines(path: Path):
    """A rollout's raw lines; a `.zst` one decompressed with `compression.zstd`
    (Python 3.14+), else the `zstd` command. One that can't be read stops the
    run: skipping it quietly is the blind spot this reader exists to close."""
    if path.suffix != ".zst":
        with path.open("rb") as fh:
            yield from fh
        return
    try:
        from compression import zstd  # Python 3.14+
    except ImportError:
        zstd = None
    if zstd is not None:
        with zstd.open(path, "rb") as fh:
            yield from fh
        return
    cli = next((found for found in map(shutil.which, ZSTD_COMMANDS) if found), None)
    if cli is None:
        raise SystemExit(f"{path.name} is compressed, and there is neither Python 3.14+ (compression.zstd) "
                         "nor a zstd command: run with Homebrew's python3, or pass --plain-rollouts-only")
    done = subprocess.run([cli, "-dc", "--", str(path)], capture_output=True, check=False)
    if done.returncode != 0:
        raise SystemExit(f"{path.name}: zstd -dc failed with {done.returncode}")
    yield from done.stdout.splitlines(keepends=True)


def read_codex_rollout(path: Path) -> dict:
    path = current_rollout(path)
    roll = {"session": None, "parent": None, "mtime": path.stat().st_mtime, "cwd": None, "first": None, "model": None,
            "compressed": path.suffix == ".zst", "totals": [], "turns": [], "turn_detail": [], "records": [],
            "rate_limits": None, "rate_ts": None}
    for raw in rollout_lines(path):
        if (b'"session_meta"' not in raw and b'"token_count"' not in raw and b'"turn_context"' not in raw
                and b'"token_usage_record"' not in raw):
            continue
        try:
            line = json.loads(raw)
        except ValueError:
            continue
        payload = line.get("payload") if isinstance(line, dict) else None
        if not isinstance(payload, dict):
            continue
        ts = iso_to_epoch(line.get("timestamp"))
        if line.get("type") == "session_meta":
            # The first names the file's own thread. A subagent's rollout
            # then carries its parent's (the history it was forked from).
            if roll["session"] is None:
                roll["session"] = payload.get("id")
                roll["cwd"] = payload.get("cwd") if isinstance(payload.get("cwd"), str) else None
                roll["first"] = iso_to_epoch(payload.get("timestamp")) or ts
                # Any `source.subagent` (a spawned thread, or `other`:
                # an auto-review run) is a subagent's rollout: its tokens
                # count, but it isn't a conversation of its own.
                source = payload.get("source") if isinstance(payload.get("source"), dict) else {}
                subagent = source.get("subagent")
                spawn = (subagent or {}).get("thread_spawn") if isinstance(subagent, dict) else None
                roll["parent"] = (payload.get("forked_from_id") or (spawn or {}).get("parent_thread_id")
                                  or ("subagent" if subagent is not None else None))
        elif line.get("type") == "token_usage_record":
            # Each model response's own usage, keyed. A context compaction
            # is a response too: its record is the only place it is counted,
            # since `token_count`'s running total leaves it out.
            usage = payload.get("usage") or {}
            if ts is not None and isinstance(usage.get("total_tokens"), int):
                key = (payload.get("thread_id"), payload.get("turn_id"), payload.get("response_id"))
                roll["records"].append((ts, key, usage["total_tokens"], (roll["model"], {
                    name: usage.get(field) if isinstance(usage.get(field), int) else 0 for name, field in CODEX_CLASSES})))
        elif line.get("type") == "turn_context":
            roll["model"] = payload.get("model") if isinstance(payload.get("model"), str) else roll["model"]
        elif payload.get("type") == "token_count" and ts is not None:
            info = payload.get("info") or {}
            total = info.get("total_token_usage") or {}
            last = info.get("last_token_usage") or {}
            if isinstance(total.get("total_tokens"), int):
                # A repeat (running total unchanged) is not a new turn. A
                # thread's first total can carry tokens from before it
                # (a subagent or compacted thread), so a thread's own use
                # is its turns' `last_token_usage`, not its final total.
                moved = not roll["totals"] or roll["totals"][-1][1]["total_tokens"] != total["total_tokens"]
                roll["totals"].append((ts, total))
                if moved and isinstance(last.get("total_tokens"), int):
                    roll["turns"].append((ts, last["total_tokens"], total["total_tokens"]))
                    roll["turn_detail"].append((roll["model"], {name: last.get(key) if isinstance(last.get(key), int) else 0
                                                                for name, key in CODEX_CLASSES}))
            if isinstance(payload.get("rate_limits"), dict):
                roll["rate_limits"], roll["rate_ts"] = payload["rate_limits"], ts
    return roll


def processes() -> list[dict]:
    """Running claude / codex CLIs: pid, tool, resume id, cwd, app."""
    out = subprocess.run(["ps", "-axo", "pid=,ppid=,args="], capture_output=True, text=True, check=False).stdout
    rows = {}
    for line in out.splitlines():
        parts = line.split(None, 2)
        if len(parts) == 3:
            rows[int(parts[0])] = (int(parts[1]), parts[2])
    found = []
    for pid, (ppid, args) in rows.items():
        exe = args.split(" -")[0].strip()
        tool = os.path.basename(exe)
        if tool not in ("claude", "codex"):
            continue
        words = set(args[len(exe):].split())
        helper = words & {"app-server", "exec-server", "mcp-server", "proto"}
        kind = sorted(helper)[0] if tool == "codex" and helper else "session"
        if is_temp(exe):
            kind = "stand-in"  # a test's stand-in binary in a temp folder, not a user's session
        cwd = process_cwd(pid)
        if kind == "session" and is_host(cwd):
            kind = "host"  # this Mac's own session, seen from a replica's HOME
        resume = None
        m = re.search(r"--resume[ =]([0-9a-f-]{36})", args)
        if m:
            resume = m.group(1)
        app, up = None, ppid
        for _ in range(12):
            if up not in rows:
                break
            # The outermost bundle of the ancestor's own executable (its
            # arguments may name another bundle).
            a = re.match(r"^.*?/([^/]+)\.app/", rows[up][1].split(" -")[0])
            if a:
                app = a.group(1)
                break
            up = rows[up][0]
        found.append({"pid": pid, "tool": tool, "kind": kind, "resume": resume, "app": app or "a terminal",
                      "cwd": cwd})
    return found


def desktop_roots() -> list[dict]:
    """Running desktop apps' main processes: pid and app (Claude, ChatGPT)."""
    out = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True, check=False).stdout
    roots = []
    for line in out.splitlines():
        pid, _, args = line.strip().partition(" ")
        m = re.match(r"^\S*?/(Claude|ChatGPT)\.app/Contents/MacOS/(Claude|ChatGPT)(?:\s|$)", args)
        if m and pid.isdigit():
            roots.append({"pid": int(pid), "app": m.group(1)})
    return roots


def is_temp(path: str) -> bool:
    """A path in a temp folder (the per-user TMPDIR, /tmp, /var/folders)."""
    real = os.path.realpath(path)
    if real.startswith(os.path.realpath(HOME).rstrip("/") + "/"):
        return False  # a CLI in the user's own home is theirs (a replica's HOME may itself be in /tmp)
    roots = ["/private/tmp/", "/private/var/folders/", "/tmp/", "/var/folders/"]
    if os.environ.get("TMPDIR"):
        roots.append(os.path.realpath(os.environ["TMPDIR"]).rstrip("/") + "/")
    return any(real.startswith(r) for r in roots)


def is_host(cwd: str | None) -> bool:
    """On a replica (a HOME that isn't this user's own), a session working
    outside that HOME is the host Mac's, not the replica's. Never true when
    HOME is the user's own home, so a real-data run leaves nothing out."""
    own = os.path.realpath(pwd.getpwuid(os.getuid()).pw_dir)
    home = os.path.realpath(HOME)
    return home != own and cwd is not None and not os.path.realpath(cwd).startswith(home.rstrip("/") + "/")


def process_cwd(pid: int) -> str | None:
    out = subprocess.run(["lsof", "-a", "-p", str(pid), "-d", "cwd", "-Fn"], capture_output=True, text=True,
                         check=False).stdout
    for line in out.splitlines():
        if line.startswith("n"):
            return line[1:]
    return None


# ---------------------------------------------------------------------------
# The truth
# ---------------------------------------------------------------------------


def build(args) -> dict:
    now = time.time()
    accounts = []  # (label, tool, folder)
    for d in claude_dirs(args.claude_dir):
        accounts.append(("~/.claude" if d == HOME / ".claude" else d.name, "claude", d))
    for d in codex_dirs(args.codex_dir):
        accounts.append(("~/.codex" if d == HOME / ".codex" else d.name, "codex", d))

    projects = defaultdict(lambda: {"how": set(), "per_account": defaultdict(int), "cwds": set(), "last": 0.0,
                                    "claude_sessions": set()})
    per_account = {}
    transcripts_by_session = {}
    session_usage = defaultdict(dict)  # session id → {(message id, request id): (ts, tokens)}
    session_span = {}  # session id → (first message, last message)
    session_totals = {}  # session id → its own tokens
    session_mtime = {}  # session id → newest write of its files
    # (account idx, the folder resolved) → [(mtime, session)]: a running
    # CLI's folder (from lsof, resolved) finds its conversations by it.
    newest_in_folder = defaultdict(list)
    # A switch copies a conversation into the next account under the same
    # session id, its earlier messages included (same message and request
    # ids). Those were used on the account they came from: each Claude
    # account's records are kept here and a message shared by copies is
    # credited to one account once every account is read.
    claude_records = {}  # account idx → {(message id, request id): (ts, tokens)}
    claude_lines = {}  # account idx → [(ts, tokens)] per line
    copies = defaultdict(dict)  # session id → {account idx: (first, last)}
    for idx, (label, tool, folder) in enumerate(accounts):
        acc = {"label": label, "tool": tool, "conversations": 0, "empty_files": 0, "subagent_files": 0,
               "changed": {"1h": 0, "24h": 0}, "tokens": {}, "tokens_per_line": {}, "codex_limits": None}
        if tool == "claude":
            records, lines = {}, []
            for path in (folder / "projects").rglob("*.jsonl"):
                conv = read_claude_conversation(path)
                sid = conv["session_id"] or conv["session"]
                session_usage[sid].update(conv["usage"])
                session_mtime[sid] = max(session_mtime.get(sid, 0), conv["mtime"])
                first, last = session_span.get(sid, (None, None))
                if conv["first"] is not None:
                    first = conv["first"] if first is None else min(first, conv["first"])
                if conv["last"] is not None:
                    last = conv["last"] if last is None else max(last, conv["last"])
                session_span[sid] = (first, last)
                if "subagents" not in path.parts:
                    first, last = copies[sid].get(idx, (None, None))
                    if conv["first"] is not None:
                        first = conv["first"] if first is None else min(first, conv["first"])
                    if conv["last"] is not None:
                        last = conv["last"] if last is None else max(last, conv["last"])
                    copies[sid][idx] = (first, last)
                if "subagents" in path.parts:
                    # A subagent's usage is the account's usage; it is
                    # not a conversation of its own.
                    acc["subagent_files"] += 1
                    records.update(conv["usage"])
                    lines += conv["usage_lines"]
                    continue
                age = now - conv["mtime"]
                acc["changed"]["1h"] += age < 3600
                acc["changed"]["24h"] += age < 86400
                transcripts_by_session[conv["session"]] = (idx, conv)
                if conv["cwd"]:
                    newest_in_folder[(idx, os.path.realpath(conv["cwd"]))].append((conv["mtime"], conv["session"]))
                records.update(conv["usage"])
                lines += conv["usage_lines"]
                if conv["messages"] == 0 or not conv["cwd"]:
                    acc["empty_files"] += 1
                    continue
                acc["conversations"] += 1
                root, how = project_of(conv["cwd"])
                p = projects[root]
                p["how"].add(how)
                p["per_account"][idx] += 1
                p["claude_sessions"].add(sid)
                p["cwds"].add(conv["cwd"])
                p["last"] = max(p["last"], conv["last"] or conv["mtime"])
            claude_records[idx], claude_lines[idx] = records, lines
        else:
            newest = None
            window_tokens = {name: 0 for name, _ in WINDOWS}
            threads = set()  # a resumed thread is a rollout per resume: one conversation
            # A thread's use is its `token_usage_record`s (one per model
            # response, compactions included), or, in a rollout without
            # them, its turns' `last_token_usage`. A record counts once by
            # its key, a turn by (timestamp, running total): a resume, or a
            # subagent forked from its parent's history, repeats what it
            # carries, and each counts on the thread that wrote it first.
            seen_turns = set()
            acc["files"] = {"plain": 0, "compressed": 0}
            all_time = acc["codex_all_time"] = {"turns": 0, "total": 0, **{name: 0 for name, _ in CODEX_CLASSES},
                                                "by_model": {}}
            for path in rollout_files(folder / "sessions", args.plain_rollouts_only):
                roll = read_codex_rollout(path)
                acc["files"]["compressed" if roll["compressed"] else "plain"] += 1
                age = now - roll["mtime"]
                acc["active_threads"] = acc.get("active_threads", 0) + (age < args.live_minutes * 60)
                acc["changed"]["1h"] += age < 3600
                acc["changed"]["24h"] += age < 86400
                if roll["session"] and roll["cwd"]:
                    newest_in_folder[(idx, os.path.realpath(roll["cwd"]))].append((roll["mtime"], roll["session"]))
                thread = roll["session"] or str(path)
                units = roll["records"] or [(ts, ("turn", ts, running), n, detail)
                                            for (ts, n, running), detail in zip(roll["turns"], roll["turn_detail"])]
                own, own_detail = [], []
                for ts, key, n, detail in units:
                    if key not in seen_turns:
                        seen_turns.add(key)
                        own.append((ts, n))
                        own_detail.append(detail)
                for (_, n), (model, classes) in zip(own, own_detail):
                    all_time["turns"] += 1
                    all_time["total"] += n
                    per_model = all_time["by_model"].setdefault(
                        model or "unknown", {"turns": 0, "total": 0, **{name: 0 for name, _ in CODEX_CLASSES}})
                    per_model["turns"] += 1
                    per_model["total"] += n
                    for name, _ in CODEX_CLASSES:
                        all_time[name] += classes[name]
                        per_model[name] += classes[name]
                if roll["session"]:
                    transcripts_by_session[roll["session"]] = (idx, roll)
                    session_mtime[roll["session"]] = max(session_mtime.get(roll["session"], 0), roll["mtime"])
                    if own:
                        session_totals[roll["session"]] = session_totals.get(roll["session"], 0) + sum(n for _, n in own)
                if roll["rate_ts"] and (newest is None or roll["rate_ts"] > newest[0]):
                    newest = (roll["rate_ts"], roll["rate_limits"])
                acc.setdefault("points", []).extend(own)
                for name, span in WINDOWS:
                    window_tokens[name] += codex_window_tokens(own, now - span)
                if not roll["cwd"]:
                    acc["empty_files"] += 1
                    continue
                root, how = project_of(roll["cwd"])
                p = projects[root]
                if roll["parent"]:
                    # A subagent's thread: its tokens are its own session's,
                    # but it is not a conversation of its own.
                    acc["subagent_files"] += 1
                elif thread not in threads:
                    threads.add(thread)
                    acc["conversations"] += 1
                    p["per_account"][idx] += 1
                p["how"].add(how)
                p["cwds"].add(roll["cwd"])
                p["last"] = max(p["last"], roll["totals"][-1][0] if roll["totals"] else roll["mtime"])
            acc["tokens"] = {name: {"total": v} for name, v in window_tokens.items()}
            if newest:
                acc["codex_limits"] = codex_limits(newest[1], newest[0], now)
        per_account[idx] = acc

    # A message in several accounts' copies of one conversation belongs to
    # the copy it was first written in: the one whose last message is
    # earliest (the copy a switch left), then the earlier first, then the
    # account's order.
    inf = float("inf")
    carried = 0
    for sid, held in copies.items():
        if len(held) < 2:
            continue
        order = sorted(held, key=lambda i: (inf if held[i][1] is None else held[i][1],
                                            inf if held[i][0] is None else held[i][0], i))
        for n, later in enumerate(order[1:], 1):
            for key in [k for k in claude_records[later] if any(k in claude_records[o] for o in order[:n])]:
                carried += sum(claude_records[later].pop(key)[1].values())
    for idx, records in claude_records.items():
        acc = per_account[idx]
        # Each message's (time, tokens), for a compare that cuts at the
        # moment the backend read the files.
        acc["points"] = [(ts, sum(t.values())) for ts, t in records.values()]
        for name, span in WINDOWS:
            acc["tokens"][name] = sum_tokens((ts, t) for ts, t in records.values() if now - ts < span)
            acc["tokens_per_line"][name] = sum_tokens((ts, t) for ts, t in claude_lines[idx] if now - ts < span)
    carried_ids = sorted(sid for sid, held in copies.items() if len(held) > 1)
    carried_sessions = len(carried_ids)

    live = []
    for proc in processes():
        entry = dict(proc)
        entry["account"], entry["project"], entry["matched_by"] = None, None, None
        if proc["resume"] and proc["resume"] in transcripts_by_session:
            entry["account"] = transcripts_by_session[proc["resume"]][0]
            entry["matched_by"] = "--resume id"
        elif proc["cwd"]:
            here = os.path.realpath(proc["cwd"])
            best = None
            for (idx, folder), items in newest_in_folder.items():
                if folder != here or accounts[idx][1] != proc["tool"]:
                    continue
                mtime, _ = max(items)
                if best is None or mtime > best[0]:
                    best = (mtime, idx)
            if best:
                fresh = now - best[0] < args.live_minutes * 60
                entry["account"] = best[1]
                entry["matched_by"] = ("newest transcript in its folder" if fresh else
                                       f"its folder's transcripts, newest {round((now - best[0]) / 60)} min old")
        if proc["cwd"]:
            entry["project"] = project_of(proc["cwd"])[0]
        live.append(entry)

    # A message a resumed copy repeats belongs to one session: the one with
    # the earliest first message (then the earlier last message, then the
    # id), as the app credits it. Account totals don't change.
    holders = defaultdict(list)
    for session, records in session_usage.items():
        for key in records:
            holders[key].append(session)

    def rank(session):
        first, last = session_span.get(session, (None, None))
        return (inf if first is None else first, inf if last is None else last, session)

    owned = defaultdict(dict)
    for key, sessions in holders.items():
        owner = min(sessions, key=rank)
        owned[owner][key] = session_usage[owner][key]
    for session, records in owned.items():
        if records:
            session_totals[session] = sum_tokens(records.values())["total"]
    return {"generated": now, "accounts": accounts, "per_account": per_account, "projects": projects,
            "carried": {"sessions": carried_sessions, "tokens": carried, "ids": carried_ids},
            "codex_sessions": sorted(s for s, (i, _) in transcripts_by_session.items() if accounts[i][1] == "codex"),
            "live": live, "session_totals": session_totals, "session_mtime": session_mtime, "desktop_roots": desktop_roots()}


def sum_tokens(items) -> dict:
    out = {"input": 0, "output": 0, "cache_write": 0, "cache_read": 0, "messages": 0}
    for _, t in items:
        for k in ("input", "output", "cache_write", "cache_read"):
            out[k] += t[k]
        out["messages"] += 1
    out["total"] = out["input"] + out["output"] + out["cache_write"] + out["cache_read"]
    return out


def codex_window_tokens(turns: list, since: float) -> int:
    """Tokens a rollout's own turns used after `since` (each turn's `last_token_usage`)."""
    return sum(n for ts, n in turns if ts >= since)


def codex_limits(rl: dict, at: float, now: float) -> dict:
    def window(w):
        if not isinstance(w, dict):
            return None
        used = w.get("used_percent")
        resets = w.get("resets_at")
        return {"used_percent": used, "left_percent": None if used is None else round(100 - used, 1),
                "window_minutes": w.get("window_minutes"),
                "resets_at": time.strftime("%a %Y-%m-%d %H:%M", time.localtime(resets)) if isinstance(resets, (int, float)) else None,
                "reset_passed": isinstance(resets, (int, float)) and resets < now}
    credits = rl.get("credits") if isinstance(rl.get("credits"), dict) else None
    return {"read_at": time.strftime("%Y-%m-%d %H:%M", time.localtime(at)), "age_minutes": max(0, round((now - at) / 60)),
            "primary": window(rl.get("primary")), "secondary": window(rl.get("secondary")),
            "plan_type": rl.get("plan_type"),
            "credits": None if credits is None else {"has_credits": credits.get("has_credits"),
                                                     "unlimited": credits.get("unlimited"),
                                                     "balance": "present" if credits.get("balance") is not None else None},
            "limit_reached": rl.get("rate_limit_reached_type")}


# ---------------------------------------------------------------------------
# Redacted report
# ---------------------------------------------------------------------------


def report(truth: dict, key_file: str | None, as_json: bool, live_minutes: int = 15) -> str:
    accounts = truth["accounts"]
    acc_name = {i: f"Account {letters(i)}" for i in range(len(accounts))}
    ranked = sorted(truth["projects"].items(), key=lambda kv: (-sum(kv[1]["per_account"].values()), kv[0]))
    proj_name = {root: f"Project {n + 1}" for n, (root, _) in enumerate(ranked)}
    if key_file:
        key = {"accounts": {acc_name[i]: f"{a[1]} {a[0]}" for i, a in enumerate(accounts)},
               "projects": {proj_name[r]: r.replace(str(HOME), "~") for r, _ in ranked}}
        Path(key_file).write_text(json.dumps(key, indent=1))
    now = truth["generated"]
    data = {
        "generated": time.strftime("%Y-%m-%d %H:%M", time.localtime(now)),
        "accounts": [{"name": acc_name[i], "tool": a[1], **{k: v for k, v in truth["per_account"][i].items()
                                                            if k not in ("label", "tool")}}
                     for i, a in enumerate(accounts)],
        "projects": [{"name": proj_name[r], "kinds": sorted(p["how"]), "folders": len(p["cwds"]),
                      "conversations": {acc_name[i]: n for i, n in sorted(p["per_account"].items())},
                      "last_active_min_ago": round((now - p["last"]) / 60)} for r, p in ranked],
        "live": [{"tool": l["tool"], "kind": l["kind"], "app": l["app"],
                  "account": acc_name.get(l["account"], "unmatched"), "matched_by": l["matched_by"],
                  "project": proj_name.get(l["project"], "a folder with no transcripts") if l["project"] else "unknown"}
                 for l in truth["live"]],
    }
    if as_json:
        return json.dumps(data, indent=1)
    out = [f"Ground truth at {data['generated']} (read only, redacted)", ""]
    out.append("Accounts")
    for a in data["accounts"]:
        line = f"- {a['name']} ({a['tool']}): {a['conversations']} conversations; changed in 1 h {a['changed']['1h']}, 24 h {a['changed']['24h']}"
        if a["empty_files"]:
            line += f"; {a['empty_files']} files with no messages"
        out.append(line)
        for w in ("5h", "7d"):
            t = a["tokens"].get(w, {})
            if a["tool"] == "claude":
                per_line = a["tokens_per_line"][w]
                out.append(f"    {w}: {t['messages']} messages · in {t['input']:,} · out {t['output']:,} · cache write {t['cache_write']:,} · cache read {t['cache_read']:,}"
                           f"  (summed per line instead: out {per_line['output']:,})")
            else:
                out.append(f"    {w}: {t.get('total', 0):,} tokens (its turns' last_token_usage)")
        if a["tool"] == "codex":
            files, every = a.get("files", {}), a.get("codex_all_time", {})
            out.append(f"    rollouts: {files.get('plain', 0)} plain, {files.get('compressed', 0)} compressed (.jsonl.zst); "
                       f"all time {every.get('turns', 0):,} turns, {every.get('total', 0):,} tokens "
                       f"(in {every.get('input', 0):,} of which cached {every.get('cached_input', 0):,} · "
                       f"out {every.get('output', 0):,} of which reasoning {every.get('reasoning', 0):,})")
        lim = a.get("codex_limits")
        if lim:
            p = lim["primary"]
            out.append(f"    limits (read {lim['age_minutes']} min ago): primary {p and p['used_percent']}% used = {p and p['left_percent']}% left, "
                       f"window {p and p['window_minutes']} min, resets {p and p['resets_at']}{' (passed)' if p and p['reset_passed'] else ''}; "
                       f"secondary {lim['secondary'] or 'none'}; plan {lim['plan_type']}; credits {lim['credits']}")
    carried = truth.get("carried") or {}
    if carried.get("sessions"):
        out.append(f"- {carried['sessions']} conversation(s) carried into another account by a switch: "
                   f"{carried['tokens']:,} tokens of their earlier messages counted once, on the account they came from")
    out += ["", f"Projects ({len(data['projects'])}; agent worktrees folded into their repo)"]
    for p in data["projects"]:
        convs = ", ".join(f"{k} {v}" for k, v in p["conversations"].items())
        out.append(f"- {p['name']}: {sum(p['conversations'].values())} conversations ({convs}); "
                   f"{p['folders']} folder(s): {', '.join(p['kinds'])}; last active {p['last_active_min_ago']} min ago")
    sessions = [l for l in data["live"] if l["kind"] == "session"]
    helpers = [l for l in data["live"] if l["kind"] != "session"]
    out += ["", f"Live sessions ({len(sessions)} CLI processes; helpers not counted: "
                f"{', '.join(sorted(l['tool'] + ' ' + l['kind'] + ' in ' + l['app'] for l in helpers)) or 'none'})"]
    for a in data["accounts"]:
        if a["tool"] == "codex":
            out.append(f"- {a['name']}: {a.get('active_threads', 0)} threads written in the last {live_minutes} min "
                       "(Codex in an app runs threads inside its app-server, not as processes)")
    for l in sessions:
        out.append(f"- {l['tool']} in {l['app']}: {l['account']} ({l['matched_by'] or 'no transcript match'}), {l['project']}")
    return "\n".join(out)


# ---------------------------------------------------------------------------
# Codex rate_limits over time (how fresh "as of" can honestly be)
# ---------------------------------------------------------------------------


def codex_history(dirs: list[Path], now: float) -> str:
    """Every `token_count.rate_limits` reading in every rollout, as cadence facts."""
    out = []
    for n, home in enumerate(dirs):
        ev = []  # (ts, thread, used %, resets_at, window minutes, plan)
        for t, path in enumerate(rollout_files(home / "sessions")):
            for raw in rollout_lines(current_rollout(path)):
                if b'"rate_limits"' not in raw:
                    continue
                try:
                    line = json.loads(raw)
                except ValueError:
                    continue
                payload = line.get("payload") if isinstance(line, dict) else None
                if not isinstance(payload, dict) or payload.get("type") != "token_count":
                    continue
                rl = payload.get("rate_limits")
                ts = iso_to_epoch(line.get("timestamp"))
                if not isinstance(rl, dict) or ts is None:
                    continue
                pr = rl.get("primary") if isinstance(rl.get("primary"), dict) else {}
                ev.append((ts, t, pr.get("used_percent"), pr.get("resets_at"), pr.get("window_minutes"),
                           rl.get("plan_type"), rl.get("secondary") is not None))
        if not ev:
            continue
        ev.sort(key=lambda e: e[0])
        q = lambda xs, f: sorted(xs)[min(len(xs) - 1, int(f * len(xs)))] if xs else None  # noqa: E731
        gaps = [b[0] - a[0] for a, b in zip(ev, ev[1:])]
        same = [b[0] - a[0] for a, b in zip(ev, ev[1:]) if a[1] == b[1]]
        changes = [b[0] for a, b in zip(ev, ev[1:]) if a[2] != b[2]]
        between = [y - x for x, y in zip(changes, changes[1:])]
        resets = sorted({e[3] for e in ev if isinstance(e[3], (int, float))})
        windows = []
        for r in resets:  # one window per reset, allowing for jitter
            if windows and r - windows[-1][-1] < 3600:
                windows[-1].append(r)
            else:
                windows.append([r])
        wid = {r: i for i, w in enumerate(windows) for r in w}
        timed = [e for e in ev if e[3] in wid]
        jitter = [abs(b[3] - a[3]) for a, b in zip(timed, timed[1:]) if a[3] != b[3] and wid[a[3]] == wid[b[3]]]
        back = [(a, b) for a, b in zip(timed, timed[1:]) if wid[b[3]] < wid[a[3]]]
        lower = [(a, b) for a, b in zip(timed, timed[1:])
                 if wid[a[3]] == wid[b[3]] and a[2] is not None and b[2] is not None and b[2] < a[2]]
        first_seen = {}
        for e in timed:
            first_seen.setdefault(wid[e[3]], e[0])
        late = [e[0] - first_seen[wid[e[3]] + 1] for e in timed
                if wid[e[3]] + 1 in first_seen and e[0] > first_seen[wid[e[3]] + 1]]
        name = f"Account {letters(n)} (codex)" if len(dirs) > 1 else "The Codex account"
        out += [f"{name}: {len(ev):,} readings in {len({e[1] for e in ev})} threads, "
                f"{time.strftime('%Y-%m-%d', time.localtime(ev[0][0]))} → {time.strftime('%Y-%m-%d %H:%M', time.localtime(ev[-1][0]))}",
                f"- cadence: every {q(same, .5)} s (median; p90 {q(same, .9)} s) while a thread works; "
                f"gaps over 1 h: {sum(g > 3600 for g in gaps)}, over 8 h: {sum(g > 8 * 3600 for g in gaps)}, longest {max(gaps) / 3600:.0f} h",
                f"- used_percent: whole numbers; changed {len(changes)} times, every {q(between, .5) / 60:.0f} min (median; p90 "
                f"{q(between, .9) / 60:.0f} min) between changes" if between else "- used_percent: never changed",
                f"- windows: {sorted({str(e[4]) for e in ev})} minutes; secondary present in {sum(e[6] for e in ev)}; "
                f"plan {sorted({str(e[5]) for e in ev})}; {len(windows)} distinct windows (resets_at jitters up to {max(jitter) if jitter else 0} s inside one)",
                f"- conflicting readings: {len(lower)} lower than the reading before in the same window "
                f"({sum(a[1] != b[1] for a, b in lower)} from another thread); {len(back)} back to an OLDER window "
                f"({sum(a[1] == b[1] for a, b in back)} inside one thread), up to {max(late) / 3600:.0f} h after the newer window appeared"
                if late or lower or back else "- conflicting readings: none",
                f"- newest reading: {(now - ev[-1][0]) / 60:.0f} min old; null primary in {sum(e[2] is None for e in ev)}, "
                f"window_minutes 0 in {sum(e[4] == 0 for e in ev)}, plan null in {sum(e[5] is None for e in ev)}"]
    return "\n".join(out) or "No Codex rate_limits found."


# ---------------------------------------------------------------------------
# Compare with what the backend returns (crates/juggler-app/tests/qa_real_readonly.rs)
# ---------------------------------------------------------------------------



def vm_stat_memory(text: str):
    """(used, page size) from `vm_stat`: app memory (anonymous less
    purgeable) + wired + compressed, as Activity Monitor counts "Memory Used"."""
    import re as _re
    page = int((_re.search(r"page size of (\d+) bytes", text) or [0, 16384])[1])
    def pages(name):
        m = _re.search(rf"^{_re.escape(name)}:\s+(\d+)\.", text, _re.M)
        return int(m.group(1)) if m else 0
    app = pages("Anonymous pages") - pages("Pages purgeable")
    return (app + pages("Pages wired down") + pages("Pages occupied by compressor")) * page, page


def verdict_vm(verdict, machine: dict, vm: dict) -> None:
    """3.4: This Mac's memory, total and pressure against macOS's own figures."""
    used, _ = vm_stat_memory(vm.get("vm_stat") or "")
    total = int(vm.get("memsize") or 0)
    if not used or not total:
        verdict(None, "This Mac vs vm_stat: macOS figures not readable")
        return
    mine = machine.get("memory_used") or 0
    verdict(abs(mine - used) <= 0.03 * total,
            f"This Mac memory vs vm_stat: backend {mine / 1e9:.1f} GB used, vm_stat app+wired+compressed {used / 1e9:.1f} GB "
            f"(off by {abs(mine - used) / total * 100:.1f}% of {total / 1e9:.0f} GB)")
    verdict(machine.get("memory_total") == total,
            f"This Mac total memory: backend {machine.get('memory_total', 0) / 1e9:.1f} GB, hw.memsize {total / 1e9:.1f} GB")
    level = {"1": "normal", "2": "warn", "4": "critical"}.get(str(vm.get("pressure_level")).strip(), "?")
    seen = str(machine.get("pressure") or "").lower()
    verdict(seen.startswith(level[:4]) or (level == "warn" and seen.startswith("warning")),
            f"This Mac pressure: backend {machine.get('pressure')}, kern.memorystatus_vm_pressure_level {vm.get('pressure_level')} ({level})")

def compare(truth: dict, harness: dict) -> str:
    """Redacted OK / MISMATCH lines: the oracle against the backend's read models."""
    accounts = truth["accounts"]
    acc_name = {i: f"Account {letters(i)}" for i in range(len(accounts))}
    # One spelling per folder: the backend resolves symlinks (/tmp is
    # /private/tmp on a Mac), so the comparison does too.
    def real(path) -> str:
        return os.path.realpath(str(path)).rstrip("/")

    folder_idx = {real(a[2]): i for i, a in enumerate(accounts)}
    by_id = {a["id"]: folder_idx.get(real(a["state_dir"])) for a in harness["accounts"]}
    by_label = {a["label"]: by_id[a["id"]] for a in harness["accounts"]}
    ranked = sorted(((real(root), p) for root, p in truth["projects"].items()),
                    key=lambda kv: (-sum(kv[1]["per_account"].values()), kv[0]))
    proj_name = {root: f"Project {n + 1}" for n, (root, _) in enumerate(ranked)}
    now_ms = harness["generated_ms"]
    lines = []

    def verdict(ok, text: str) -> None:
        # ok: True / False, or None for a fact that is neither (INFO).
        lines.append(("INFO      " if ok is None else "OK        " if ok else "MISMATCH  ") + text)

    # First run: every setup found, every bring-in done.
    found = {real(i["path"]) for i in harness["scan"]["installs"]}
    missing = [acc_name[i] for f, i in folder_idx.items() if f not in found]
    verdict(not missing, f"setups found: {len(found)} of {len(accounts)}" + (f" (missing {', '.join(missing)})" if missing else ""))
    failed = [i for i in harness["imports"] if not i["ok"]]
    verdict(not failed, f"brought in where they are: {len(harness['imports']) - len(failed)} of {len(harness['imports'])}")

    # Projects: every backend row should be one oracle project; counts per project.
    rows = defaultdict(list)
    for row in harness["projects"]:
        rows[real(project_of(row["root"])[0])].append(row)
    verdict(len(harness["projects"]) == len(ranked),
            f"projects: backend {len(harness['projects'])} rows, oracle {len(ranked)}")
    for root, p in ranked:
        # A conversation a switch carried into another account is one
        # conversation, in both accounts' lists.
        claude_convs = len(p["claude_sessions"])
        carried = sum(n for i, n in p["per_account"].items() if accounts[i][1] == "claude") - claude_convs
        codex_convs = sum(n for i, n in p["per_account"].items() if accounts[i][1] == "codex")
        got = rows.get(root, [])
        shown = sum(r["history_sessions"] or 0 for r in got)
        extra = [r for r in got if real(project_of(r["root"])[0]) != real(r["root"]) or real(r["root"]) != root]
        names = sorted({acc_name[by_label[l]] for r in got for l in r["accounts_used"] if by_label.get(l) is not None})
        want = sorted({acc_name[i] for i in p["per_account"]})
        detail = (f"{proj_name[root]}: backend {shown} conversations in {len(got)} row(s)"
                  f"{' (' + str(len(extra)) + ' split off as their own projects)' if extra else ''}, accounts {names or '-'}; "
                  f"oracle {claude_convs} Claude{f' ({carried} also in another account)' if carried else ''} + {codex_convs} Codex, accounts {want}")
        verdict(len(got) == 1 and shown == claude_convs + codex_convs and names == want, detail)

    carried = truth.get("carried") or {}
    if carried.get("sessions"):
        verdict(None, f"switch copies: {carried['sessions']} conversation(s) also in another account; "
                      f"{carried['tokens']:,} tokens of their earlier messages count once, on the account they came from "
                      "(windows and cost rows that count them again read high)")

    # Sessions: the rail's list against what is running.
    live = [l for l in truth["live"] if l["kind"] == "session"]
    threads = sum(truth["per_account"][i].get("active_threads", 0) for i in range(len(accounts)))
    convs = harness.get("conversations")
    if convs is None:
        verdict(len(harness["sessions"]) >= len(live),
                f"sessions listed: backend {len(harness['sessions'])}; running {len(live)} CLI sessions + {threads} active Codex threads")
    else:
        # AJ-E1529: every account's conversations, and the live ones, from
        # conversations_list (sessions started outside AI Juggler included).
        per, on = defaultdict(int), defaultdict(int)
        for c in convs:
            i = by_id.get(c["account_id"])
            per[i] += 1
            on[i] += 1 if c["live"] else 0
        running = defaultdict(int)
        for l in live:
            if l.get("account") is not None:
                running[l["account"]] += 1
        for i in range(len(accounts)):
            want = truth["per_account"][i]["conversations"]
            verdict(per[i] == want, f"{acc_name[i]} conversations: backend {per[i]}, oracle {want}")
            verdict(on[i] == running[i], f"{acc_name[i]} live: backend {on[i]}, running {running[i]}")
        # A live session the oracle can't put on an account would drop out of
        # both sides above: it must fail here instead of passing unseen.
        unmatched = [l for l in live if l.get("account") is None]
        verdict(not unmatched, f"live sessions with no account in the oracle: {len(unmatched)}"
                + (f" ({', '.join(sorted({l['tool'] + ' in ' + l['app'] for l in unmatched}))})" if unmatched else ""))
    stand_ins = [l for l in truth["live"] if l["kind"] == "stand-in"]
    if stand_ins:
        verdict(None, f"left out: {len(stand_ins)} stand-in CLI(s) run from a temp folder (tests, not sessions)")
    host = [l for l in truth["live"] if l["kind"] == "host"]
    if host:
        verdict(None, f"left out: {len(host)} of this Mac's own session(s), working outside the replica's HOME")

    # Quota: percent, reset, tokens per window.
    for q in harness["quota_display_all"]:
        i = by_id.get(q["account"])
        if i is None:
            continue
        acc = truth["per_account"][i]
        name = acc_name[i]
        for key, window in (("five_hour", "5h"), ("weekly", "7d")):
            w = q.get(key)
            if not w:
                lim = (acc.get("codex_limits") or {}) if acc["tool"] == "codex" else {}
                if key == "five_hour" and lim and lim.get("secondary") is None:
                    verdict(True, f"{name} 5-hour window: none, as Codex reports (secondary null)")
                continue
            pct = w.get("percentage")
            if acc["tool"] == "codex":
                lim = acc.get("codex_limits") or {}
                prim = lim.get("primary") or {}
                weekly = prim.get("window_minutes") == 10080
                if key == "weekly" and weekly:
                    verdict(pct is not None and abs(pct - prim["used_percent"]) < 1.5,
                            f"{name} weekly: backend {pct if pct is not None else 'no percent'}; rollouts say {prim.get('used_percent')}% used, resets {prim.get('resets_at')}")
                if key == "five_hour" and lim and lim.get("secondary") is None and weekly:
                    verdict(False, f"{name} 5-hour window: backend tracks one ({w['state']}); Codex reports no 5-hour window (secondary null)")
            elif pct is not None:
                verdict(True, f"{name} {window}: backend {pct}% used (Measured sample)")
            elif w.get("resets_at_ms") is None:
                verdict(True, f"{name} {window}: no percent and no reset time (nothing measured)")
            resets = w.get("resets_at_ms")
            if resets is not None and abs(resets - now_ms) < 5 * 60_000:
                verdict(False, f"{name} {window}: resets at {round((resets - now_ms) / 60000, 1)} min from now — the window's start + its length, not a vendor reset")
            elif resets is not None and resets < now_ms:
                verdict(False, f"{name} {window}: reset time already passed ({round((now_ms - resets) / 60000)} min ago)")
            truth_tokens = (acc["tokens"].get(window) or {}).get("total")
            band = None
            if "ingested_ms" in harness and acc.get("points") is not None:
                # Only what was on disk when the backend read it, in the window
                # as it stood then: live writers after that aren't a miss.
                cut = harness["ingested_ms"] / 1000
                span = dict(WINDOWS)[window]
                truth_tokens = sum(n for ts, n in acc["points"] if cut - span <= ts <= cut)
                done = harness.get("ingest_done_ms")
                if done is not None:
                    # Live writers during the passes: anything between the
                    # files as they were at the start and at the end is right.
                    end = done / 1000
                    upper = sum(n for ts, n in acc["points"] if end - span <= ts <= end)
                    burned_now = w.get("burned_tokens")
                    if upper != truth_tokens:
                        band = (truth_tokens, upper)
                    if burned_now is not None and truth_tokens <= burned_now <= upper:
                        truth_tokens = burned_now
            burned = w.get("burned_tokens")
            if truth_tokens is not None and burned is not None:
                ratio = burned / truth_tokens if truth_tokens else (0 if burned == 0 else float("inf"))
                said = (f"oracle {band[0]:,} at the ingest's start to {band[1]:,} at its end (live writers)"
                        if band else f"oracle {truth_tokens:,}")
                verdict(0.9 <= ratio <= 1.1 or (truth_tokens == 0 and burned == 0),
                        f"{name} {window} tokens: backend {burned:,}, {said} (×{ratio:.2f})")

    # Usage screen payload: nulls the UI's `!== undefined` guards let through.
    cost = harness["cost_rollups"]
    nulls = [k for k in ("api_equivalent_usd", "flat_fee_usd", "value_ratio") if cost["subscription"].get(k, "absent") is None]
    if cost.get("total_usd", "absent") is None:
        nulls.append("total_usd")
    null_rows = sum(1 for r in cost["rows"] if r.get("credits", "absent") is None or (r.get("usd") or {}).get("usd", "absent") is None)
    verdict(None if nulls or null_rows else True,
            f"cost payload: null {', '.join(nulls) or 'none'}; {null_rows} of {len(cost['rows'])} rows with null credits/usd"
            " (honest nulls; main's CostMatrix.tsx crashes on them until its guards check for null)")
    backend_sessions = defaultdict(int)
    for r in cost["rows"]:
        t = r.get("tokens") or {}
        # Every class: disjoint classes sum to the tool's own total (on main,
        # Codex's input still includes its cached input, which shows as about x2).
        backend_sessions[r["key"]] += sum(t.get(k, 0) for k in ("input", "output", "cache_read", "cache_write", "reasoning"))
    # Sessions written after the backend read them would differ for that
    # reason alone: leave them out (the harness records when it ingested;
    # older outputs: 5 minutes before they were written).
    ingested_s = harness.get("ingested_ms", harness["generated_ms"] - 300_000) / 1000
    oracle_sessions = {k: v for k, v in truth["session_totals"].items()
                       if truth["session_mtime"].get(k, 0) < ingested_s}
    skipped = len(truth["session_totals"]) - len(oracle_sessions)
    # A switch copy's earlier lines are left out by the app through its own
    # switch records, which a fresh data folder doesn't have: not checkable.
    carried_ids = set((truth.get("carried") or {}).get("ids") or [])
    both = [k for k in backend_sessions if k in oracle_sessions and oracle_sessions[k] and k not in carried_ids]
    copies_seen = [k for k in backend_sessions if k in oracle_sessions and k in carried_ids]
    if copies_seen:
        verdict(None, f"cost rows of {len(copies_seen)} switch copies: not checkable without the switch records "
                      "(a fresh data folder has none), left out of the check below")
    ratios = sorted(backend_sessions[k] / oracle_sessions[k] for k in both)
    close = sum(1 for r in ratios if 0.95 <= r <= 1.05)
    only_backend = len([k for k in backend_sessions if k not in truth["session_totals"]])
    # A session with tokens and no cost row is history the backend never
    # read (e.g. Codex's compressed rollouts); one with no tokens has no row.
    only_oracle = len([k for k, v in oracle_sessions.items() if v and k not in backend_sessions])
    if ratios:
        verdict(close == len(ratios) and only_backend == 0 and only_oracle == 0,
                f"cost rows per session: {len(both)} sessions in both; {close} within 5% of the transcripts; "
                f"ratio min ×{ratios[0]:.2f}, median ×{ratios[len(ratios) // 2]:.2f}, max ×{ratios[-1]:.2f}; "
                f"{only_backend} only in the backend, {only_oracle} with tokens only in the transcripts; "
                f"{skipped} still being written during the run, left out")
    # All of Codex's history, summed: the sessions both read (left out:
    # those written during the run).
    codex = [k for k in truth.get("codex_sessions", []) if k in oracle_sessions]
    if codex:
        mine = sum(oracle_sessions[k] for k in codex)
        theirs = sum(backend_sessions.get(k, 0) for k in codex)
        verdict(abs(theirs - mine) <= 0.05 * mine,
                f"Codex all time: backend {theirs:,} tokens in {sum(1 for k in codex if k in backend_sessions)} sessions, "
                f"oracle {mine:,} in {len(codex)} (×{theirs / mine:.2f})")
    unpriced = sum(1 for r in cost["rows"] if (r.get("usd") or {}).get("usd") is None)
    verdict(unpriced == 0, f"cost rows priced: {len(cost['rows']) - unpriced} of {len(cost['rows'])}")

    # This Mac (the tools part needs the app's process collector, which a test doesn't run).
    if harness.get("this_mac"):
        m = harness["this_mac"][-1]
        verdict(True, f"This Mac machine: CPU {round(m['machine'].get('cpu_pct') or -1)}%, memory {m['machine']['memory_used'] / 1e9:.1f} of "
                      f"{m['machine']['memory_total'] / 1e9:.0f} GB ({m['machine']['memory_basis']}), pressure {m['machine']['pressure']}; "
                      f"tools {m['tools']['count']}")
        vm = m.get("qa_vm")
        if vm:
            verdict_vm(verdict, m["machine"], vm)
    # This Mac counts the whole Mac. On a replica (a HOME that isn't this
    # user's own) the host's apps are in it too, so these lines are facts
    # there, not checks.
    replica = os.path.realpath(pwd.getpwuid(os.getuid()).pw_dir) != os.path.realpath(HOME)

    def mac_verdict(ok, text: str) -> None:
        verdict(None if replica else ok, text + (" (a replica: This Mac counts the whole Mac)" if replica else ""))

    # This Mac's tools against `ps` trees of the desktop apps' main processes
    # and the CLIs outside them, from the ps rows the harness took with its last read.
    panel = (harness.get("this_mac") or [{}])[-1]
    ps_rows = panel.get("qa_ps") or []
    if ps_rows:
        children = defaultdict(list)
        rss = {}
        for row in ps_rows:
            pid, ppid, kb = row[0], row[1], row[2]
            if pid is None:
                continue
            children[ppid].append(pid)
            rss[pid] = (kb or 0) * 1024
        # The app leaves its own process tree out (here, the test that ran
        # it, often inside a Claude app session): so does the ps side.
        own = harness.get("qa_pid")
        if own in rss:
            todo = [own]
            while todo:
                p = todo.pop()
                if rss.pop(p, None) is not None:
                    todo += children.get(p, [])

        def tree(pid):
            todo, seen = [pid], set()
            while todo:
                p = todo.pop()
                if p in seen or p not in rss:
                    continue
                seen.add(p)
                todo += children.get(p, [])
            return sum(rss[p] for p in seen)

        # The desktop apps' main processes as they were during the run, when
        # the snapshot names them; else as they are now.
        named = {row[0]: row[5] for row in ps_rows if len(row) >= 6 and row[0] is not None}
        parent = {row[0]: row[1] for row in ps_rows if row[0] is not None}
        if named:
            roots = [{"pid": pid, "app": name} for pid, name in named.items()
                     if name in ("Claude", "ChatGPT") and named.get(parent.get(pid)) != name and pid in rss]
        else:
            roots = [r for r in truth["desktop_roots"] if r["pid"] in rss]
        claude = sorted((tree(r["pid"]) for r in roots if r["app"] == "Claude"), reverse=True)
        chatgpt = [tree(r["pid"]) for r in roots if r["app"] == "ChatGPT"]
        tools = panel.get("tools") or {}
        mac_verdict(tools.get("count") == len(claude),
                f"This Mac tools: backend {tools.get('count')}; ps has {len(claude)} Claude app instances"
                f"{' + ' + str(len(chatgpt)) + ' ChatGPT (Codex) not counted by design' if chatgpt else ''}")
        total = tools.get("rss_bytes") or 0
        want = sum(claude)
        mac_verdict(want and 0.85 <= total / want <= 1.15,
                f"This Mac tools memory: backend {total / 1e9:.1f} GB; ps RSS of the Claude app trees {want / 1e9:.1f} GB"
                + (f" (×{total / want:.2f})" if want else ""))
        top = [r.get("rss_bytes") or 0 for r in panel.get("top") or []]
        mac_verdict(len(top) == min(3, len(claude)) and all(abs(a - b) <= 0.15 * b for a, b in zip(top, claude)),
                f"This Mac top rows (GB): backend {[round(t / 1e9, 1) for t in top]}; ps trees {[round(t / 1e9, 1) for t in claude[:3]]}"
                f"; {max(0, len(claude) - len(top))} instance(s) not shown")
        others = panel.get("others")
        if others is not None:
            shown = sorted(top + [r.get("rss_bytes") or 0 for r in others], reverse=True)
            mac_verdict(len(shown) == len(claude) and all(abs(a - b) <= 0.15 * b for a, b in zip(shown, claude)),
                    f"This Mac every tree: backend {len(shown)} (top {len(top)} + others {len(others)}) "
                    f"{[round(t / 1e9, 1) for t in shown]}; ps {len(claude)} {[round(t / 1e9, 1) for t in claude]}")
        # AJ-E1528: the This Mac page's groups, by tool: the Claude app's
        # windows against ps, and each group's rows against its own count
        # (the backend says each tree's group; older builds by tool/place).
        groups = panel.get("by_tool") or []
        desktop = next((g for g in groups if not g.get("ours") and g.get("tool") == "desktop"), None)
        verdict(((desktop or {}).get("totals") or {}).get("count", 0) == len(claude),
                f"This Mac page, Claude app group: backend {((desktop or {}).get('totals') or {}).get('count', 0)} windows; ps {len(claude)}")
        every = (panel.get("top") or []) + (panel.get("others") or [])
        def in_group(t: dict, g: dict) -> bool:
            if (t.get("session_id") is not None) != bool(g.get("ours")):
                return False
            if "group" in t:
                return t.get("group") == g.get("tool")
            return (t.get("tool") or "claude-code") == g.get("tool") if g.get("ours") else t.get("place") == g.get("tool")
        for g in groups:
            rows = [t for t in every if in_group(t, g)]
            count = (g.get("totals") or {}).get("count", 0)
            verdict(len(rows) == count, f"This Mac page group {g.get('label')}: {len(rows)} rows for its {count} trees")
        # Rows that name their tree's root: each against that root's ps tree.
        rows_all = list(panel.get("top") or []) + list(others or [])
        paired = [(r.get("root_pid"), r.get("rss_bytes") or 0) for r in rows_all if r.get("root_pid")]
        if paired:
            off = []
            for pid, got in paired:
                want_tree = tree(pid) if pid in rss else None
                if want_tree is None or not (abs(got - want_tree) <= 0.15 * max(want_tree, 1)):
                    off.append((pid, got, want_tree))
            mac_verdict(not off, f"This Mac rows by root: {len(paired) - len(off)} of {len(paired)} within 15% of their own ps tree"
                        + (f"; off: {[(round(g / 1e9, 1), None if w is None else round(w / 1e9, 1)) for _, g, w in off]}" if off else ""))
        cpu = tools.get("cpu_pct")
        mac_verdict(True, f"This Mac tools CPU: backend {cpu if cpu is None else round(cpu, 1)}% of all cores (ps %CPU is a decaying average, not compared)")
    kinds = [a["kind"] for a in harness.get("machine_alerts", [])]
    verdict(len(kinds) == len(set(kinds)), f"alerts over the run: {len(kinds)} ({', '.join(kinds) or 'none'})")
    return "\n".join(lines)


# ---------------------------------------------------------------------------


def selftest() -> None:
    assert iso_to_epoch("1970-01-01T00:01:00.5Z") == 60.0
    assert iso_to_epoch("nope") is None and iso_to_epoch(None) is None
    assert encode_cwd("/Users/a/Dev/x.y") == "-Users-a-Dev-x-y"
    gone = "/nonexistent/qa-selftest"
    assert project_of(f"{gone}/repo/.claude/worktrees/feat-1a2b3c") == (f"{gone}/repo", "removed worktree")
    assert project_of(f"{gone}/repo/.claude/worktrees/feat/sub") == (f"{gone}/repo", "removed worktree")
    assert project_of(f"{gone}/wt", lambda _: "gitdir: /r/main/.git/worktrees/wt\n") == ("/r/main", "worktree")
    assert project_of(f"{gone}/plain", lambda _: None) == (f"{gone}/plain", "missing folder")
    assert [letters(n) for n in (0, 25, 26)] == ["A", "Z", "AA"]
    t = {"input": 1, "output": 2, "cache_write": 3, "cache_read": 4}
    assert sum_tokens([(0, t), (0, t)])["total"] == 20
    turns = [(10, 100), (20, 50), (30, 250)]
    assert codex_window_tokens(turns, 15) == 300 and codex_window_tokens(turns, 0) == 400
    assert codex_window_tokens(turns, 31) == 0
    lim = codex_limits({"primary": {"used_percent": 68.0, "window_minutes": 10080, "resets_at": 0},
                        "credits": {"has_credits": True, "balance": "1"}}, 0, 60)
    assert lim["primary"]["left_percent"] == 32.0 and lim["primary"]["reset_passed"]
    assert lim["credits"]["balance"] == "present" and lim["secondary"] is None
    assert is_temp("/tmp/x/claude") and is_temp("/private/var/folders/ab/T/x/codex")
    assert not is_temp("/Applications/Claude.app/Contents/MacOS/claude")
    assert not is_temp(str(HOME / "bin/claude"))
    assert not is_host(None) and not is_host(str(HOME / "Dev/app"))
    assert dict(WINDOWS)["5h"] == 5 * 3600 and dict(WINDOWS)["7d"] == 7 * 86400
    vm = ("Mach Virtual Memory Statistics: (page size of 16384 bytes)\nPages free: 10.\n"
          "Anonymous pages: 1000.\nPages purgeable: 100.\nPages wired down: 200.\n"
          "Pages occupied by compressor: 300.\n")
    assert vm_stat_memory(vm) == ((1000 - 100 + 200 + 300) * 16384, 16384)
    selftest_rollouts()
    print("real_truth selftest: OK")


def selftest_rollouts() -> None:
    """A compressed rollout reads like its plain twin; both forms of one thread read once."""
    import tempfile
    rows = [{"timestamp": "2026-01-01T00:00:00Z", "type": "session_meta",
             "payload": {"id": "t1", "cwd": "/r", "timestamp": "2026-01-01T00:00:00Z"}},
            {"timestamp": "2026-01-01T00:00:01Z", "type": "turn_context", "payload": {"model": "m1"}},
            {"timestamp": "2026-01-01T00:00:02Z", "type": "event_msg", "payload": {"type": "token_count", "info": {
                "total_token_usage": {"total_tokens": 30},
                "last_token_usage": {"total_tokens": 30, "input_tokens": 20, "cached_input_tokens": 5,
                                     "output_tokens": 10, "reasoning_output_tokens": 4}}}},
            {"timestamp": "2026-01-01T00:00:02Z", "type": "token_usage_record", "payload": {
                "thread_id": "t1", "turn_id": "u1", "response_id": "r1",
                "usage": {"total_tokens": 30, "input_tokens": 20, "cached_input_tokens": 5,
                          "output_tokens": 10, "reasoning_output_tokens": 4}}},
            # A compaction's own response: a record no token_count reports.
            {"timestamp": "2026-01-01T00:00:03Z", "type": "token_usage_record", "payload": {
                "thread_id": "t1", "turn_id": "u1", "response_id": "r2", "usage": {"total_tokens": 7}}},
            {"timestamp": "2026-01-01T00:00:03Z", "type": "compacted", "payload": {}}]
    data = "".join(json.dumps(r) + "\n" for r in rows).encode()
    with tempfile.TemporaryDirectory() as tmp:
        day = Path(tmp) / "sessions/2026/01/01"
        day.mkdir(parents=True)
        (day / "rollout-a.jsonl").write_bytes(data)
        (day / "rollout-b.jsonl").write_bytes(data)
        try:
            from compression import zstd
            packed = zstd.compress(data)
        except ImportError:
            cli = next((found for found in map(shutil.which, ZSTD_COMMANDS) if found), None)
            if cli is None:
                print("real_truth selftest: no zstd here; compressed rollouts not exercised")
                return
            packed = subprocess.run([cli, "-c"], input=data, capture_output=True, check=True).stdout
        (day / "rollout-b.jsonl.zst").write_bytes(packed)  # mid-compression: read once, plain
        (day / "rollout-c.jsonl.zst").write_bytes(packed)
        names = [f.name for f in rollout_files(Path(tmp) / "sessions")]
        assert names == ["rollout-a.jsonl", "rollout-b.jsonl", "rollout-c.jsonl.zst"], names
        assert [f.name for f in rollout_files(Path(tmp) / "sessions", plain_only=True)] == names[:2]
        plain, comp = read_codex_rollout(day / "rollout-a.jsonl"), read_codex_rollout(day / "rollout-c.jsonl.zst")
        assert comp["compressed"] and not plain["compressed"]
        for key in ("session", "cwd", "first", "model", "turns", "turn_detail", "records"):
            assert plain[key] == comp[key], key
        assert comp["turn_detail"] == [("m1", {"input": 20, "cached_input": 5, "output": 10, "reasoning": 4})]
        assert [(key[2], n) for _, key, n, _ in comp["records"]] == [("r1", 30), ("r2", 7)]
        (day / "rollout-a.jsonl").rename(day / "rollout-a.jsonl.zst")  # compressed after listing
        (day / "rollout-a.jsonl.zst").write_bytes(packed)
        assert read_codex_rollout(day / "rollout-a.jsonl")["compressed"]
        assert plain["parent"] is None
        review = [{"timestamp": "2026-01-01T00:00:00Z", "type": "session_meta",
                   "payload": {"id": "t2", "cwd": "/r", "source": {"subagent": "other"}}}]
        (day / "rollout-d.jsonl").write_text("".join(json.dumps(r) + "\n" for r in review))
        assert read_codex_rollout(day / "rollout-d.jsonl")["parent"] == "subagent"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--json", action="store_true", help="machine-readable redacted output")
    ap.add_argument("--key", help="write the redaction key (real names) to this private file")
    ap.add_argument("--live-minutes", type=int, default=15, help="a transcript this fresh counts as the live one")
    ap.add_argument("--claude-dir", action="append", default=[])
    ap.add_argument("--codex-dir", action="append", default=[])
    ap.add_argument("--compare", help="the JSON the qa_real_readonly test wrote: print OK / MISMATCH lines")
    ap.add_argument("--codex-history", action="store_true", help="Codex rate_limits over time: cadence and conflicts")
    ap.add_argument("--plain-rollouts-only", action="store_true",
                    help="leave Codex's compressed rollouts (.jsonl.zst) out: what a reader that skips them shows")
    args = ap.parse_args()
    if args.selftest:
        selftest()
        return 0
    if args.codex_history:
        print(codex_history(codex_dirs(args.codex_dir), time.time()))
        return 0
    truth = build(args)
    if args.compare:
        print(compare(truth, json.loads(Path(args.compare).read_text())))
        return 0
    print(report(truth, args.key, args.json, args.live_minutes))
    return 0


if __name__ == "__main__":
    sys.exit(main())
