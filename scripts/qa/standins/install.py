#!/usr/bin/env python3
"""AJ-E1612: puts the lab stand-in tools into a lab, for a dry run.

    python3 -I scripts/qa/standins/install.py --lab /tmp/aij-lab-t16 [--with-hermes]
    python3 -I scripts/qa/standins/install.py --selftest

Copies this folder's stand-ins into `<lab>/standins`, lets each make its
launcher (and ZCode its stand-in app bundle), and writes
`<lab>/standins/env.sh`: the adapters' test overrides (AI_JUGGLER_*_BIN,
AI_JUGGLER_ZCODE_ENGINE) that point AI Juggler Dev at them, plus
AIJ_LAB_STANDINS (`lab_env.sh` passes the overrides on only while that is
set, and only when they point into it). `lab_scenario.sh setup T16
--standins` runs this before the seed; `open` sources the same env.sh.

Nothing here runs a real tool, touches the real home or reaches the network.
"""
from __future__ import annotations

import argparse
import importlib.util
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
# Each stand-in: its file here, and whether it is Hermes (in only --with-hermes).
STANDINS = {
    "opencode": "opencode.py",
    "kimi-code": "kimi.py",
    "grok-build": "grok.py",
    "zcode": "zcode_engine.py",
    "muse-code": "muse.py",
    "hermes": "hermes.py",
}
OPTIONAL = {"hermes"}


def load(path: Path):
    spec = importlib.util.spec_from_file_location(path.stem, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def install(lab: Path, with_hermes: bool) -> dict[str, str]:
    common = load(HERE / "standin_common.py")
    common.require_lab(lab, "install", "the lab")
    if not lab.is_dir():
        sys.exit(f"install: {lab} isn't there")
    dest = lab / "standins"
    dest.mkdir(exist_ok=True)
    shutil.copy2(HERE / "standin_common.py", dest / "standin_common.py")
    env: dict[str, str] = {"AIJ_LAB_STANDINS": str(dest)}
    for tool, name in STANDINS.items():
        if tool in OPTIONAL and not with_hermes:
            continue
        source = HERE / name
        if not source.is_file():
            sys.exit(f"install: the {tool} stand-in ({name}) isn't there")
        shutil.copy2(source, dest / name)
        module = load(dest / name)
        made = module.install(dest)
        for key, value in made.items():
            if not key.startswith("AI_JUGGLER_") or common.lab_root_of(value) is None:
                sys.exit(f"install: {name} asked for {key}={value}, outside the lab or not an adapter override")
            env[key] = value
        print(f"install: {tool} stand-in ready ({', '.join(sorted(made))})")
    (dest / "INSTALLED").write_text("lab stand-ins: a dry run, not a live sitting\n")
    (dest / "env.sh").write_text("".join(f"export {k}={shlex.quote(v)}\n" for k, v in sorted(env.items())))
    print(f"install: overrides in {dest / 'env.sh'}")
    return env


def selftest() -> int:
    common = load(HERE / "standin_common.py")
    common.selftest_common()
    failed = []
    for tool, name in STANDINS.items():
        path = HERE / name
        if not path.is_file():
            failed.append(f"{tool}: {name} missing")
            continue
        done = subprocess.run(["/usr/bin/python3", "-I", str(path), "--selftest"], capture_output=True, text=True, timeout=300)
        print(f"{tool}: {'passed' if done.returncode == 0 else 'FAILED'}")
        if done.returncode != 0:
            failed.append(f"{tool}: {done.stdout[-800:]}{done.stderr[-800:]}")
    with common.selftest_lab("install") as lab:
        env = install(lab, with_hermes=True)
        assert set(env) >= {"AIJ_LAB_STANDINS", "AI_JUGGLER_OPENCODE_BIN", "AI_JUGGLER_KIMI_BIN", "AI_JUGGLER_GROK_BIN",
                            "AI_JUGGLER_ZCODE_ENGINE", "AI_JUGGLER_MUSE_BIN", "AI_JUGGLER_HERMES_BIN"}, env
        for key, value in env.items():
            assert Path(value).exists(), (key, value)
        assert (lab / "standins" / "env.sh").read_text().count("export ") == len(env)
    for line in failed:
        print(line, file=sys.stderr)
    print("standins install: selftest " + ("FAILED" if failed else "passed"))
    return 1 if failed else 0


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--lab", required=True)
    parser.add_argument("--with-hermes", action="store_true")
    args = parser.parse_args()
    install(Path(args.lab), args.with_hermes)
    return 0


if __name__ == "__main__":
    sys.exit(main())
