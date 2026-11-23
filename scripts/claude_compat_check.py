#!/usr/bin/env python3
"""AJ-E2328 req 9: is the installed Claude app still the one a move knows?

A Claude app update can change the files a move and the ownership repair
rewrite (session records, each profile's worktree file and its cleanup
sightings, the side-session notes). The adapter keeps what it knows in one
manifest, `crates/juggler-adapters/src/claude_move/known_formats.json`,
each entry naming the app versions it was read on; the app's version gate
reads the same file. This script compares this Mac with it.

What it reads: the installed app's version (its bundle's Info.plist) and,
in every Claude app profile folder, the session records, `git-worktrees.json`
and the `side-session-notes/` folder, by structure only: key names and value
types. Read-only, no network, no environment variable of any process (the
home folder comes from the account database). It never prints a value, a
title, an id or a path: drift names the manifest entry and the key.

Verdicts (JSON on stdout, exit code):
  ok               0  the version is one the manifest accepts, every file fits
  drift            1  a file has a key or type the manifest doesn't know
  unknown_version  2  the app's version isn't one the manifest accepts
                      (wins over drift; drift is still listed)
  error            3  the manifest can't be read

Usage: claude_compat_check.py [--app BUNDLE] [--profiles DIR ...]
       claude_compat_check.py --selftest   (fixture profiles in a temp dir)
"""
from __future__ import annotations

import argparse
import io
import json
import plistlib
import pwd
import os
import sys
import tempfile
from contextlib import redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "crates/juggler-adapters/src/claude_move/known_formats.json"
RECORD_MAX = 4 * 1024 * 1024
EXIT = {"ok": 0, "drift": 1, "unknown_version": 2, "error": 3}


def home() -> Path:
    """The user's home folder from the account database (no environment)."""
    return Path(pwd.getpwuid(os.getuid()).pw_dir)


def default_app() -> Path:
    return Path("/Applications/Claude.app")


def default_profiles() -> list[Path]:
    support = home() / "Library/Application Support"
    out = [support / "Claude"]
    many = support / "ClaudeProfiles"
    if many.is_dir():
        out += sorted(p for p in many.iterdir() if p.is_dir() and not p.name.startswith("."))
    return [p for p in out if p.is_dir()]


def app_version(bundle: Path) -> str | None:
    try:
        with open(bundle / "Contents/Info.plist", "rb") as handle:
            value = plistlib.load(handle).get("CFBundleShortVersionString")
    except (OSError, plistlib.InvalidFileException, ValueError):
        return None
    return value.strip() if isinstance(value, str) and value.strip() else None


def parse_version(text: str) -> tuple[int, ...] | None:
    parts = text.split(".")
    if not parts or not all(p.isdigit() for p in parts):
        return None
    nums = tuple(int(p) for p in parts)
    return nums + (0,) * (3 - len(nums)) if len(nums) < 3 else nums


def accepted(version: str | None, app: dict) -> bool:
    mine = parse_version(version) if version else None
    if mine is None:
        return False
    for span in app.get("accept", []):
        low = parse_version(span.get("min", "0"))
        high = parse_version(span["below"]) if span.get("below") else None
        if low is not None and mine >= low and (high is None or mine < high):
            return True
    return False


def kind_of(value: object) -> str:
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, (int, float)):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    if isinstance(value, dict):
        return "object"
    return "null"


def fits(value: object, spec: str) -> bool:
    """Whether `value` is of the manifest's type `spec` (`array<string>`,
    `object<number>`, or a plain JSON type)."""
    if spec.startswith("array<"):
        inner = spec[6:-1]
        return isinstance(value, list) and all(kind_of(v) == inner for v in value)
    if spec.startswith("object<"):
        inner = spec[7:-1]
        return isinstance(value, dict) and all(kind_of(v) == inner for v in value.values())
    return kind_of(value) == spec


def read_json(path: Path) -> object | None:
    try:
        if path.is_symlink() or not path.is_file() or path.stat().st_size > RECORD_MAX:
            return None
        return json.loads(path.read_bytes())
    except (OSError, ValueError):
        return None


def check_record(record: object, spec: dict, drift: set[str]) -> None:
    if not isinstance(record, dict):
        drift.add("session_record: not an object")
        return
    for key, want in spec["required"].items():
        if key not in record:
            drift.add(f"session_record.{key}: missing")
        elif not fits(record[key], want):
            drift.add(f"session_record.{key}: {kind_of(record[key])}, expected {want}")
    for key, want in spec["typed"].items():
        if key in record and record[key] is not None and not fits(record[key], want):
            drift.add(f"session_record.{key}: {kind_of(record[key])}, expected {want}")
    grant = spec["permission_update"]
    for update in record.get("sessionPermissionUpdates") or []:
        if not isinstance(update, dict):
            drift.add("session_record.sessionPermissionUpdates[]: not an object")
            continue
        for key in sorted(set(update) - set(grant["keys"])):
            drift.add(f"session_record.sessionPermissionUpdates[].{key}: unknown key")
        if "directories" in update and not fits(update["directories"], grant["directories"]):
            drift.add("session_record.sessionPermissionUpdates[].directories: " + kind_of(update["directories"]))


def check_worktrees(value: object, spec: dict, gc: dict, drift: set[str]) -> None:
    if not isinstance(value, dict):
        drift.add("worktree_file: not an object")
        return
    if value.get("schemaVersion") != spec["schemaVersion"]:
        found = value.get("schemaVersion")
        drift.add(f"worktree_file.schemaVersion: {found if isinstance(found, int) else kind_of(found)}, expected {spec['schemaVersion']}")
    for key, item in value.items():
        want = spec["top"].get(key)
        if want is None:
            drift.add(f"worktree_file.{key}: unknown key")
        elif not fits(item, want):
            drift.add(f"worktree_file.{key}: {kind_of(item)}, expected {want}")
    lease = spec["lease"]
    known = set(lease["required"]) | set(lease["typed"])
    for entry in (value.get("worktrees") or {}).values() if isinstance(value.get("worktrees"), dict) else []:
        if not isinstance(entry, dict):
            drift.add("worktree_file.worktrees.<name>: not an object")
            continue
        for key, want in lease["required"].items():
            if key not in entry:
                drift.add(f"worktree_file.worktrees.<name>.{key}: missing")
            elif not fits(entry[key], want):
                drift.add(f"worktree_file.worktrees.<name>.{key}: {kind_of(entry[key])}, expected {want}")
        for key, item in entry.items():
            if key not in known:
                drift.add(f"worktree_file.worktrees.<name>.{key}: unknown key")
            elif key in lease["typed"] and item is not None and not fits(item, lease["typed"][key]):
                drift.add(f"worktree_file.worktrees.<name>.{key}: {kind_of(item)}, expected {lease['typed'][key]}")
    sweep = value.get("untrackedDirGc")
    if isinstance(sweep, dict):
        for key, item in sweep.items():
            want = gc["keys"].get(key)
            if want is None:
                drift.add(f"cleanup_sightings.{key}: unknown key")
            elif not fits(item, want):
                drift.add(f"cleanup_sightings.{key}: expected {want}")


def check_notes(folder: Path, spec: dict, drift: set[str]) -> int:
    if not folder.is_dir():
        return 0
    count = 0
    for entry in folder.iterdir():
        if entry.name.startswith("."):
            continue
        count += 1
        if entry.is_symlink() or not entry.is_dir():
            drift.add("side_session_notes.<entry>: not a folder")
        elif not entry.name.startswith(spec["named"]):
            drift.add(f"side_session_notes.<entry>: not named {spec['named']}<id>")
    return count


def record_files(profile: Path) -> list[Path]:
    root = profile / "claude-code-sessions"
    out: list[Path] = []
    if not root.is_dir():
        return out
    for account in sorted(root.iterdir()):
        if not account.is_dir() or account.is_symlink():
            continue
        for org in sorted(account.iterdir()):
            if not org.is_dir() or org.is_symlink():
                continue
            out += sorted(p for p in org.glob("local_*.json") if p.is_file() and not p.is_symlink())
    return out


def check(app: Path, profiles: list[Path], manifest_path: Path = MANIFEST) -> tuple[str, dict]:
    try:
        manifest = json.loads(manifest_path.read_text())
        entries = manifest["entries"]
    except (OSError, ValueError, KeyError):
        return "error", {"verdict": "error", "why": "the manifest can't be read"}
    version = app_version(app)
    drift: set[str] = set()
    records = 0
    notes = 0
    for profile in profiles:
        for path in record_files(profile):
            record = read_json(path)
            if record is None:
                continue
            records += 1
            check_record(record, entries["session_record"], drift)
        worktrees = profile / "git-worktrees.json"
        if worktrees.exists():
            value = read_json(worktrees)
            if value is None:
                drift.add("worktree_file: not readable as JSON")
            else:
                check_worktrees(value, entries["worktree_file"], entries["cleanup_sightings"], drift)
        notes += check_notes(profile / "side-session-notes", entries["side_session_notes"], drift)
    ok_version = accepted(version, manifest["app"])
    verdict = "unknown_version" if not ok_version else ("drift" if drift else "ok")
    shown = version if version and parse_version(version) else None
    return verdict, {
        "verdict": verdict,
        "version": shown,
        "checked": manifest["app"].get("checked", []),
        "profiles": len(profiles),
        "records": records,
        "notes_folders": notes,
        "drift": sorted(drift),
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--app", type=Path, default=None)
    parser.add_argument("--profiles", type=Path, nargs="*", default=None)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    verdict, report = check(
        args.app or default_app(),
        args.profiles if args.profiles is not None else default_profiles(),
        args.manifest,
    )
    print(json.dumps(report, separators=(",", ":")))
    return EXIT[verdict]


# ---- selftest: fixture profiles only --------------------------------------

SECRET = "do-not-print-7f3a"


def _fixture(root: Path, version: str) -> tuple[Path, list[Path]]:
    app = root / "Claude.app"
    (app / "Contents").mkdir(parents=True)
    with open(app / "Contents/Info.plist", "wb") as handle:
        plistlib.dump({"CFBundleShortVersionString": version}, handle)
    profile = root / "profiles/alice"
    org = profile / "claude-code-sessions/acct/org"
    org.mkdir(parents=True)
    (org / "local_a.json").write_text(json.dumps({
        "sessionId": "local_a", "cliSessionId": "a", "title": SECRET, "isArchived": False,
        "cwd": "/Users/someone/w/.claude/worktrees/wt-1", "lastActivityAt": 1,
        "worktreeName": "wt-1", "worktreePath": "/Users/someone/w/.claude/worktrees/wt-1",
        "branch": "claude/wt-1", "shellFolders": ["/Users/someone/w"],
        "sessionPermissionUpdates": [{"type": "addDirectories", "destination": "session", "directories": ["/Users/someone/x"]}],
        "someFutureField": {"kept": True},
    }))
    (profile / "git-worktrees.json").write_text(json.dumps({
        "schemaVersion": 2, "originUrls": {}, "originPins": {},
        "worktrees": {"wt-1": {"name": "wt-1", "path": "/Users/someone/w/.claude/worktrees/wt-1", "leasedBy": "local_a",
                               "branch": "claude/wt-1", "baseRepo": "/Users/someone/w", "createdAt": 1,
                               "anchors": [], "autoTrusted": True}},
        "untrackedDirGc": {"sightings": {"/Users/someone/w/.claude/worktrees/old": 5}, "roots": {}, "cwds": {}},
    }))
    (profile / "side-session-notes/local_a").mkdir(parents=True)
    (profile / "side-session-notes/local_a/notes.md").write_text(SECRET)
    return app, [profile]


def _run(app: Path, profiles: list[Path]) -> tuple[int, dict, str]:
    out = io.StringIO()
    with redirect_stdout(out):
        code = main(["--app", str(app), "--profiles", *map(str, profiles)])
    text = out.getvalue()
    return code, json.loads(text), text


def selftest() -> int:
    failures: list[str] = []

    def expect(what: str, cond: bool) -> None:
        if not cond:
            failures.append(what)

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        app, profiles = _fixture(root / "one", "5.11492.5")
        code, report, text = _run(app, profiles)
        expect(f"a known version and shape is ok ({report})", code == 0 and report["verdict"] == "ok")
        expect("counts the record and notes", report["records"] == 1 and report["notes_folders"] == 1)
        expect("never prints a path, a value or a title", tmp not in text and SECRET not in text and "wt-1" not in text)

        # A new lease key and a new sweep key: drift, by exact key.
        path = profiles[0] / "git-worktrees.json"
        value = json.loads(path.read_text())
        value["worktrees"]["wt-1"]["ownerProfile"] = SECRET
        value["untrackedDirGc"]["tombs"] = {}
        value["schemaVersion"] = 3
        path.write_text(json.dumps(value))
        code, report, text = _run(app, profiles)
        expect(f"a new key drifts ({report})", code == 1 and report["verdict"] == "drift")
        expect("names the exact lease key", "worktree_file.worktrees.<name>.ownerProfile: unknown key" in report["drift"])
        expect("names the sweep key", "cleanup_sightings.tombs: unknown key" in report["drift"])
        expect("names the schema", "worktree_file.schemaVersion: 3, expected 2" in report["drift"])
        expect("still prints no value", SECRET not in text and tmp not in text)

        # A record field of another type drifts.
        app2, profiles2 = _fixture(root / "two", "5.11492.7")
        rec = profiles2[0] / "claude-code-sessions/acct/org/local_a.json"
        value = json.loads(rec.read_text())
        value["shellFolders"] = [1]
        value["isArchived"] = "no"
        rec.write_text(json.dumps(value))
        code, report, _ = _run(app2, profiles2)
        expect(f"a patch of the checked build is accepted, a type change drifts ({report})", code == 1)
        expect("names the record keys", {"session_record.shellFolders: array, expected array<string>",
                                         "session_record.isArchived: string, expected boolean"} <= set(report["drift"]))

        # An unchecked version: unknown_version wins, drift still listed.
        app3, profiles3 = _fixture(root / "three", "5.11610.2")
        code, report, _ = _run(app3, profiles3)
        expect(f"an unchecked version is unknown ({report})", code == 2 and report["verdict"] == "unknown_version")
        expect("its version is said", report["version"] == "5.11610.2")
        code, report, _ = _run(root / "missing.app", profiles3)
        expect("no app: unknown version", code == 2 and report["version"] is None)

        # A notes entry that isn't a record's folder.
        (profiles3[0] / "side-session-notes/stray.txt").write_text(SECRET)
        _, report, _ = _run(app, profiles3)
        expect("a stray notes entry drifts", "side_session_notes.<entry>: not a folder" in report["drift"])

        # An unreadable manifest is an error, exit 3.
        bad = root / "bad.json"
        bad.write_text("{")
        out = io.StringIO()
        with redirect_stdout(out):
            code = main(["--app", str(app), "--profiles", str(profiles[0]), "--manifest", str(bad)])
        expect("an unreadable manifest is an error", code == 3)

    # The real manifest is well formed and accepts what it says it checked.
    manifest = json.loads(MANIFEST.read_text())
    expect("every checked version is accepted", all(accepted(v, manifest["app"]) for v in manifest["app"]["checked"]))
    expect("every entry names its versions", all(e.get("verified_on") for e in manifest["entries"].values()))
    for failure in failures:
        print(f"FAIL: {failure}", file=sys.stderr)
    print(f"claude_compat_check selftest: {'FAIL' if failures else 'ok'} ({len(failures)} failures)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
