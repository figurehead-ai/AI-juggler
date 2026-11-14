#!/usr/bin/env python3
"""AJ-E1612 (D-107): the live-check lab's sign-ins, checked without
reading one.

`lab_scenario.sh setup T16` seeds two fresh Apps per tool (OpenCode, Kimi
Code, Grok Build, ZCode, Muse Code) and records each App's Sign in, as
its action runs it, in <lab>/sign-ins.json. This script holds that to the
lab, and watches the real home while the maintainer signs lab A in:

  check --home LAB [--signed-in TOOL ...] [--record FILE]
      ten Apps, two per tool, each a fresh App with its launch settings,
      each folder in the lab; every folder its Sign in names is the lab's
      (or the tool's own app bundle); Muse's runs on the file backend
      (TBH_CREDENTIAL_BACKEND=file exactly, HOME and every XDG root in its
      folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH). With --signed-in,
      that tool's lab A has its sign-in file and lab B none; without,
      none has one. Files are only looked at by name.
  real --home LAB --save | --compare
      the real home's sign-in files for the five tools: each one's
      modification time, by name (never opened), saved in the lab and
      compared after a sign-in.
  muse-gate --home LAB --save | --compare
      Muse's version (1.4.13 or stop: the file backend's proof is bound to
      it) and the real Keychain item's modification date, from
      `security find-generic-password` without -w (never the secret).
  --selftest

`--record FILE` appends what was checked to an evidence file, through the
repo's anonymizer (scripts/qa/anonymize.py): no real name or path.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import pwd
import re
import sqlite3
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
DATA = "Library/Application Support/AI Juggler Dev"
LAB = re.compile(r"^(/private)?/tmp/aij-lab[^/]*$")


def _anonymizer():
    """scripts/qa/anonymize.py, loaded by its path (python3 -I keeps the
    script's folder off sys.path)."""
    spec = importlib.util.spec_from_file_location("anonymize", HERE / "anonymize.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module.anonymize


@dataclass(frozen=True)
class Tool:
    """A tool's Sign in and where its sign-in lands (the adapters' own
    names: crates/juggler-adapters/src/sign_in.rs, held by the selftest)."""

    name: str
    words: tuple[str, ...]  # its sign-in command's words, in order
    lab: tuple[str, ...]  # its sign-in, relative to a lab App's folder
    real: tuple[str, ...]  # its sign-in files in the real home (a folder: each file in it)


TOOLS: dict[str, Tool] = {
    # XDG_DATA_HOME=<folder>/data; `opencode auth login` → OpenAI → ChatGPT.
    "opencode": Tool("OpenCode", ("auth", "login"), ("data/opencode/auth.json",),
                     (".local/share/opencode/auth.json", ".local/share/opencode/mcp-auth.json")),
    # KIMI_CODE_HOME=<folder>; `kimi login` (a device code).
    "kimi-code": Tool("Kimi Code", ("login",), ("credentials/kimi-code.json",), (".kimi-code/credentials",)),
    # GROK_HOME=<folder>; `grok login` (the browser).
    "grok-build": Tool("Grok Build", ("login",), ("auth.json",), (".grok/auth.json", ".grok/mcp_credentials.json")),
    # ZCODE_DATA_BASE_DIR=<folder>; the engine's `login` (Z.AI).
    "zcode": Tool("ZCode", ("login",), (".zcode/v2/credentials.json",), (".zcode/v2/credentials.json",)),
    # HOME=<folder>, XDG_CONFIG_HOME=<folder>/.config, the file backend.
    "muse-code": Tool("Muse Code", ("login",), (".config/muse/auth.json",), (".config/muse/auth.json",)),
}
MUSE_PROVEN = "1.4.13"
MUSE_KEYCHAIN = ("ai.meta.dev.credentials", "meta")
SECURITY = "/usr/bin/security"


def lab_home(text: str) -> Path:
    if not LAB.match(text.rstrip("/")) or ".." in Path(text).parts:
        sys.exit(f"lab-sign-ins: {text} isn't a lab (/tmp/aij-lab…) — refusing")
    return Path(text.rstrip("/"))


def real_home() -> Path:
    """The real home, from the user record (a walker's shell may have the
    lab's HOME)."""
    return Path(pwd.getpwuid(os.getuid()).pw_dir)


def inside(path: str, root: Path) -> bool:
    bare = str(root).removeprefix("/private")
    return any(path == r or path.startswith(r + "/") for r in (bare, f"/private{bare}"))


def bundle_of(program: str) -> str | None:
    """The .app a program lies in (ZCode's engine runs on its app's runtime)."""
    at = program.find(".app/")
    return program[: at + 4] if at >= 0 else None


class Report:
    def __init__(self) -> None:
        self.lines: list[str] = []
        self.failed = False

    def ok(self, text: str) -> None:
        self.lines.append(f"ok    {text}")

    def fail(self, text: str) -> None:
        self.failed = True
        self.lines.append(f"FAIL  {text}")

    def show(self, lab: Path, record: str | None, title: str) -> int:
        anonymize = _anonymizer()
        bare = str(lab).removeprefix("/private")
        text = "\n".join(line.replace(f"/private{bare}", "<lab>").replace(bare, "<lab>") for line in self.lines)
        text = anonymize(text, home=str(real_home()))
        print(text)
        if record:
            out = Path(record)
            out.parent.mkdir(parents=True, exist_ok=True)
            with out.open("a") as handle:
                handle.write(f"\n### {title}\n\n```text\n{text}\n```\n")
            print(f"lab-sign-ins: recorded in {out}")
        print("lab-sign-ins: " + ("FAILED" if self.failed else "all checks pass"))
        return 1 if self.failed else 0


def read_only(path: Path, query: str) -> list[tuple]:
    for mode in ("mode=ro", "immutable=1"):
        try:
            db = sqlite3.connect(f"{path.as_uri()}?{mode}", uri=True)
            try:
                return list(db.execute(query))
            finally:
                db.close()
        except sqlite3.OperationalError:
            continue
    sys.exit(f"lab-sign-ins: {path} can't be read")


def check_entry(entry: dict, lab: Path, report: Report) -> None:
    app, tool = entry.get("id", "?"), entry.get("tool", "?")
    spec = TOOLS.get(tool)
    folder = str(entry.get("folder", ""))
    if spec is None:
        report.fail(f"{app}: {tool} isn't one of the five tools")
        return
    if not inside(folder, lab):
        report.fail(f"{app}: its folder {folder} isn't in the lab")
        return
    if "refused" in entry:
        report.fail(f"{app}: its Sign in is refused: {entry['refused']}")
        return
    args = list(entry.get("args", []))
    words = list(spec.words)
    if not any(args[at : at + len(words)] == words for at in range(len(args))):
        report.fail(f"{app}: its Sign in runs {args}, not {' '.join(words)}")
    program = str(entry.get("program", ""))
    if not Path(program).exists():
        report.fail(f"{app}: its program {program} isn't there")
    if not inside(str(entry.get("cwd", "")), lab):
        report.fail(f"{app}: its Sign in runs outside the lab")
    bundle = bundle_of(program)
    env: dict = entry.get("env", {})
    for key, value in sorted(env.items()):
        if isinstance(value, str) and value.startswith("/"):
            if not (inside(value, lab) or (bundle and inside(value, Path(bundle)))):
                report.fail(f"{app}: {key} points outside the lab ({value})")
    if tool == "muse-code":
        muse = {
            "TBH_CREDENTIAL_BACKEND": env.get("TBH_CREDENTIAL_BACKEND") == "file",
            "HOME is its folder": env.get("HOME") == folder,
            "MUSE_NO_AUTO_UPDATE=1": env.get("MUSE_NO_AUTO_UPDATE") == "1",
            "no MUSE_AUTH_PATH": "MUSE_AUTH_PATH" in env and env["MUSE_AUTH_PATH"] is None,
        }
        for root in ("XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME"):
            muse[f"{root} in its folder"] = isinstance(env.get(root), str) and inside(env[root], Path(folder))
        for inherited in ("XDG_RUNTIME_DIR", "XDG_CONFIG_DIRS", "XDG_DATA_DIRS"):
            muse[f"no {inherited}"] = inherited in env and env[inherited] is None
        bad = [name for name, good in muse.items() if not good]
        if bad:
            report.fail(f"{app}: Muse's lab environment is wrong: {', '.join(bad)}")
        else:
            report.ok(f"{app}: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, "
                      "MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH")
    shown = " ".join([Path(program).name, *args])
    report.ok(f"{app}: Sign in runs `{shown}` in {folder}")


def check(lab: Path, signed_in: list[str], record: str | None) -> int:
    report = Report()
    path = lab / "sign-ins.json"
    if not path.is_file():
        sys.exit(f"lab-sign-ins: {path} isn't there (set the lab up with lab_scenario.sh setup T16)")
    entries = json.loads(path.read_text())
    by_tool: dict[str, list[str]] = {}
    for entry in entries:
        by_tool.setdefault(entry.get("tool", "?"), []).append(entry.get("id", "?"))
    if len(entries) == 10 and set(by_tool) == set(TOOLS) and all(len(ids) == 2 for ids in by_tool.values()):
        report.ok("ten Apps, two per tool: " + ", ".join(f"{TOOLS[t].name} ×2" for t in TOOLS))
    else:
        report.fail(f"{len(entries)} App(s): {by_tool}")
    folders = [str(entry.get("folder", "")) for entry in entries]
    if len(set(folders)) != len(folders) or any(a != b and inside(a, Path(b)) for a in folders for b in folders):
        report.fail("two Apps share a folder, or one lies in another's")
    for entry in entries:
        check_entry(entry, lab, report)
    # AI Juggler's own data: each a fresh App with its launch settings.
    rows = read_only(lab / DATA / "juggler.sqlite",
                     "select a.provider_id, a.label, a.state_dir, "
                     "(select count(*) from launch_recipes r where r.account_id = a.id and r.removed_at is null) "
                     "from accounts a")
    mine = {row[2]: row for row in rows if row[0] in TOOLS}
    for entry in entries:
        row = mine.get(str(entry.get("folder")))
        if row is None:
            report.fail(f"{entry.get('id')}: no App in AI Juggler's data for its folder")
        elif row[3] < 1:
            report.fail(f"{entry.get('id')}: its App has no launch settings")
    if len(mine) == len(entries) == len(rows):
        report.ok(f"AI Juggler Dev's data: {len(rows)} Apps, each with its launch settings, nothing else")
    elif len(rows) != len(entries):
        report.fail(f"AI Juggler Dev's data has {len(rows)} Apps, the lab {len(entries)}")
    if (lab / "repos" / "lab").is_dir():
        report.ok("the throwaway project: <lab>/repos/lab")
    else:
        report.fail("no throwaway project at <lab>/repos/lab")
    # Sign-ins by name only: none in setup; lab A only once signed in.
    for entry in entries:
        tool, app = entry.get("tool"), str(entry.get("id"))
        if tool not in TOOLS:
            continue
        has = any((Path(str(entry["folder"])) / name).exists() for name in TOOLS[tool].lab)
        want = tool in signed_in and app.endswith("-a")
        if has == want:
            report.ok(f"{app}: {'signed in' if has else 'no sign-in'} in its own folder")
        else:
            report.fail(f"{app}: {'has a sign-in it should not' if has else 'has no sign-in'}")
    return report.show(lab, record, "check" + (f" (signed in: {', '.join(signed_in)})" if signed_in else ""))


def real_files(home: Path) -> dict[str, int | None]:
    """Each real sign-in file's modification time (ns), by name; a folder's
    files one by one. Never opened."""
    out: dict[str, int | None] = {}
    for spec in TOOLS.values():
        for name in spec.real:
            path = home / name
            if path.is_dir():
                for child in sorted(path.rglob("*")):
                    if child.is_file():
                        out[f"~/{child.relative_to(home)}"] = child.stat().st_mtime_ns
                out[f"~/{name}/"] = path.stat().st_mtime_ns
            else:
                out[f"~/{name}"] = path.stat().st_mtime_ns if path.exists() else None
    return out


def compare(before: dict, now: dict, report: Report) -> None:
    for name in sorted(set(before) | set(now)):
        if before.get(name) == now.get(name):
            report.ok(f"{name}: unchanged" + ("" if now.get(name) is not None else " (not there)"))
        else:
            report.fail(f"{name}: CHANGED ({'appeared' if before.get(name) is None else 'gone' if now.get(name) is None else 'modified'})")


def real(lab: Path, save: bool, record: str | None) -> int:
    saved = lab / "real-sign-ins.json"
    now = real_files(real_home())
    if save:
        saved.write_text(json.dumps(now, indent=1))
        print(f"lab-sign-ins: the real home's {len(now)} sign-in file(s) recorded by name and date")
        return 0
    report = Report()
    compare(json.loads(saved.read_text()), now, report)
    return report.show(lab, record, "the real home's sign-in files")


def real_env() -> dict[str, str]:
    """This shell's environment on the real home (a walker's shell may
    carry the lab's HOME; the Keychain and the user's own Muse are the
    real home's)."""
    return {**os.environ, "HOME": str(real_home())}


def muse_version() -> str | None:
    """`muse --version` on the PATH (updates off), as the user's own Muse
    runs: `Muse Code 1.4.13 (1.4.13-R6996.1)`."""
    try:
        done = subprocess.run(["muse", "--version"], capture_output=True, text=True, timeout=30,
                              env={**real_env(), "MUSE_NO_AUTO_UPDATE": "1"}, stdin=subprocess.DEVNULL)
    except (OSError, subprocess.TimeoutExpired):
        return None
    found = re.search(r"\b(\d+\.\d+\.\d+)\b", done.stdout)
    return found.group(1) if found else None


def keychain_mdat(listing: str) -> str | None:
    """The `mdat` attribute of `security find-generic-password`'s listing
    (without -w: attributes only); nothing else is kept."""
    found = re.search(r'"mdat"<timedate>=\S*\s+"([0-9]{14}Z)', listing)
    return found.group(1) if found else None


def muse_item_mdat() -> str | None:
    service, account = MUSE_KEYCHAIN
    done = subprocess.run([SECURITY, "find-generic-password", "-s", service, "-a", account],
                          capture_output=True, text=True, timeout=30, stdin=subprocess.DEVNULL, env=real_env())
    return keychain_mdat(done.stdout) if done.returncode == 0 else None


def muse_gate(lab: Path, save: bool, record: str | None) -> int:
    saved = lab / "muse-gate.json"
    version = muse_version()
    now = {"version": version, "keychain_item_mdat": muse_item_mdat()}
    report = Report()
    if version != MUSE_PROVEN:
        report.fail(f"Muse is {version or 'not found'}, not {MUSE_PROVEN}: STOP — the file backend's proof is bound to "
                    f"{MUSE_PROVEN}; skip Muse in this sitting")
        return report.show(lab, record, "Muse's gate")
    report.ok(f"Muse {version}")
    if save:
        saved.write_text(json.dumps(now))
        report.ok("the real Keychain item's date recorded" if now["keychain_item_mdat"] else
                  "no real Keychain item (the user's own Muse isn't signed in): recorded as none")
        return report.show(lab, record, "Muse's gate (before)")
    before = json.loads(saved.read_text())
    if before.get("keychain_item_mdat") == now["keychain_item_mdat"]:
        report.ok("the real Keychain item is unchanged: the lab's sign-in never reached it")
    else:
        report.fail("the real Keychain item CHANGED: STOP. Tell the maintainer to run `muse login` once in their own "
                    "terminal (their own Muse's sign-in was replaced)")
    return report.show(lab, record, "Muse's gate (after)")


def selftest() -> int:
    import tempfile

    for bad in ("/Users/you", "/tmp/other", "/tmp/aij-lab-x/../y", "/tmp/aij-lab-x/inner"):
        try:
            lab_home(bad)
            raise AssertionError(f"accepted {bad}")
        except SystemExit:
            pass
    assert lab_home("/tmp/aij-lab-t16") == Path("/tmp/aij-lab-t16")
    assert inside("/private/tmp/aij-lab-t16/x", Path("/tmp/aij-lab-t16"))
    assert not inside("/tmp/aij-lab-t16x/y", Path("/tmp/aij-lab-t16"))
    assert bundle_of("/Applications/ZCode.app/Contents/MacOS/ZCode") == "/Applications/ZCode.app"
    assert bundle_of("/opt/homebrew/bin/opencode") is None
    listing = ('keychain: "/Users/you/Library/Keychains/login.keychain-db"\nattributes:\n'
               '    "acct"<blob>="meta"\n    "mdat"<timedate>=0x32303236  "20261113101500Z\\000"\n')
    assert keychain_mdat(listing) == "20261113101500Z"
    assert keychain_mdat("nothing") is None
    # The adapters' own sign-in names (sign_in.rs) are the ones above.
    names = (HERE.parent.parent / "crates/juggler-adapters/src/sign_in.rs").read_text()
    for constant, value in (("OPENCODE_AUTH", "auth.json"), ("OPENCODE_MCP_AUTH", "mcp-auth.json"),
                            ("GROK_AUTH", "auth.json"), ("GROK_MCP_AUTH", "mcp_credentials.json"),
                            ("KIMI_CREDENTIALS", "credentials"), ("KIMI_MEMBERSHIP", "kimi-code.json"),
                            ("ZCODE_SIGN_IN_PATH", "v2/credentials.json"), ("MUSE_AUTH", "auth.json"),
                            ("MUSE_KEYCHAIN_SERVICE", MUSE_KEYCHAIN[0])):
        assert f'{constant}: &str = "{value}"' in names, constant
    with tempfile.TemporaryDirectory(prefix="aij-lab-selftest-", dir="/tmp") as tmp:
        lab = lab_home(tmp)
        entries = []
        for tool in TOOLS:
            for half in "ab":
                folder = lab / DATA / "accounts" / tool / f"{tool}-{half}"
                folder.mkdir(parents=True)
                env = {"HOME": str(folder), "XDG_CONFIG_HOME": f"{folder}/.config", "XDG_DATA_HOME": f"{folder}/.local/share",
                       "XDG_CACHE_HOME": f"{folder}/.cache", "XDG_STATE_HOME": f"{folder}/.local/state",
                       "XDG_RUNTIME_DIR": None, "XDG_CONFIG_DIRS": None, "XDG_DATA_DIRS": None,
                       "TBH_CREDENTIAL_BACKEND": "file", "MUSE_NO_AUTO_UPDATE": "1", "MUSE_AUTH_PATH": None}
                entries.append({"id": f"{tool.split('-')[0]}-{half}", "tool": tool, "folder": str(folder),
                                "program": sys.executable, "args": list(TOOLS[tool].words), "cwd": str(folder),
                                "env": env if tool == "muse-code" else {}})
        (lab / "sign-ins.json").write_text(json.dumps(entries))
        (lab / "repos" / "lab").mkdir(parents=True)
        db = sqlite3.connect(lab / DATA / "juggler.sqlite")
        db.execute("create table accounts (id, provider_id, label, state_dir)")
        db.execute("create table launch_recipes (id, account_id, removed_at)")
        for n, entry in enumerate(entries):
            db.execute("insert into accounts values (?, ?, ?, ?)", (f"a{n}", entry["tool"], "lab", entry["folder"]))
            db.execute("insert into launch_recipes values (?, ?, null)", (f"r{n}", f"a{n}"))
        db.commit()
        db.close()
        assert check(lab, [], None) == 0
        # Lab A of Muse signed in: only it, and only when said so.
        signed = Path(entries[8]["folder"]) / TOOLS["muse-code"].lab[0]
        signed.parent.mkdir(parents=True)
        signed.write_text("{}")
        assert check(lab, [], None) == 1
        assert check(lab, ["muse-code"], None) == 0
        # A folder outside the lab, or Muse off the file backend, fails.
        entries[0]["env"] = {"XDG_DATA_HOME": "/Users/you/.local/share"}
        entries[9]["env"]["TBH_CREDENTIAL_BACKEND"] = "keychain"
        (lab / "sign-ins.json").write_text(json.dumps(entries))
        assert check(lab, ["muse-code"], None) == 1
        # The real home's files, compared by date.
        report = Report()
        compare({"~/a": 1, "~/b": None}, {"~/a": 1, "~/b": None}, report)
        assert not report.failed
        compare({"~/a": 1}, {"~/a": 2}, report)
        assert report.failed
    print("lab_sign_ins: selftest passed")
    return 0


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["check", "real", "muse-gate"])
    parser.add_argument("--home", required=True, help="the lab's HOME (/tmp/aij-lab-…)")
    parser.add_argument("--signed-in", nargs="*", default=[], choices=sorted(TOOLS), help="tools whose lab A is signed in")
    when = parser.add_mutually_exclusive_group()
    when.add_argument("--save", action="store_true", help="record the state before a sign-in")
    when.add_argument("--compare", action="store_true", help="compare with the recorded state")
    parser.add_argument("--record", help="append the result to this evidence file (anonymized)")
    args = parser.parse_args()
    lab = lab_home(args.home)
    if args.command == "check":
        return check(lab, args.signed_in, args.record)
    if not (args.save or args.compare):
        parser.error(f"{args.command} needs --save or --compare")
    if args.command == "real":
        return real(lab, args.save, args.record)
    return muse_gate(lab, args.save, args.record)


if __name__ == "__main__":
    sys.exit(main())
