#!/usr/bin/env python3
"""PL1's worktree shapes (0.5.1, AJ-E2328, AJ-E2315, AJ-E2317), seeded into
a lab by the Claude app's own on-disk formats
(`crates/juggler-adapters/src/claude_move/known_formats.json`, explained in
`docs/design/claude-known-formats.md`). Run sheet:
`docs/qa/0.5-walks/0.5.1-PL1.md` (A2, I1, I3, I6, I9, I10, I12).

  seed          A2: (a) a Code session of <profile-B> (`work`) in its own
                worktree `lab-wt-1` of the lab repo `repos/web`, on branch
                `lab/wt-1`, leased in work's `git-worktrees.json` (its
                record in the signed-in account's records folder when the
                maintainer signed the lab profile in, else the lab's);
                (b)+(c) a further profile `personal-2` (<profile-C>) on the
                Account Personal, as <profile-A> (`personal`) is: two
                profiles on one account; its Code session was given a
                folder inside <profile-B> (`--add-dir`).
  hold-add-dir  I3/J2: AJ-E2315's process shape: the Claude app's own
                `disclaimer` helper wrapping a lab stand-in `claude` that
                sits in <profile-C>, with `--add-dir=` into <profile-B>.
                Runs at most an hour; `release-add-dir` ends it.
  split         I12: the 0.4.x damage, with every lab profile quit: the
                worktree's lease put in `--claim-in` (default `work`), every
                other profile's lease of it dropped, and the record's branch
                made to differ from the worktree's HEAD.
  show          I1/I6/I9/I10: read-only, structure only: per lab profile
                its worktree file's sha256 and mtime, its cleanup's latest
                `roots` time, the lease (leasedBy, branch),
                whether its cleanup has sighted the worktree, and who holds
                the record; the worktree's HEAD.

Every command takes `--home <lab>`; `--dry-run` lists what it would write
and writes nothing.

Hard rules: it writes only inside the lab's own profile folders
(`<lab>/Library/Application Support/ClaudeProfiles/<p>/`, its Claude Code
folders `…/ClaudeCodeProfiles/<p>/`) and lab projects (`<lab>/repos/<r>/`,
`~/lab/<r>/`), and refuses every other path, the maintainer's real profile
folders first (a lab HOME is under /tmp and named aij-lab…, checked on its
real spelling, symlinks resolved). It writes a profile's worktree file and
records only while no Claude app process runs on that profile (found by
its `--user-data-dir` in the process table's arguments). It never opens a
credential (a placeholder sign-in file is written, never read) and never
reads another process's environment (D-37).

  python3 -I scripts/qa/lab_seed_worktrees.py --selftest
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pwd
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path
from typing import IO

ROOT = Path(__file__).resolve().parents[2]
FORMATS = ROOT / "crates/juggler-adapters/src/claude_move/known_formats.json"
REPLICA = ROOT / "scripts/replica.py"
DISCLAIMER = Path("/Applications/Claude.app/Contents/Helpers/disclaimer")
PROFILES = "Library/Application Support/ClaudeProfiles"
CODE = "Library/Application Support/ClaudeCodeProfiles"
WORKTREES_FILE = "git-worktrees.json"
SESSIONS = "claude-code-sessions"
REPO = "web"
WORKTREE = "lab-wt-1"
BRANCH = "lab/wt-1"
STALE = "lab/wt-0"
HOLDER = "work"
SECOND = "personal-2"
SECOND_ACCOUNT = "Personal"
ADD_DIR = "local-agent-mode-sessions"
STANDIN_VERSION = "2.1.296"
HOLD_SECS = 3600
LAB = re.compile(r"^(/private)?/tmp/[^/]*aij-lab[^/]*$")
UDD = re.compile(r"--user-data-dir[= ](.+?)(?= --|$)")


class Refused(SystemExit):
    pass


def refuse(why: str) -> None:
    raise Refused(f"lab-seed-worktrees: {why} — refusing")


def real_home() -> Path:
    """The account's own home (never $HOME, which a lab run sets)."""
    return Path(pwd.getpwuid(os.getuid()).pw_dir)


def lab_home(text: str) -> Path:
    """A lab HOME on its real spelling: under /tmp, named aij-lab…."""
    if ".." in Path(text).parts:
        refuse(f"{text} isn't a lab")
    real = Path(os.path.realpath(text))
    if not LAB.match(str(real)):
        refuse(f"{text} isn't a lab (under /tmp, named aij-lab…)")
    return real


def allowed_roots(home: Path) -> list[Path]:
    return [home / PROFILES, home / CODE, home / "repos", Path(os.path.realpath(real_home() / "lab"))]


def guard(path: Path, home: Path) -> Path:
    """`path` on its real spelling if it lies inside a named lab profile or
    lab project (one level below an allowed root, or deeper); else refused."""
    real = Path(os.path.realpath(path))
    for root in allowed_roots(home):
        root_real = Path(os.path.realpath(root))
        if real != root_real and real.is_relative_to(root_real):
            return real
    refuse(f"{path} isn't a lab profile folder or lab project")
    raise AssertionError  # unreachable


def profile_dir(home: Path, name: str) -> Path:
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", name):
        refuse(f"{name!r} isn't a profile name")
    return guard(home / PROFILES / name, home)


def processes() -> list[str]:
    """Every process's arguments (never its environment, D-37)."""
    out = subprocess.run(["ps", "-axww", "-o", "args="], capture_output=True, text=True, check=True).stdout
    return out.splitlines()


def running_on(profile: Path, table: list[str]) -> bool:
    want = os.path.realpath(profile)
    return any(os.path.realpath(found.rstrip("/")) == want for args in table for found in UDD.findall(args))


class Writer:
    """Every write goes through here: guarded, and listed on a dry run."""

    def __init__(self, home: Path, dry: bool, table: list[str] | None = None):
        self.home, self.dry, self.table = home, dry, table
        self.planned: list[str] = []

    def quit_check(self, profile: Path) -> None:
        table = self.table if self.table is not None else processes()
        if running_on(profile, table):
            refuse(f"the Claude app runs on {profile.name}: quit it first (Dev's quit), then run this again")

    def write(self, path: Path, data: bytes, why: str) -> None:
        real = guard(path, self.home)
        self.planned.append(f"write {real}  ({why})")
        if self.dry:
            return
        real.parent.mkdir(parents=True, exist_ok=True)
        tmp = real.with_name(f".{real.name}.lab-tmp")
        tmp.write_bytes(data)
        os.replace(tmp, real)

    def mkdir(self, path: Path, why: str) -> None:
        real = guard(path, self.home)
        self.planned.append(f"mkdir {real}  ({why})")
        if not self.dry:
            real.mkdir(parents=True, exist_ok=True)

    def run(self, argv: list[str], why: str, guarded: list[Path], env: dict[str, str] | None = None) -> str:
        for path in guarded:
            guard(path, self.home)
        self.planned.append(f"run {' '.join(argv[:3])} …  ({why})")
        if self.dry:
            return ""
        return subprocess.run(argv, capture_output=True, text=True, check=True, env=env).stdout


def dumps(value: object) -> bytes:
    return (json.dumps(value, indent=2) + "\n").encode()


def formats() -> dict:
    return json.loads(FORMATS.read_text())["entries"]


def fresh_worktrees_file() -> dict:
    schema = formats()["worktree_file"]["schemaVersion"]
    return {"schemaVersion": schema, "originUrls": {}, "originPins": {}, "worktrees": {},
            "untrackedDirGc": {"sightings": {}, "roots": {}, "cwds": {}}, "pendingTombstones": []}


def read_worktrees_file(profile: Path) -> dict | None:
    path = profile / WORKTREES_FILE
    if not path.is_file():
        return None
    value = json.loads(path.read_text())
    want = formats()["worktree_file"]["schemaVersion"]
    if value.get("schemaVersion") != want:
        refuse(f"{path.name} in {profile.name} is schema {value.get('schemaVersion')}, not {want}")
    return value


def git_env(home: Path) -> dict[str, str]:
    """git on the lab's HOME only: none of this account's config, hooks or signing."""
    return {"HOME": str(home), "PATH": "/usr/bin:/bin", "GIT_CONFIG_NOSYSTEM": "1", "LANG": "C"}


def head(worktree: Path, home: Path) -> str | None:
    if not worktree.exists():
        return None
    out = subprocess.run(["git", "-C", str(worktree), "rev-parse", "--abbrev-ref", "HEAD"],
                         capture_output=True, text=True, env=git_env(home))
    return out.stdout.strip() or None


def org_dir(profile: Path, account: str) -> Path:
    who = slug(account)
    return profile / SESSIONS / f"lab-account-{who}" / f"lab-org-{who}"


def holder_records(holder: Path, home: Path) -> Path:
    """Where <profile-B>'s Code tab keeps its records: the one signed-in
    account's folder when the maintainer has signed the lab profile in (a
    folder not named lab-account-…; names only, nothing in it opened),
    else the lab's Work folder."""
    signed = [p for p in sorted((holder / SESSIONS).glob("*/*"))
              if p.is_dir() and not p.parent.name.startswith("lab-account-")]
    if len(signed) > 1:
        refuse(f"{holder.name} has {len(signed)} signed-in account folders; leave one")
    return guard(signed[0], home) if signed else org_dir(holder, "Work")


def slug(name: str) -> str:
    """As the seeder spells a scenario Account ("Work Max": "work-max")."""
    return "".join(c.lower() if c.isascii() and c.isalnum() else "-" for c in name)


def conversation(w: Writer, folder: Path, records: Path, cwd: Path, topic: str, key: str) -> tuple[str, Path]:
    """A lab conversation and the app's record of it (replica.py's writer:
    Claude Code's transcript in `folder`, the record in `records`)."""
    session = str(uuid.uuid5(uuid.NAMESPACE_URL, f"aij-lab-worktrees:{key}"))
    local = f"local_{uuid.uuid5(uuid.NAMESPACE_URL, f'aij-lab-record:{session}')}"
    w.run([sys.executable, "-I", str(REPLICA), "conversation", "--folder", str(folder), "--cwd", str(cwd),
           "--session", session, "--entrypoint", "claude-desktop", "--app-record", str(records), "--topic", topic,
           "--ago-minutes", "90"], "a lab Code-tab session and its record", [folder, records])
    return session, records / f"{local}.json"


def patch_record(w: Writer, path: Path, change: dict, why: str) -> None:
    record = json.loads(path.read_text()) if path.exists() else {}
    record.update(change)
    w.write(path, dumps(record), why)


def seed(home: Path, dry: bool, table: list[str] | None = None) -> list[str]:
    w = Writer(home, dry, table)
    holder = profile_dir(home, HOLDER)
    second = profile_dir(home, SECOND)
    for profile in (holder, second):
        w.quit_check(profile)
    if not holder.is_dir():
        refuse(f"{holder} isn't there: set the lab up first (lab_scenario.sh setup PL1)")
    repo = guard(home / "repos" / REPO, home)
    if not (repo / ".git").exists():
        refuse(f"{repo} isn't a git repository: set the lab up first")
    base = head(repo, home) or "main"
    worktree = repo / ".claude" / "worktrees" / WORKTREE
    now = int(time.time() * 1000)

    # (a) the worktree, its session in <profile-B>, and work's lease of it.
    if worktree.exists():
        refuse(f"{worktree} exists already: this lab is seeded")
    w.run(["git", "-C", str(repo), "-c", "core.hooksPath=/dev/null", "worktree", "add", "-q", "-b", BRANCH, str(worktree)],
          f"the worktree {WORKTREE} on {BRANCH}", [repo, worktree], env=git_env(home))
    records = holder_records(holder, home)
    _, record = conversation(w, guard(home / CODE / HOLDER, home), records, worktree,
                             "Tidy the checkout page in its own worktree", "holder")
    patch_record(w, record, {"originCwd": str(repo), "worktreeName": WORKTREE, "worktreePath": str(worktree),
                             "branch": BRANCH}, "the record names its worktree and branch")
    local = record.stem
    file = read_worktrees_file(holder) or fresh_worktrees_file()
    file.setdefault("worktrees", {})[WORKTREE] = {
        "name": WORKTREE, "path": str(worktree), "leasedBy": local, "branch": BRANCH, "baseRepo": str(repo),
        "sourceBranch": base, "createdAt": now, "autoTrusted": False, "creationGrant": False,
    }
    gc = file.setdefault("untrackedDirGc", {})
    for key in ("sightings", "roots", "cwds"):
        gc.setdefault(key, {})
    gc["roots"][str(repo / ".claude" / "worktrees")] = now
    w.write(holder / WORKTREES_FILE, dumps(file), f"work leases {WORKTREE}")

    # (b)+(c) <profile-C>: on Personal (as <profile-A>), its Code session
    # given a folder inside <profile-B>.
    inside = holder / ADD_DIR
    w.mkdir(inside, "the folder inside <profile-B> that <profile-C>'s session was given")
    code = guard(home / CODE / SECOND, home)
    w.write(code / "settings.json", b"{}\n", "its Claude Code folder's settings")
    w.write(code / ".claude.json", dumps({"oauthAccount": {"accountUuid": f"lab-account-{slug(SECOND_ACCOUNT)}",
                                                           "emailAddress": f"{slug(SECOND_ACCOUNT)}@example.invalid"}}),
            "the lab's made-up Personal identity (as the seeder's lab sign-in)")
    w.write(code / ".credentials.json", b'{"lab": "not a sign-in"}', "a placeholder: presence only, signs nothing in")
    w.mkdir(second, "<profile-C>'s app folder")
    api = home / "repos" / "api"
    cwd = guard(api, home) if api.is_dir() else repo
    _, other = conversation(w, code, org_dir(second, SECOND_ACCOUNT), cwd, "Review the API client", "second")
    patch_record(w, other, {"sessionPermissionUpdates": [
        {"type": "addDirectories", "destination": "session", "directories": [str(inside)]}]},
        "its session was given a folder inside <profile-B> (--add-dir)")
    stand = second / "claude-code" / STANDIN_VERSION / "claude.app/Contents/MacOS/claude"
    w.write(stand, STANDIN.encode(), "the stand-in `claude` its disclaimer wraps (hold-add-dir)")
    if not dry:
        os.chmod(stand, 0o755)
    return w.planned


STANDIN = f"""#!/bin/bash
# A lab stand-in for <profile-C>'s Code-tab claude (lab_seed_worktrees.py
# hold-add-dir): runs nothing, reads nothing, ends within {HOLD_SECS} s.
case "$(/bin/pwd -P)" in /private/tmp/*aij-lab*|/tmp/*aij-lab*) ;; *) exit 2 ;; esac
trap 'exit 0' TERM INT HUP
end=$((SECONDS + {HOLD_SECS}))
while [ "$SECONDS" -lt "$end" ]; do sleep 5 & wait $!; done
"""


# The selftest's stand-in for the Claude app's `disclaimer --pgroup --`.
FAKE_DISCLAIMER = """import os, sys
if sys.argv[1] == "--pgroup":
    try:
        os.setpgid(0, 0)
    except OSError as error:
        sys.stderr.write(f"Failed to create process group: {error.strerror}\\n")
        sys.exit(1)
os.execv(sys.argv[3], sys.argv[3:])
"""


def hold_command(home: Path, helper: Path | None = None) -> list[str]:
    second = profile_dir(home, SECOND)
    holder = profile_dir(home, HOLDER)
    stand = second / "claude-code" / STANDIN_VERSION / "claude.app/Contents/MacOS/claude"
    return [str(helper or DISCLAIMER), "--pgroup", "--", str(stand), "--resume=lab-add-dir", f"--add-dir={holder / ADD_DIR}"]


def said(err: IO[bytes]) -> str:
    """What a wrapper that ended wrote to stderr: its last lines, one line."""
    err.seek(0)
    lines = [line.strip() for line in err.read().decode(errors="replace").splitlines() if line.strip()]
    return " / ".join(lines[-3:])[:400] or "it said nothing on stderr"


def hold(home: Path, dry: bool, helper: Path | None = None) -> list[str]:
    helper = helper or DISCLAIMER
    argv = hold_command(home, helper)
    pidfile = home / ".aij-lab-add-dir.pid"
    if dry:
        return [f"run {' '.join(argv)}", f"write {pidfile}"]
    if not helper.exists():
        refuse(f"{helper} isn't there (the Claude app's helper): record I3/J2 BLOCKED")
    if not Path(argv[3]).exists():
        refuse("run `seed` first (no stand-in in <profile-C>)")
    if pidfile.exists():
        refuse(f"a hold runs already (pid {pidfile.read_text().strip()}); release-add-dir first")
    # F41 (F-PL1-21): never a new session. `disclaimer --pgroup` makes its
    # own process group (as the Claude app starts it), which a session
    # leader can't ("Failed to create process group: Operation not
    # permitted", exit 1); the group is what release-add-dir ends. Its
    # stderr is kept, so an early end says why.
    with tempfile.TemporaryFile() as err:
        child = subprocess.Popen(argv, cwd=str(home), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                 stderr=err, env={"HOME": str(home), "PATH": "/usr/bin:/bin"})
        time.sleep(1)
        if child.poll() is not None:
            refuse(f"the disclaimer wrapper ended at once (exit {child.returncode}: {said(err)}): "
                   "record I3/J2 BLOCKED with this line")
    pidfile.write_text(f"{child.pid}\n")
    return [f"started the wrapper (pid {child.pid}): {' '.join(argv)}"]


def release(home: Path) -> list[str]:
    pidfile = home / ".aij-lab-add-dir.pid"
    if not pidfile.exists():
        return ["no hold runs"]
    pid = int(pidfile.read_text().strip())
    argv = hold_command(home)
    alive = [a for a in processes_with_pids() if a[0] == pid]
    if alive and argv[3] in alive[0][1]:
        for kill in (lambda: os.killpg(pid, signal.SIGTERM), lambda: os.kill(pid, signal.SIGTERM)):
            try:
                kill()
            except ProcessLookupError:
                pass
    pidfile.unlink()
    return [f"ended the hold (pid {pid})" if alive else f"the hold (pid {pid}) had ended"]


def processes_with_pids() -> list[tuple[int, str]]:
    out = subprocess.run(["ps", "-axww", "-o", "pid=,args="], capture_output=True, text=True, check=True).stdout
    rows = []
    for line in out.splitlines():
        pid, _, args = line.strip().partition(" ")
        if pid.isdigit():
            rows.append((int(pid), args))
    return rows


def lab_profiles(home: Path) -> list[Path]:
    root = home / PROFILES
    return sorted(guard(p, home) for p in root.iterdir() if p.is_dir()) if root.is_dir() else []


def record_holders(home: Path) -> list[tuple[Path, Path, dict]]:
    """(profile, record file, record) for each record naming the worktree."""
    out = []
    for profile in lab_profiles(home):
        for path in sorted((profile / SESSIONS).glob("*/*/local_*.json")):
            try:
                record = json.loads(path.read_text())
            except (OSError, ValueError):
                continue
            if record.get("worktreeName") == WORKTREE:
                out.append((profile, path, record))
    return out


def split(home: Path, claim_in: str, dry: bool, table: list[str] | None = None) -> list[str]:
    w = Writer(home, dry, table)
    for profile in lab_profiles(home):
        w.quit_check(profile)
    holders = record_holders(home)
    if len(holders) != 1:
        refuse(f"{len(holders)} records name {WORKTREE}; run `seed` (and the walk's move) first")
    holder, record_path, record = holders[0]
    owner = profile_dir(home, claim_in)
    if owner == holder:
        refuse(f"the record sits in {claim_in} already: a split needs the claim in another profile (run the move first)")
    entry = None
    for profile in lab_profiles(home):
        file = read_worktrees_file(profile)
        if file and WORKTREE in file.get("worktrees", {}):
            entry = entry or dict(file["worktrees"][WORKTREE])
            if profile != owner:
                del file["worktrees"][WORKTREE]
                w.write(profile / WORKTREES_FILE, dumps(file), f"{profile.name} no longer leases {WORKTREE}")
    if entry is None:
        refuse(f"no profile leases {WORKTREE}")
    entry["leasedBy"] = record["sessionId"]
    file = read_worktrees_file(owner) or fresh_worktrees_file()
    file.setdefault("worktrees", {})[WORKTREE] = entry
    w.write(owner / WORKTREES_FILE, dumps(file), f"{owner.name} leases {WORKTREE} while {holder.name} holds the record")
    patch_record(w, record_path, {"branch": STALE}, f"the record says {STALE}, the worktree is on its own branch")
    return w.planned


def sha(path: Path) -> str | None:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16] if path.is_file() else None


def show(home: Path) -> dict:
    worktree = home / "repos" / REPO / ".claude" / "worktrees" / WORKTREE
    holders = {p.name: r.get("branch") for p, _, r in record_holders(home)}
    profiles = {}
    for profile in lab_profiles(home):
        file = read_worktrees_file(profile)
        entry = (file or {}).get("worktrees", {}).get(WORKTREE)
        sightings = (file or {}).get("untrackedDirGc", {}).get("sightings", {})
        profiles[profile.name] = {
            "file_sha256": sha(profile / WORKTREES_FILE),
            "file_mtime": int((profile / WORKTREES_FILE).stat().st_mtime) if (profile / WORKTREES_FILE).is_file() else None,
            "gc_latest": max((v for v in (file or {}).get("untrackedDirGc", {}).get("roots", {}).values()
                              if isinstance(v, (int, float))), default=None),
            "lease": {"leasedBy": entry.get("leasedBy"), "branch": entry.get("branch")} if entry else None,
            "sighted": any(os.path.realpath(k) == os.path.realpath(worktree) for k in sightings),
            "holds_record": profile.name in holders,
            "record_branch": holders.get(profile.name),
        }
    return {"worktree": WORKTREE, "exists": worktree.is_dir(), "head": head(worktree, home), "profiles": profiles}


def selftest() -> None:
    real = real_home()
    for bad in [real / PROFILES / "work", real / PROFILES / "default", real / "Library/Application Support/Claude", real / ".claude",
                real / CODE / "work", real / ".codex"]:
        try:
            guard(bad, Path("/private/tmp/aij-lab-selftest-none"))
        except Refused:
            continue
        raise AssertionError(f"{bad} wasn't refused")
    for bad_home in [str(real / "aij-lab-x"), "/tmp/aij-lab-x/../../Users", str(real)]:
        try:
            lab_home(bad_home)
        except Refused:
            continue
        raise AssertionError(f"{bad_home} wasn't refused as a lab")
    lab = Path(tempfile.mkdtemp(prefix="aij-lab-selftest-", dir="/tmp"))
    outside = Path(tempfile.mkdtemp(prefix="aij-notlab-", dir="/tmp"))
    try:
        home = lab_home(str(lab))
        for name in ("work", "personal"):
            (home / PROFILES / name).mkdir(parents=True)
            (home / CODE / name).mkdir(parents=True)
        (home / PROFILES / "work" / SESSIONS / "lab-account-work" / "lab-org-work").mkdir(parents=True)
        for repo in ("web", "api"):
            (home / "repos" / repo).mkdir(parents=True)
            env = git_env(home)
            subprocess.run(["git", "init", "-q", "-b", "main", str(home / "repos" / repo)], check=True, env=env)
            (home / "repos" / repo / "README.md").write_text("lab\n")
            subprocess.run(["git", "-C", str(home / "repos" / repo), "add", "README.md"], check=True, env=env)
            subprocess.run(["git", "-C", str(home / "repos" / repo), "-c", "user.name=lab", "-c", "user.email=lab@aij-lab.invalid",
                            "-c", "commit.gpgsign=false", "commit", "-qm", "readme"], check=True, env=env)
        # The roots themselves, and a symlink out of the lab, are refused.
        for bad in [home / PROFILES, home / "repos", home / "elsewhere"]:
            try:
                guard(bad, home)
            except Refused:
                continue
            raise AssertionError(f"{bad} wasn't refused")
        (home / PROFILES / "escape").symlink_to(outside)
        try:
            guard(home / PROFILES / "escape" / WORKTREES_FILE, home)
            raise AssertionError("a symlink out of the lab wasn't refused")
        except Refused:
            pass
        (home / PROFILES / "escape").unlink()

        work_dir = home / PROFILES / "work"
        assert holder_records(work_dir, home).name == "lab-org-work"
        (work_dir / SESSIONS / "acct-real" / "org-real").mkdir(parents=True)
        assert holder_records(work_dir, home).name == "org-real", "the signed-in folder wasn't picked"
        shutil.rmtree(work_dir / SESSIONS / "acct-real")

        def files() -> list[str]:
            return sorted(str(p) for p in home.rglob("*"))

        before = files()
        planned = seed(home, dry=True, table=[])
        assert files() == before, "a dry run wrote something"
        assert any(WORKTREES_FILE in line for line in planned), planned
        running = [f"/Applications/Claude.app/Contents/MacOS/Claude --user-data-dir={home / PROFILES / 'work'}"]
        try:
            seed(home, dry=False, table=running)
            raise AssertionError("seeded while work's app runs")
        except Refused:
            pass
        assert files() == before, "a refused seed wrote something"
        seed(home, dry=False, table=[])
        spec = formats()
        work = home / PROFILES / "work"
        file = json.loads((work / WORKTREES_FILE).read_text())
        assert file["schemaVersion"] == spec["worktree_file"]["schemaVersion"]
        assert set(spec["worktree_file"]["top"]) <= set(file), file.keys()
        lease = file["worktrees"][WORKTREE]
        kinds = {"string": str, "number": int, "boolean": bool, "object": dict, "array": list}
        for key, kind in {**spec["worktree_file"]["lease"]["required"], **spec["worktree_file"]["lease"]["typed"]}.items():
            if key in lease:
                assert isinstance(lease[key], kinds[kind]), key
        assert set(spec["worktree_file"]["lease"]["required"]) <= set(lease)
        assert set(spec["cleanup_sightings"]["keys"]) <= set(file["untrackedDirGc"])
        state = show(home)
        assert state["head"] == BRANCH and state["exists"], state
        assert state["profiles"]["work"]["lease"]["branch"] == BRANCH and state["profiles"]["work"]["holds_record"]
        record_path = next((work / SESSIONS).glob("*/*/local_*.json"))
        record = json.loads(record_path.read_text())
        assert lease["leasedBy"] == record["sessionId"] == record_path.stem
        for key in spec["session_record"]["required"]:
            assert isinstance(record[key], str), key
        assert record["worktreeName"] == WORKTREE and record["branch"] == BRANCH
        second = home / PROFILES / SECOND
        other = json.loads(next((second / SESSIONS / "lab-account-personal" / "lab-org-personal").glob("local_*.json")).read_text())
        update = other["sessionPermissionUpdates"][0]
        assert set(spec["session_record"]["permission_update"]["keys"]) <= set(update)
        assert update["directories"] == [str(work / ADD_DIR)]
        assert json.loads((home / CODE / SECOND / ".claude.json").read_text())["oauthAccount"]["accountUuid"] == "lab-account-personal"
        argv = hold_command(home)
        assert argv[0] == str(DISCLAIMER) and argv[3].startswith(str(second)) and argv[-1] == f"--add-dir={work / ADD_DIR}"
        assert hold(home, dry=True)[0].startswith("run ")
        # F41: a stand-in `disclaimer` that, as the real one, makes its own
        # process group (refused to a session leader) and runs the rest.
        fake = lab / "fake-disclaimer"
        fake.write_text(f"#!{sys.executable} -I\n" + FAKE_DISCLAIMER)
        fake.chmod(0o755)
        # And the Claude app's own helper, when it's installed here.
        for helper in [fake] + ([DISCLAIMER] if DISCLAIMER.exists() else []):
            try:
                started = hold(home, dry=False, helper=helper)
            except Refused as refused:
                raise AssertionError(f"hold-add-dir refused under {helper.name}: {refused}") from None
            pid = int((home / ".aij-lab-add-dir.pid").read_text())
            assert started[0].startswith(f"started the wrapper (pid {pid})"), started
            assert os.getpgid(pid) == pid, "the wrapper isn't its own process group"
            assert release(home) == [f"ended the hold (pid {pid})"]
            for _ in range(50):
                if not any(p == pid or str(second) in a for p, a in processes_with_pids()):
                    break
                time.sleep(0.1)
            else:
                raise AssertionError("release-add-dir left the hold running")
            assert not (home / ".aij-lab-add-dir.pid").exists()
        # One that ends at once says why (its stderr), and leaves no pid file.
        fake.write_text("#!/bin/bash\necho 'Failed to spawn process: lab says no' >&2\nexit 1\n")
        try:
            hold(home, dry=False, helper=fake)
            raise AssertionError("a wrapper that ended at once wasn't refused")
        except Refused as refused:
            assert "exit 1: Failed to spawn process: lab says no" in str(refused), refused
        assert not (home / ".aij-lab-add-dir.pid").exists()
        fake.unlink()
        # A split: the walk's move put record and lease in personal; split
        # puts the lease back in work and the record's branch off HEAD.
        personal = home / PROFILES / "personal"
        target = personal / SESSIONS / "lab-account-personal" / "lab-org-personal"
        target.mkdir(parents=True)
        shutil.move(str(record_path), str(target / record_path.name))
        moved = json.loads((work / WORKTREES_FILE).read_text())
        entry = moved["worktrees"].pop(WORKTREE)
        (work / WORKTREES_FILE).write_text(json.dumps(moved))
        (personal / WORKTREES_FILE).write_text(json.dumps({**fresh_worktrees_file(), "worktrees": {WORKTREE: entry}}))
        try:
            split(home, "work", dry=False, table=[f"x --user-data-dir={personal}"])
            raise AssertionError("split while personal's app runs")
        except Refused:
            pass
        try:
            split(home, "personal", dry=False, table=[])
            raise AssertionError("split into the holder")
        except Refused:
            pass
        sums = (sha(work / WORKTREES_FILE), sha(personal / WORKTREES_FILE))
        split(home, "work", dry=True, table=[])
        assert (sha(work / WORKTREES_FILE), sha(personal / WORKTREES_FILE)) == sums, "a dry split wrote"
        split(home, "work", dry=False, table=[])
        state = show(home)
        assert state["profiles"]["work"]["lease"] is not None and state["profiles"]["personal"]["lease"] is None, state
        assert state["profiles"]["personal"]["holds_record"] and state["profiles"]["personal"]["record_branch"] == STALE
        assert state["head"] == BRANCH
    finally:
        shutil.rmtree(lab)
        shutil.rmtree(outside)
    print("lab_seed_worktrees selftest: ok")


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        return 0
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", choices=["seed", "hold-add-dir", "release-add-dir", "split", "show"])
    parser.add_argument("--home", required=True)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--claim-in", default=HOLDER)
    args = parser.parse_args()
    home = lab_home(args.home)
    if args.command == "show":
        print(json.dumps(show(home), indent=2))
        return 0
    lines = {"seed": lambda: seed(home, args.dry_run), "hold-add-dir": lambda: hold(home, args.dry_run),
             "release-add-dir": lambda: release(home), "split": lambda: split(home, args.claim_in, args.dry_run)}[args.command]()
    prefix = "would " if args.dry_run else ""
    for line in lines:
        print(f"lab-seed-worktrees: {prefix}{line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
