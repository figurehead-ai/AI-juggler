#!/usr/bin/env python3
"""AJ-E1531: a realistic replica of a heavy AI Juggler user, in a throwaway HOME.

Every hand walk and journey on release/0.1 runs against this, never the
maintainer's real data. It builds what a multi-account Mac looks like on
disk, with real-shaped files and no real content:

- `~/.claude` plus six Claude profiles (`~/.claude-<name>`), each with
  settings, `.claude.json` and months of `projects/<cwd>/<uuid>.jsonl`
  transcripts (some profiles already have a status line of their own);
- `~/.codex` and `~/.codex-work` with `sessions/Y/M/D/rollout-*.jsonl`
  rollouts, including `token_count` events with `rate_limits`;
- 13+ git repositories under `~/Dev`, among them three clones of one repo
  (`acme-web`, `acme-web2`, `acme-web3`) and agent worktrees under
  `<repo>/.claude/worktrees/<name>`, some of them since removed (their
  transcripts remain);
- project icons for AJ-E1532: Tauri, Electron and Xcode app icons, a
  favicon, a root logo and a `.github` logo (each a small drawn image, so
  a walk sees them), an oversized logo and a "logo" that isn't an image;
- shell aliases pointing `CLAUDE_CONFIG_DIR` / `CODEX_HOME` at profiles.

`live` then runs, while the walk goes on, live `claude` and `codex`
processes (the fake harness under those names) appending real-shaped
lines, a Claude desktop stand-in, a ChatGPT-like helper that isn't ours to
classify, and optional memory pressure.

`truth` prints the generator's own ground truth (`~/.replica/truth.json`):
accounts, projects, conversations per account and project, worktrees and
live sessions. It is independent of the app's readers, so an oracle such
as `scripts/qa/real_truth.py` can be checked against it too.

Nothing here reads or writes outside the HOME it is given, and it refuses
a HOME that looks real (one that already holds anything it didn't make).

Usage:
  replica.py build --home DIR [--seed N] [--days N] [--now ISO]
  replica.py live  --home DIR --harness PATH --desktop PATH
                   [--claude N] [--codex N] [--pressure-mb N] [--minutes N]
  replica.py truth --home DIR
  replica.py selftest
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import random
import shutil
import struct
import zlib
import hashlib
import signal
import subprocess
import sys
import time
import uuid
from pathlib import Path

MARKER = ".replica"
DOMAIN = "example.test"  # RFC 6761: never a real address

# Profiles: (folder suffix or "" for ~/.claude, plan, has its own status line)
CLAUDE_HOMES = [
    ("", "max", False),
    ("work", "max", True),
    ("personal", "pro", False),
    ("client-acme", "max", False),
    ("client-globex", "team", True),
    ("max-2", "max", False),
    ("oss", "pro", False),
]
CODEX_HOMES = [("", "plus"), ("work", "pro")]

# Projects: (name, kind, icon) — kind "repo" | "clone:<of>" ; icon kinds below.
PROJECTS = [
    ("acme-web", "repo", "tauri"),
    ("acme-web2", "clone:acme-web", "tauri"),
    ("acme-web3", "clone:acme-web", "tauri"),
    ("payments-api", "repo", None),
    ("marketing-site", "repo", "favicon"),
    ("mobile-app", "repo", "xcode"),
    ("infra", "repo", "electron"),
    ("docs-portal", "repo", "github-logo"),
    ("data-pipeline", "repo", None),
    ("design-system", "repo", "oversize"),
    ("cli-tools", "repo", "not-an-image"),
    ("ml-experiments", "repo", None),
    ("blog", "repo", "favicon"),
    ("AIJuggler", "repo", "tauri"),
    ("scratch", "repo", "logo"),
    ("codex-playground", "repo", None),
]
# Used only from Codex: a project the Claude history never mentions.
CODEX_ONLY = {"codex-playground"}
# Agent worktrees: (repo, name, state) — "present"; "removed" (the folder
# is gone, its transcripts remain); "pruned" (the folder is still there but
# its `.git` file isn't: git forgot it).
WORKTREES = [
    ("AIJuggler", "e15-manage-9b6a83", "removed"),
    ("AIJuggler", "pensive-curie-02e8fc", "removed"),
    ("AIJuggler", "e15-feedback-83da50", "present"),
    ("acme-web", "fix-epub-export-4c1d2e", "present"),
    ("acme-web2", "tidy-imports-7a0b11", "pruned"),
]

TOPICS = [
    "Fix the flaky login test", "Add pagination to the orders endpoint",
    "Why does the build fail on CI?", "Refactor the settings screen",
    "Write tests for the parser", "Bump dependencies and fix breakages",
    "Explain this stack trace", "Add dark mode to the dashboard",
    "Speed up the import job", "Review my migration", "Port the script to Rust",
    "Draft release notes", "Find the memory leak in the worker",
    "Add a retry to the uploader", "Clean up dead code in utils",
]
MODELS_CLAUDE = ["claude-opus-4-6", "claude-sonnet-4-5", "claude-opus-4-6"]
MODELS_CODEX = ["gpt-5-codex", "gpt-5"]


def iso(t: dt.datetime) -> str:
    return t.astimezone(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.") + f"{t.microsecond // 1000:03d}Z"


def claude_dir_name(cwd: str) -> str:
    """Claude Code's projects/ folder name for a cwd: every non-alphanumeric
    character becomes '-'."""
    return "".join(c if c.isalnum() else "-" for c in cwd)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def touch(path: Path, when: dt.datetime) -> None:
    stamp = when.timestamp()
    os.utime(path, (stamp, stamp))


def refuse_real(home: Path) -> None:
    if home.exists() and any(home.iterdir()) and not (home / MARKER).exists():
        sys.exit(f"replica: {home} isn't empty and isn't a replica — refusing to touch it")
    real = Path(os.path.expanduser("~")).resolve()
    if home.resolve() == real:
        sys.exit("replica: that's your real HOME — refusing")


# ---------------------------------------------------------------- build


def git(repo: Path, *args: str) -> None:
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null",
           "HOME": str(repo), "GIT_AUTHOR_DATE": "2026-06-01T09:00:00Z",
           "GIT_COMMITTER_DATE": "2026-06-01T09:00:00Z"}
    subprocess.run(["git", "-C", str(repo), "-c", "user.name=Replica", "-c", f"user.email=replica@{DOMAIN}",
                    "-c", "init.defaultBranch=main", *args], check=True, env=env,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


PNG_1PX = bytes.fromhex(
    "89504e470d0a1a0a0000000d4948445200000001000000010806000000"
    "1f15c4890000000d49444154789c6360f8cfc0f01f0005fe02fea7d6a8"
    "c70000000049454e44ae426082")
def colour_of(name: str) -> tuple[int, int, int]:
    digest = hashlib.sha256(name.encode()).digest()
    return 60 + digest[0] % 160, 60 + digest[1] % 160, 60 + digest[2] % 160


def png_icon(name: str, size: int = 64) -> bytes:
    """A small drawn icon (a disc on a light tile), the same for a name."""
    r, g, b = colour_of(name)
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            inside = (x - size / 2 + 0.5) ** 2 + (y - size / 2 + 0.5) ** 2 <= (size * 0.36) ** 2
            row += bytes([r, g, b, 255]) if inside else bytes([238, 240, 245, 255])
        rows.append(bytes(row))
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b"")


def svg_icon(name: str) -> str:
    r, g, b = colour_of(name)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="24" height="24" rx="5" fill="#eef0f5"/>'
            f'<path d="M5 18 L12 5 L19 18 Z" fill="rgb({r},{g},{b})"/></svg>\n')


def write_icon(repo: Path, kind: str | None) -> str | None:
    if kind == "tauri":
        path = repo / "src-tauri" / "icons" / "icon.png"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(png_icon(repo.name))
    elif kind == "electron":
        path = repo / "build" / "icon.png"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(png_icon(repo.name))
    elif kind == "xcode":
        icons = repo / "MobileApp" / "Assets.xcassets" / "AppIcon.appiconset"
        icons.mkdir(parents=True, exist_ok=True)
        (icons / "icon-40.png").write_bytes(png_icon(repo.name + "-small", 40))
        path = icons / "icon-1024.png"
        path.write_bytes(png_icon(repo.name, 96))
    elif kind == "favicon":
        path = repo / "public" / "favicon.svg"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(svg_icon(repo.name))
    elif kind == "logo":
        path = repo / "logo.png"
        path.write_bytes(png_icon(repo.name))
    elif kind == "github-logo":
        path = repo / ".github" / "docs-logo.svg"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(svg_icon(repo.name))
    elif kind == "oversize":  # over 512 KB: must be refused
        path = repo / "logo.png"
        path.write_bytes(PNG_1PX + b"\0" * (600 * 1024))
    elif kind == "not-an-image":  # named like one, isn't: must be refused
        path = repo / "logo.png"
        path.write_text("this is not an image\n")
    else:
        return None
    return str(path.relative_to(repo))


def build_repos(home: Path, truth: dict) -> dict[str, Path]:
    dev = home / "Dev"
    repos: dict[str, Path] = {}
    for name, kind, icon in PROJECTS:
        repo = dev / name
        repo.mkdir(parents=True, exist_ok=True)
        (repo / "README.md").write_text(f"# {name}\n\nReplica project. No real content.\n")
        git(repo, "init", "-q")
        origin = kind.split(":", 1)[1] if kind.startswith("clone:") else name
        git(repo, "remote", "add", "origin", f"git@{DOMAIN}:replica/{origin}.git")
        git(repo, "add", "-A")
        git(repo, "commit", "-q", "-m", "Initial commit")
        repos[name] = repo
        truth["projects"][name] = {
            "path": str(repo), "origin": origin, "clone_of": origin if origin != name else None,
            "icon": write_icon(repo, icon), "icon_kind": icon, "conversations": 0,
        }
    for repo_name, wt, state in WORKTREES:
        repo = repos[repo_name]
        path = repo / ".claude" / "worktrees" / wt
        git(repo, "worktree", "add", "-q", "-b", f"claude/{wt}", str(path))
        if state == "removed":
            git(repo, "worktree", "remove", "--force", str(path))
        elif state == "pruned":
            (path / ".git").unlink()
            git(repo, "worktree", "prune")
        truth["worktrees"].append({"repo": repo_name, "name": wt, "path": str(path), "state": state})
    # Our own agent worktrees must never be projects; neither must a stray
    # folder with no git at all.
    (dev / "notes").mkdir(exist_ok=True)
    return repos


def claude_settings(own_status_line: bool) -> dict:
    settings: dict = {"model": "opus", "permissions": {"allow": ["Bash(npm test:*)"]}}
    if own_status_line:
        settings["statusLine"] = {"type": "command", "command": "~/.config/statusline.sh"}
    return settings


def claude_line(kind: str, cwd: str, session: str, branch: str, parent: str | None, at: dt.datetime,
                rng: random.Random, text: str = "", model: str = "") -> tuple[dict, str]:
    uid = str(uuid.UUID(int=rng.getrandbits(128)))
    base = {"parentUuid": parent, "isSidechain": False, "userType": "external", "cwd": cwd,
            "sessionId": session, "version": "2.1.296", "gitBranch": branch, "uuid": uid, "timestamp": iso(at)}
    if kind == "user":
        base.update({"type": "user", "message": {"role": "user", "content": text}})
    else:
        inp = rng.randint(800, 60000)
        base.update({"type": "assistant", "message": {
            "id": f"msg_{uid[:12]}", "type": "message", "role": "assistant", "model": model,
            "content": [{"type": "text", "text": "Done. Replica reply."}], "stop_reason": "end_turn",
            "usage": {"input_tokens": inp // 10, "cache_creation_input_tokens": rng.randint(0, 4000),
                      "cache_read_input_tokens": inp, "output_tokens": rng.randint(40, 3000),
                      "service_tier": "standard"}}})
    return base, uid


def write_claude_conversation(folder: Path, cwd: str, branch: str, start: dt.datetime, rng: random.Random,
                              session: str | None = None) -> tuple[str, dt.datetime, int, str]:
    session = session or str(uuid.UUID(int=rng.getrandbits(128)))
    topic = rng.choice(TOPICS)
    model = rng.choice(MODELS_CLAUDE)
    path = folder / claude_dir_name(cwd) / f"{session}.jsonl"
    path.parent.mkdir(parents=True, exist_ok=True)
    at, parent, tokens = start, None, 0
    lines = [json.dumps({"type": "summary", "summary": topic, "leafUuid": "leaf"})]
    for turn in range(rng.randint(2, 18)):
        line, parent = claude_line("user", cwd, session, branch, parent, at, rng,
                                   text=topic if turn == 0 else "Keep going.")
        lines.append(json.dumps(line))
        at += dt.timedelta(seconds=rng.randint(5, 240))
        line, parent = claude_line("assistant", cwd, session, branch, parent, at, rng, model=model)
        usage = line["message"]["usage"]
        tokens += usage["input_tokens"] + usage["output_tokens"]
        lines.append(json.dumps(line))
        at += dt.timedelta(seconds=rng.randint(20, 900))
    path.write_text("\n".join(lines) + "\n")
    touch(path, at)
    return session, at, tokens, topic


def codex_rollout(folder: Path, cwd: str, start: dt.datetime, rng: random.Random,
                  used: tuple[float, float]) -> tuple[str, dt.datetime, int, str]:
    thread = str(uuid.UUID(int=rng.getrandbits(128)))
    topic = rng.choice(TOPICS)
    day = folder / "sessions" / f"{start:%Y}" / f"{start:%m}" / f"{start:%d}"
    day.mkdir(parents=True, exist_ok=True)
    path = day / f"rollout-{start:%Y-%m-%dT%H-%M-%S}-{thread}.jsonl"
    at, tokens = start, 0
    lines = [{"timestamp": iso(at), "type": "session_meta", "payload": {
        "id": thread, "cwd": cwd, "cli_version": "0.160.0", "originator": "codex_cli_rs",
        "history_mode": "paginated"}}]
    for turn in range(rng.randint(2, 12)):
        at += dt.timedelta(seconds=rng.randint(10, 600))
        lines.append({"timestamp": iso(at), "type": "turn_context",
                      "payload": {"cwd": cwd, "model": rng.choice(MODELS_CODEX)}})
        lines.append({"timestamp": iso(at), "type": "response_item", "payload": {
            "type": "message", "role": "user", "content": [{"type": "input_text", "text": topic}]}})
        inp, out = rng.randint(2000, 80000), rng.randint(50, 4000)
        tokens += inp + out
        usage = {"input_tokens": inp, "cached_input_tokens": inp // 3, "output_tokens": out,
                 "reasoning_output_tokens": out // 4, "total_tokens": inp + out}
        # As Codex writes it: each response's usage once, keyed by its ids.
        lines.append({"timestamp": iso(at), "type": "token_usage_record", "payload": {
            "thread_id": thread, "turn_id": str(turn), "response_id": f"resp_{thread[:8]}_{turn}", "usage": usage}})
        resets = int((at + dt.timedelta(hours=rng.randint(1, 5))).timestamp())
        lines.append({"timestamp": iso(at), "type": "event_msg", "payload": {
            "type": "token_count", "info": {"total_token_usage": usage, "last_token_usage": usage},
            "rate_limits": {
                "primary": {"used_percent": used[0], "window_minutes": 300, "resets_at": resets},
                "secondary": {"used_percent": used[1], "window_minutes": 10080, "resets_at": resets + 400000},
            }}})
    path.write_text("\n".join(json.dumps(line) for line in lines) + "\n")
    touch(path, at)
    return thread, at, tokens, topic


def pad_newest_rollout(home: Path, truth: dict, mb: int) -> None:
    """QA batch 10: one Mac's rollouts were 6.6 GB, the newest the biggest,
    and the first index pass read each whole before anything showed. This
    makes the newest Codex rollout about `mb` MB with reasoning items
    (no tokens, no new ask), so the truth is unchanged and a walk sees the
    first pass on a large file."""
    newest = max((c for c in truth["conversations"] if c["tool"] == "codex"), key=lambda c: c["last"])
    path = next(home.glob(f".codex*/sessions/*/*/*/rollout-*{newest['id']}.jsonl"))
    stamp = path.stat().st_mtime
    lines = path.read_text().splitlines()
    first_at = json.loads(lines[0])["timestamp"]
    chunk = json.dumps({"timestamp": first_at, "type": "response_item", "payload": {
        "type": "reasoning", "summary": [], "content": [{"type": "reasoning_text", "text": "x" * 65_000}]}})
    with path.open("w") as out:
        out.write(lines[0] + "\n")
        for _ in range(mb * 1024 * 1024 // len(chunk)):
            out.write(chunk + "\n")
        out.write("\n".join(lines[1:]) + "\n")
    os.utime(path, (stamp, stamp))


def build(args: argparse.Namespace) -> None:
    home = Path(args.home).expanduser()
    refuse_real(home)
    if home.exists():
        shutil.rmtree(home)
    home.mkdir(parents=True)
    (home / MARKER).mkdir()
    rng = random.Random(args.seed)
    now = dt.datetime.fromisoformat(args.now) if args.now else dt.datetime.now(dt.timezone.utc)
    truth: dict = {"seed": args.seed, "now": iso(now), "home": str(home), "accounts": [], "projects": {},
                   "worktrees": [], "conversations": [], "live": []}
    repos = build_repos(home, truth)
    worktree_cwds = [(w["repo"], w["path"], f"claude/{w['name']}") for w in truth["worktrees"]]

    def cwd_pick(codex: bool = False) -> tuple[str, str, str]:
        if rng.random() < 0.12:
            repo, path, branch = rng.choice(worktree_cwds)
            return repo, path, branch
        name = rng.choice([n for n in repos if codex or n not in CODEX_ONLY])
        return name, str(repos[name]), rng.choice(["main", "main", "feature/x", "fix/y"])

    for suffix, plan, status_line in CLAUDE_HOMES:
        folder = home / (".claude" if not suffix else f".claude-{suffix}")
        label = suffix or "default"
        write_json(folder / "settings.json", claude_settings(status_line))
        write_json(folder / ".claude.json", {"oauthAccount": {
            "emailAddress": f"{label}@{DOMAIN}", "organizationName": f"Replica {label}"},
            "numStartups": rng.randint(3, 400)})
        write_json(folder / ".credentials.json", {"claudeAiOauth": {
            "accessToken": "replica-not-a-real-token", "refreshToken": "replica-not-a-real-token",
            "expiresAt": int((now + dt.timedelta(days=30)).timestamp() * 1000), "subscriptionType": plan}})
        account = {"tool": "claude-code", "folder": str(folder), "label": label, "plan": plan,
                   "own_status_line": status_line, "conversations": 0}
        truth["accounts"].append(account)
        for _ in range(rng.randint(25, 70)):
            project, cwd, branch = cwd_pick()
            start = now - dt.timedelta(days=rng.uniform(0.02, args.days))
            session, last, tokens, topic = write_claude_conversation(folder / "projects", cwd, branch, start, rng)
            truth["conversations"].append({"tool": "claude-code", "account": label, "project": project,
                                           "cwd": cwd, "id": session, "title": topic, "last": iso(last),
                                           "tokens": tokens})
            account["conversations"] += 1
            truth["projects"][project]["conversations"] += 1

    for suffix, plan in CODEX_HOMES:
        folder = home / (".codex" if not suffix else f".codex-{suffix}")
        label = f"codex-{suffix or 'default'}"
        (folder).mkdir(parents=True, exist_ok=True)
        (folder / "config.toml").write_text('model = "gpt-5-codex"\n')
        write_json(folder / "auth.json", {"OPENAI_API_KEY": None, "tokens": {
            "id_token": "replica-not-a-real-token", "access_token": "replica-not-a-real-token",
            "account_id": f"replica-{label}"}})
        account = {"tool": "codex", "folder": str(folder), "label": label, "plan": plan, "conversations": 0}
        truth["accounts"].append(account)
        used = (round(rng.uniform(5, 85), 1), round(rng.uniform(5, 60), 1))
        account["limits"] = {"five_hour_used_percent": used[0], "weekly_used_percent": used[1]}
        for _ in range(rng.randint(15, 40)):
            project, cwd, _branch = cwd_pick(codex=True)
            start = now - dt.timedelta(days=rng.uniform(0.02, args.days))
            thread, last, tokens, topic = codex_rollout(folder, cwd, start, rng, used)
            truth["conversations"].append({"tool": "codex", "account": label, "project": project, "cwd": cwd,
                                           "id": thread, "title": topic, "last": iso(last), "tokens": tokens})
            account["conversations"] += 1
            truth["projects"][project]["conversations"] += 1

    if args.large_rollout_mb:
        pad_newest_rollout(home, truth, args.large_rollout_mb)

    (home / ".zshrc").write_text("".join(
        f"alias claude-{s}='CLAUDE_CONFIG_DIR=~/.claude-{s} claude'\n" for s, _, _ in CLAUDE_HOMES if s)
        + "alias codex-work='CODEX_HOME=~/.codex-work codex'\n")
    status = home / ".config" / "statusline.sh"
    status.parent.mkdir(parents=True, exist_ok=True)
    status.write_text("#!/bin/sh\necho replica-status\n")
    status.chmod(0o755)
    write_json(home / MARKER / "truth.json", truth)
    print(summary(truth))


def summary(truth: dict) -> str:
    claude = [a for a in truth["accounts"] if a["tool"] == "claude-code"]
    codex = [a for a in truth["accounts"] if a["tool"] == "codex"]
    gone = sum(1 for w in truth["worktrees"] if w["state"] != "present")
    return (f"replica: {truth['home']}\n"
            f"  accounts: {len(claude)} Claude, {len(codex)} Codex\n"
            f"  projects: {len(truth['projects'])} (+{len(truth['worktrees'])} agent worktrees, {gone} removed or pruned)\n"
            f"  conversations: {len(truth['conversations'])}\n"
            f"  live: {len(truth['live'])}")


# ---------------------------------------------------------------- live


INFO_PLIST = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>{exe}</string>
<key>CFBundleIdentifier</key><string>test.aij-replica.{ident}</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>
"""


def make_bundle(bundle: Path, exe: str, source: str, nested: list[Path] | None = None) -> Path:
    """A fake `.app` macOS accepts: an Info.plist, its executable, and an
    ad-hoc signature over the whole bundle. A bare binary copied into
    `Contents/MacOS` keeps the linker's signature, which says the bundle
    has resources; macOS then calls the app damaged and shows a dialog on
    every launch (the coordinator, 2026-10-22). `codesign --verify
    --strict` must pass before anything starts from it. (`spctl` rejects
    every ad-hoc bundle, AI Juggler's own Dev builds included; that is not
    what raises the dialog, so it isn't the check.)"""
    target = bundle / "Contents" / "MacOS" / exe
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    ident = "".join(c if c.isalnum() else "-" for c in bundle.stem.lower())
    (bundle / "Contents" / "Info.plist").write_text(INFO_PLIST.format(exe=exe, ident=ident))
    if sys.platform == "darwin":
        for path in [*(nested or []), bundle]:
            subprocess.run(["codesign", "--force", "--sign", "-", str(path)], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        check = subprocess.run(["codesign", "--verify", "--strict", str(bundle)], capture_output=True, text=True)
        if check.returncode != 0:
            sys.exit(f"replica: {bundle.name} isn't a whole signed bundle, so nothing starts from it: {check.stderr.strip()}")
    return target


def live(args: argparse.Namespace) -> None:
    home = Path(args.home).expanduser()
    if not (home / MARKER / "truth.json").exists():
        sys.exit("replica: build it first (replica.py build --home DIR)")
    truth = json.loads((home / MARKER / "truth.json").read_text())
    rng = random.Random(args.seed + 1)
    bin_dir = home / MARKER / "bin"
    bin_dir.mkdir(exist_ok=True)
    # The names the app's process reader recognises: a `claude` CLI, and the
    # Claude desktop app's main executable.
    claude_bin = bin_dir / "claude"
    codex_bin = bin_dir / "codex"
    for target in (claude_bin, codex_bin):
        shutil.copy2(args.harness, target)
    # The desktop app runs each of its Code sessions from a bundled CLI;
    # the desktop app and a ChatGPT-like helper are bundles too. Each is a
    # whole, signed bundle (see `make_bundle`).
    bundled_bin = make_bundle(home / MARKER / "claude-code" / "2.1.0" / "claude.app", "claude", args.harness)
    app = make_bundle(home / MARKER / "Applications" / "Claude.app", "Claude", args.desktop)
    chatgpt = home / MARKER / "Applications" / "ChatGPT.app"
    helper = chatgpt / "Contents" / "Frameworks" / "ChatGPT Helper"
    helper.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(args.desktop, helper)
    make_bundle(chatgpt, "ChatGPT", args.desktop, nested=[helper])

    procs: list[subprocess.Popen] = []
    # A clean environment: nothing from the shell running this (above
    # all no CLAUDE_* or CODEX_* pointing at a real setup).
    env_base = {"HOME": str(home), "PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "LANG": "en_US.UTF-8",
                "TMPDIR": os.environ.get("TMPDIR", "/tmp"), "USER": os.environ.get("USER", "replica")}
    claude_accounts = [a for a in truth["accounts"] if a["tool"] == "claude-code"]
    codex_accounts = [a for a in truth["accounts"] if a["tool"] == "codex"]
    projects = list(truth["projects"].values())
    truth["live"] = []
    def writer(options: list[str]) -> str:
        return "\x1f".join(options)

    for i in range(args.claude):
        account = claude_accounts[i % len(claude_accounts)]
        project = projects[(i * 3) % len(projects)]
        env = {**env_base}
        if account["label"] != "default":
            env["CLAUDE_CONFIG_DIR"] = account["folder"]
        # Every other one resumes a conversation it already had there
        # (`claude --resume <id>`); the rest start fresh (`claude`).
        earlier = [c for c in truth["conversations"]
                   if c["tool"] == "claude-code" and c["account"] == account["label"] and c["cwd"] == project["path"]]
        # Every third runs as the desktop app's Code sessions do: from its
        # bundled CLI, resuming with `--resume=<id>` (one word). A resumed
        # one of those sits idle, so only its id names its conversation
        # (nothing in its folder is newer than its start).
        app_run = i % 3 == 2
        exe = str(bundled_bin if app_run else claude_bin)
        idle = False
        if app_run and i % 2 == 1:
            # Any of the account's own conversations, in its own folder.
            mine = [c for c in truth["conversations"] if c["tool"] == "claude-code" and c["account"] == account["label"]]
            if mine:
                project = {"path": mine[0]["cwd"]}
                earlier = mine[:1]
        if i % 2 == 1 and earlier:
            session = earlier[0]["id"]
            argv = [exe, f"--resume={session}"] if app_run else [exe, "--resume", session]
            idle = app_run
        else:
            session = str(uuid.UUID(int=rng.getrandbits(128)))
            argv = [exe]
        folder = Path(account["folder"]) / "projects" / claude_dir_name(project["path"])
        options = ["--format", "claude", "--file", str(folder / f"{session}.jsonl"),
                   "--cwd", project["path"], "--session-id", session, "--every-ms", "4000"]
        if len(argv) == 1:
            options += ["--ask", rng.choice(TOPICS)]  # a fresh one starts with a real ask
        if idle:
            options += ["--idle"]
        if i == 0:
            # QA (the real setup): Claude Code runs its search helpers as the
            # same `claude` binary under another name (`ugrep`), and they can
            # outlive their session under a stray shell. One sits here, on
            # this account and in this project, started just before the
            # session: it must never count as one (it's in no truth list).
            helper_env = {**env, "AIJ_REPLICA_WRITE": writer([
                "--format", "claude", "--file", str(home / MARKER / "helper-scratch.jsonl"),
                "--cwd", project["path"], "--session-id", "helper", "--every-ms", "60000", "--idle"])}
            procs.append(subprocess.Popen(["ugrep", "-r", "TODO", "."], executable=exe, cwd=project["path"],
                                          env=helper_env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL))
            truth["helpers"] = [{"name": "ugrep", "pid": procs[-1].pid, "account": account["label"],
                                 "project": Path(project["path"]).name}]
            time.sleep(0.2)
        env["AIJ_REPLICA_WRITE"] = writer(options)
        procs.append(subprocess.Popen(argv, cwd=project["path"], env=env,
                                      stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL))
        truth["live"].append({"tool": "claude-code", "account": account["label"], "project": Path(project["path"]).name,
                              "id": session, "pid": procs[-1].pid})
    for i in range(args.codex):
        account = codex_accounts[i % len(codex_accounts)]
        project = projects[(i * 5 + 1) % len(projects)]
        thread = str(uuid.UUID(int=rng.getrandbits(128)))
        now = dt.datetime.now(dt.timezone.utc)
        day = Path(account["folder"]) / "sessions" / f"{now:%Y}" / f"{now:%m}" / f"{now:%d}"
        env = {**env_base, "CODEX_HOME": account["folder"]}
        env["AIJ_REPLICA_WRITE"] = writer([
            "--format", "codex", "--file", str(day / f"rollout-{now:%Y-%m-%dT%H-%M-%S}-{thread}.jsonl"),
            "--cwd", project["path"], "--session-id", thread, "--every-ms", "5000", "--ask", rng.choice(TOPICS),
            "--used", str(account["limits"]["five_hour_used_percent"]), str(account["limits"]["weekly_used_percent"])])
        procs.append(subprocess.Popen([str(codex_bin)], cwd=project["path"], env=env,
                                      stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL))
        truth["live"].append({"tool": "codex", "account": account["label"], "project": Path(project["path"]).name,
                              "id": thread, "pid": procs[-1].pid})
    if args.desktop_app:
        procs.append(subprocess.Popen([str(app), "--exit-after-secs", str(args.minutes * 60)],
                                      env={**env_base, "CLAUDE_CONFIG_DIR": claude_accounts[1]["folder"]}))
        procs.append(subprocess.Popen([str(helper), "--exit-after-secs", str(args.minutes * 60)], env=env_base))
    if args.pressure_mb:
        procs.append(subprocess.Popen([sys.executable, "-c", (
            "import time,sys\n"
            f"b=bytearray({args.pressure_mb}*1024*1024)\n"
            "for i in range(0,len(b),4096): b[i]=1\n"
            f"time.sleep({args.minutes * 60})\n")]))
    write_json(home / MARKER / "truth.json", truth)
    print(summary(truth))
    print(f"  running for {args.minutes} min; Ctrl-C stops everything")

    def stop(*_: object) -> None:
        for proc in procs:
            if proc.poll() is None:
                proc.terminate()
        for proc in procs:
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
        truth["live"] = []
        write_json(home / MARKER / "truth.json", truth)
        sys.exit(0)

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)
    deadline = time.monotonic() + args.minutes * 60
    while time.monotonic() < deadline:
        time.sleep(1)
    stop()


def selftest() -> None:
    """Determinism, the refusal and the shapes the app reads."""
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        ns = lambda home: argparse.Namespace(home=home, seed=7, days=30, now="2026-10-21T12:00:00+00:00", large_rollout_mb=0)
        truths = []
        for name in ("a", "b"):
            build(ns(f"{tmp}/{name}"))
            truth = json.loads((Path(tmp) / name / MARKER / "truth.json").read_text())
            truths.append(json.dumps({**truth, "home": ""}, sort_keys=True).replace(f"{tmp}/{name}", "~"))
        assert truths[0] == truths[1], "the same seed builds the same replica"
        home = Path(tmp) / "a"
        truth = json.loads((home / MARKER / "truth.json").read_text())
        assert len([a for a in truth["accounts"] if a["tool"] == "claude-code"]) == 7
        assert len(truth["projects"]) >= 13
        assert {w["state"] for w in truth["worktrees"]} == {"present", "removed", "pruned"}
        for conversation in truth["conversations"]:
            assert conversation["project"] in truth["projects"]
        # AJ-E1532: every kind of icon, drawn; the two that must be refused.
        icons = {p["icon_kind"]: Path(p["path"]) / p["icon"] for p in truth["projects"].values() if p["icon"]}
        assert set(icons) == {"tauri", "electron", "xcode", "favicon", "logo", "github-logo", "oversize", "not-an-image"}
        for kind in ("tauri", "electron", "xcode", "logo"):
            assert icons[kind].read_bytes().startswith(b"\x89PNG") and icons[kind].stat().st_size < 512 * 1024, kind
        assert icons["oversize"].stat().st_size > 512 * 1024
        assert not icons["not-an-image"].read_bytes().startswith(b"\x89PNG")
        # Every transcript line is JSON; Claude's folder is named from the cwd.
        claude = list((home / ".claude-work" / "projects").glob("*/*.jsonl"))
        assert claude and all(c.parent.name == claude_dir_name(json.loads(c.read_text().splitlines()[1])["cwd"])
                              for c in claude)
        for path in list(home.glob(".claude*/projects/*/*.jsonl")) + list(home.glob(".codex*/sessions/*/*/*/*.jsonl")):
            for line in path.read_text().splitlines():
                json.loads(line)
        rollout = next(home.glob(".codex/sessions/*/*/*/rollout-*.jsonl")).read_text()
        assert '"rate_limits"' in rollout and '"used_percent"' in rollout
        # The pruned worktree is a folder git no longer knows.
        pruned = next(w for w in truth["worktrees"] if w["state"] == "pruned")
        assert Path(pruned["path"]).is_dir() and not (Path(pruned["path"]) / ".git").exists()
        # A HOME that isn't a replica is refused, and never touched.
        real = Path(tmp) / "someone"
        real.mkdir()
        (real / "keep.txt").write_text("mine")
        try:
            build(ns(str(real)))
            raise AssertionError("a non-replica HOME was accepted")
        except SystemExit:
            pass
        assert (real / "keep.txt").read_text() == "mine"
    print("replica selftest: OK")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    b = sub.add_parser("build")
    b.add_argument("--home", required=True)
    b.add_argument("--seed", type=int, default=1531)
    b.add_argument("--days", type=float, default=120)
    b.add_argument("--now")
    b.add_argument("--large-rollout-mb", type=int, default=0,
                   help="make the newest Codex rollout about this many MB (a first pass on a large file)")
    l = sub.add_parser("live")
    l.add_argument("--home", required=True)
    l.add_argument("--harness", required=True, help="the fake-harness binary")
    l.add_argument("--desktop", required=True, help="the fake-desktop binary")
    l.add_argument("--seed", type=int, default=1531)
    l.add_argument("--claude", type=int, default=6)
    l.add_argument("--codex", type=int, default=2)
    l.add_argument("--no-desktop", dest="desktop_app", action="store_false")
    l.add_argument("--pressure-mb", type=int, default=0)
    l.add_argument("--minutes", type=int, default=60)
    t = sub.add_parser("truth")
    t.add_argument("--home", required=True)
    sub.add_parser("selftest")
    args = parser.parse_args()
    if args.command == "build":
        build(args)
    elif args.command == "live":
        live(args)
    elif args.command == "selftest":
        selftest()
    else:
        print(json.dumps(json.loads((Path(args.home).expanduser() / MARKER / "truth.json").read_text()), indent=2))


if __name__ == "__main__":
    main()
