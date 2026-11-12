#!/usr/bin/env python3
"""A lab's desktop Apps, started and quit as AI Juggler's own Open starts
them (PL1, 0.4.3's rail: "an App open with no session").

`start` starts each Claude app profile the lab's scenario says runs
(`running = true`) with the launch settings the seeder stored for it, as
Open does (`places::start`): `open -n -g` (a further instance, behind the
app in front) with its Claude Code folder in `CLAUDE_CONFIG_DIR` and its
own `--user-data-dir`, both under the lab. A lab profile isn't signed in,
so each shows Claude's sign-in window: never type or click in it.

`stop` quits every Claude app instance whose `--user-data-dir` is under the
lab, and its helpers and crash reporter (a folder of theirs under the
lab), found in the process table by those arguments (never by reading an
environment): asked first (SIGTERM), made to (SIGKILL) only if still
there. Nothing else is started or stopped: the Codex app is this Mac's
own ChatGPT.app, only reported (a build on the lab sees only the lab's
processes, so in a lab it reads closed).

  lab_apps.py start --home /tmp/aij-lab-pl1
  lab_apps.py stop  --home /tmp/aij-lab-pl1
  lab_apps.py list  --home /tmp/aij-lab-pl1
  lab_apps.py --selftest
"""
from __future__ import annotations

import argparse
import json
import os
import re
import signal
import sqlite3
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

CLAUDE_BUNDLE = "Claude.app"
CHATGPT_MAIN = "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT"
DATA = "Library/Application Support/AI Juggler Dev"
FLAG = re.compile(r"--user-data-dir[= ](.+?)(?= --|$)")
# A Claude app process's folders: its instance's (the main process and its
# helpers) and its crash reporter's database (`chrome_crashpad_handler`,
# which outlives its parent: launchd adopts it).
FOLDERS = re.compile(r"--(?:user-data-dir|database)[= ](.+?)(?= --|$)")


def lab_home(text: str) -> Path:
    """A lab's HOME: under /tmp and named aij-lab… (as lab_scenario.sh
    insists); anything else is refused."""
    if not re.match(r"^(/private)?/tmp/.*aij-lab", text) or ".." in Path(text).parts:
        sys.exit(f"lab-apps: {text} isn't a lab (under /tmp, named aij-lab…) — refusing")
    return Path(text.rstrip("/"))


def under(path: str, home: Path) -> bool:
    """`path` is inside the lab `home`, however /tmp is spelt."""
    bare = str(home).removeprefix("/private")
    return any(path.startswith(f"{root}/") for root in (bare, f"/private{bare}"))


@dataclass
class Plan:
    """How Open starts one of the lab's Claude app profiles."""

    name: str
    bundle: Path
    exe: Path
    given: str
    folder: str


def plans(home: Path) -> list[Plan]:
    """The lab's Claude app profiles its scenario says run, with the launch
    settings the seeder stored (the lab's own AI Juggler data, read-only).
    One whose settings point outside the lab, or at another app, is
    refused."""
    seed = json.loads((home / "scenario-seed.json").read_text())
    wanted = [
        app.get("profile") or app.get("id")
        for app in seed.get("apps", [])
        if app.get("tool") == "claude-desktop" and app.get("running") is True and app.get("added") is not False
    ]
    rows = read_only(
        home / DATA / "juggler.sqlite",
        "select a.launch_json, f.state_dir from apps a left join accounts f on f.id = a.folder_id "
        "where a.kind = 'desktop' and a.host_app_id = 'claude-app'",
    )
    out = []
    for name in wanted:
        found = [(json.loads(launch or "{}"), folder) for launch, folder in rows]
        match = next(((ran, folder) for ran, folder in found if ran.get("instance") == name), None)
        if match is None:
            sys.exit(f"lab-apps: {name} has no launch settings in the lab's data (set the lab up again)")
        ran, folder = match
        bundle = Path(ran.get("bundle", ""))
        exe = Path(ran.get("exe", ""))
        given = str(ran.get("given") or "")
        if bundle.name != CLAUDE_BUNDLE or not bundle.is_absolute() or exe.parent != bundle / "Contents/MacOS":
            sys.exit(f"lab-apps: {name}'s launch settings aren't the Claude app's — refusing")
        if not under(given, home) or not folder or not under(folder, home):
            sys.exit(f"lab-apps: {name}'s folders aren't the lab's — refusing")
        out.append(Plan(name=name, bundle=bundle, exe=exe, given=given, folder=folder))
    return out


def read_only(path: Path, query: str) -> list[tuple]:
    """`query`'s rows, the database opened read-only: through its shared
    memory while the build runs on it, else (a WAL database closed cleanly
    has none, and a read-only open can't make it) as an unchanging file,
    which it is while nothing runs on it."""
    for mode in ("mode=ro", "immutable=1"):
        try:
            db = sqlite3.connect(f"{path.as_uri()}?{mode}", uri=True)
            try:
                return list(db.execute(query))
            finally:
                db.close()
        except sqlite3.OperationalError:
            continue
    sys.exit(f"lab-apps: {path} can't be read")


def processes() -> list[tuple[int, str]]:
    """Every process's pid and command line."""
    listed = subprocess.run(["ps", "-axww", "-o", "pid=,args="], capture_output=True, text=True, check=True).stdout
    out = []
    for line in listed.splitlines():
        pid, _, args = line.strip().partition(" ")
        if pid.isdigit():
            out.append((int(pid), args.strip()))
    return out


def user_data_dir(args: str) -> str | None:
    found = FLAG.search(args)
    return found.group(1) if found else None


def on_lab(home: Path, table: list[tuple[int, str]]) -> list[tuple[int, str]]:
    """The Claude app's processes on the lab: an instance, its helpers and
    its crash reporter, each by a folder of its under the lab."""
    return [
        (pid, args)
        for pid, args in table
        if args.startswith(f"/Applications/{CLAUDE_BUNDLE}/") and any(under(folder, home) for folder in FOLDERS.findall(args))
    ]


def mains(home: Path, table: list[tuple[int, str]]) -> dict[str, int]:
    """Each lab instance's main process, by its `--user-data-dir`."""
    out = {}
    for pid, args in on_lab(home, table):
        if args.startswith(f"/Applications/{CLAUDE_BUNDLE}/Contents/MacOS/Claude "):
            out[user_data_dir(args) or ""] = pid
    return out


def open_command(plan: Plan) -> list[str]:
    """`places::start`'s command for a profile: a further instance, behind
    the app in front, on its Claude Code folder and its own instance
    folder."""
    return ["/usr/bin/open", "-n", "-g", "--env", f"CLAUDE_CONFIG_DIR={plan.folder}", "-a", str(plan.bundle),
            "--args", f"--user-data-dir={plan.given}"]


def chatgpt_runs(table: list[tuple[int, str]]) -> bool:
    return any(args == CHATGPT_MAIN or args.startswith(CHATGPT_MAIN + " ") for _, args in table)


def start(home: Path) -> int:
    todo = plans(home)
    table = processes()
    running = mains(home, table)
    for plan in todo:
        if plan.given in running:
            print(f"lab-apps: Claude app · {plan.name} runs already (pid {running[plan.given]})")
            continue
        subprocess.run(open_command(plan), check=True, stdin=subprocess.DEVNULL)
        deadline = time.monotonic() + 30
        while plan.given not in (now := mains(home, processes())) and time.monotonic() < deadline:
            time.sleep(0.5)
        if plan.given not in now:
            print(f"lab-apps: Claude app · {plan.name} didn't start in 30 s", file=sys.stderr)
            return 1
        print(f"lab-apps: Claude app · {plan.name} started on the lab (pid {now[plan.given]}); its sign-in window shows: leave it")
    seed = json.loads((home / "scenario-seed.json").read_text())
    if any(app.get("tool") == "codex-app" for app in seed.get("apps", [])):
        print("lab-apps: the Codex app is this Mac's own ChatGPT.app (" + ("running" if chatgpt_runs(table) else "not running")
              + "), never started by the lab; a build on the lab sees only the lab's processes (AJ-E3306), so there it reads closed")
    return 0


def stop(home: Path) -> int:
    table = processes()
    first = mains(home, table)
    for folder, pid in first.items():
        print(f"lab-apps: quitting the Claude app on {Path(folder).name} (pid {pid})")
        with_signal(pid, signal.SIGTERM)
    deadline = time.monotonic() + 20
    while mains(home, processes()) and time.monotonic() < deadline:
        time.sleep(0.5)
    for sig, wait in ((signal.SIGTERM, 3), (signal.SIGKILL, 2)):
        left = on_lab(home, processes())
        if not left:
            break
        for pid, _ in left:
            with_signal(pid, sig)
        time.sleep(wait)
    left = on_lab(home, processes())
    if left:
        print(f"lab-apps: still running on the lab: {', '.join(str(pid) for pid, _ in left)}", file=sys.stderr)
        return 1
    print("lab-apps: no Claude app runs on the lab")
    return 0


def with_signal(pid: int, sig: int) -> None:
    try:
        os.kill(pid, sig)
    except ProcessLookupError:
        pass


def listing(home: Path) -> int:
    table = processes()
    for folder, pid in sorted(mains(home, table).items()):
        print(f"Claude app · {Path(folder).name}: pid {pid}")
    print(f"Codex app (this Mac's ChatGPT.app): {'running' if chatgpt_runs(table) else 'not running'}")
    return 0


def selftest() -> int:
    import tempfile

    home = lab_home("/tmp/aij-lab-z1")
    for bad in ("/Users/you", "/tmp/other", "/tmp/aij-lab-z1/../x"):
        try:
            lab_home(bad)
            raise AssertionError(f"accepted {bad}")
        except SystemExit:
            pass
    spaced = "/tmp/aij-lab-z1/Library/Application Support/ClaudeProfiles/work"
    table = [
        (10, f"/Applications/Claude.app/Contents/MacOS/Claude --user-data-dir={spaced}"),
        (11, f"/Applications/Claude.app/Contents/Frameworks/Claude Helper.app/Contents/MacOS/Claude Helper --type=gpu --user-data-dir={spaced} --x"),
        (12, "/Applications/Claude.app/Contents/MacOS/Claude --user-data-dir=/Users/you/Library/Application Support/ClaudeProfiles/work"),
        (13, f"/Applications/Claude.app/Contents/MacOS/Claude --user-data-dir=/private{spaced}"),
        (14, "/Applications/Claude.app/Contents/MacOS/Claude"),
        (15, CHATGPT_MAIN),
        (16, f"/Applications/Claude.app/Contents/Frameworks/Electron Framework.framework/Helpers/chrome_crashpad_handler --no-rate-limit --database=/private{spaced}/Crashpad --url=https://x"),
        (17, f"/usr/bin/some-tool --user-data-dir={spaced}"),
    ]
    assert user_data_dir(table[1][1]) == spaced
    assert [pid for pid, _ in on_lab(home, table)] == [10, 11, 13, 16], "the lab's, however /tmp is spelt; never a real profile, never another app"
    assert mains(home, table) == {spaced: 10, f"/private{spaced}": 13}
    assert chatgpt_runs(table) and not chatgpt_runs(table[:5])
    with tempfile.TemporaryDirectory(prefix="aij-lab-selftest-", dir="/tmp") as tmp:
        lab = lab_home(tmp)
        (lab / DATA).mkdir(parents=True)
        (lab / "scenario-seed.json").write_text(json.dumps({"apps": [
            {"id": "a", "tool": "claude-desktop", "profile": "work", "running": True},
            {"id": "b", "tool": "claude-desktop", "profile": "client", "running": False},
            {"id": "c", "tool": "codex-app", "running": True},
        ]}))
        db = sqlite3.connect(lab / DATA / "juggler.sqlite")
        # As AI Juggler keeps it: WAL, closed cleanly (no shared memory left).
        db.execute("pragma journal_mode=wal")
        db.execute("create table apps (id, kind, host_app_id, launch_json, folder_id)")
        db.execute("create table accounts (id, state_dir)")
        for name in ("work", "client"):
            given = f"{lab}/Library/Application Support/ClaudeProfiles/{name}"
            launch = {"name": "Claude app", "instance": name, "bundle": "/Applications/Claude.app",
                      "exe": "/Applications/Claude.app/Contents/MacOS/Claude", "given": given}
            db.execute("insert into apps values (?, 'desktop', 'claude-app', ?, ?)", (f"app:{name}", json.dumps(launch), f"f-{name}"))
            db.execute("insert into accounts values (?, ?)", (f"f-{name}", f"{lab}/Library/Application Support/ClaudeCodeProfiles/{name}"))
        db.commit()
        db.close()
        [plan] = plans(lab)
        assert plan.name == "work" and plan.folder.endswith("ClaudeCodeProfiles/work"), plan
        assert open_command(plan) == [
            "/usr/bin/open", "-n", "-g", "--env", f"CLAUDE_CONFIG_DIR={plan.folder}", "-a", "/Applications/Claude.app",
            "--args", f"--user-data-dir={plan.given}",
        ]
        # Settings that point outside the lab are refused.
        db = sqlite3.connect(lab / DATA / "juggler.sqlite")
        db.execute("update accounts set state_dir = '/Users/you/.claude' where id = 'f-work'")
        db.commit()
        db.close()
        try:
            plans(lab)
            raise AssertionError("a real folder was accepted")
        except SystemExit:
            pass
    print("lab_apps: selftest passed")
    return 0


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["start", "stop", "list"])
    parser.add_argument("--home", required=True, help="the lab's HOME (/tmp/aij-lab-…)")
    args = parser.parse_args()
    home = lab_home(args.home)
    return {"start": start, "stop": stop, "list": listing}[args.command](home)


if __name__ == "__main__":
    sys.exit(main())
