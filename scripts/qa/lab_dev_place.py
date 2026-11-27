#!/usr/bin/env python3
"""Where a lab's Dev copy may live (0.5.2 G38, the RW1 walk's V1).

AI Juggler runs its own `juggler-agent` only from a folder no other user
can change (AJ-E3013, `file_trust::executable_trust`): every entry on the
way, up to `/` and through any link, is root's or the user's, never
writable by others, and writable by its group only when that group is the
administrators' (wheel, admin). A Dev bundle under `/tmp` fails that
(`/private/tmp` is writable by everyone): Dev refuses its own bridge, no
launch gets hooks, and a session stays "Working" after its turn ended.
So the lab keeps its Dev copy in a folder of the user's own, and refuses
to open one that the app itself would refuse.

  lab_dev_place.py path           the lab's Dev copy: $AIJ_LAB_DEV_DIR, else
                                  ~/Dev/AIJuggler-shared-target/lab-dev, + "/AI Juggler Dev.app"
  lab_dev_place.py check <path>   exit 1, naming the entry, when the app would
                                  refuse what is at <path> (the entries that
                                  exist, as file_trust walks them)
  lab_dev_place.py --selftest
"""
from __future__ import annotations

import os
import stat
import sys
import tempfile
from pathlib import Path

APP = "AI Juggler Dev.app"
# file_trust::ADMIN_GROUPS on a Mac: wheel and admin.
ADMIN_GROUPS = (0, 80)


def default_dir(env: dict[str, str] | None = None) -> Path:
    env = os.environ if env is None else env
    chosen = env.get("AIJ_LAB_DEV_DIR")
    if chosen:
        return Path(chosen)
    return Path(env.get("HOME", os.path.expanduser("~"))) / "Dev/AIJuggler-shared-target/lab-dev"


def entry_trusted(owner: int, group: int, mode: int, is_link: bool, me: int) -> bool:
    """file_trust::entry_trusted, the same rule on raw ids."""
    owner_ok = owner in (0, me)
    others_ok = mode & 0o002 == 0
    group_ok = mode & 0o020 == 0 or group in ADMIN_GROUPS
    return owner_ok and (is_link or (others_ok and group_ok))


def first_untrusted(path: Path, me: int) -> str | None:
    """The first entry on the way that the app would refuse, in words;
    None when every entry that exists passes. The path and the one it
    resolves to are both walked, as file_trust does."""
    entries: list[Path] = []
    for walk in (path, Path(os.path.realpath(path))):
        for entry in (walk, *walk.parents):
            if entry not in entries:
                entries.append(entry)
    for entry in entries:
        try:
            meta = os.lstat(entry)
        except FileNotFoundError:
            continue
        if meta.st_uid not in (0, me):
            return f"{entry} belongs to another user"
        if not entry_trusted(meta.st_uid, meta.st_gid, meta.st_mode, stat.S_ISLNK(meta.st_mode), me):
            return f"other users can change {entry}"
    return None


def selftest() -> int:
    me = 501
    assert entry_trusted(0, 0, 0o755, False, me)
    assert entry_trusted(me, 20, 0o700, False, me)
    assert not entry_trusted(502, 20, 0o755, False, me), "another user's"
    assert not entry_trusted(0, 20, 0o775, False, me), "staff can write"
    assert not entry_trusted(0, 0, 0o1777, False, me), "a shared /tmp"
    assert entry_trusted(me, 80, 0o775, False, me), "the admins' group may write"
    assert entry_trusted(0, 20, 0o777, True, me), "a link's own mode means nothing"
    assert default_dir({"HOME": "/Users/someone"}) == Path("/Users/someone/Dev/AIJuggler-shared-target/lab-dev")
    assert default_dir({"HOME": "/x", "AIJ_LAB_DEV_DIR": "/y/lab"}) == Path("/y/lab")
    real_me = os.getuid()
    # The old place (RW1 V1): /tmp is everyone's, so a copy there is refused.
    if Path("/tmp").exists() and os.stat("/tmp").st_mode & 0o002:
        why = first_untrusted(Path("/tmp/aij-lab-dev") / APP / "Contents/MacOS/juggler-agent", real_me)
        assert why and "other users can change" in why, why
    # A folder of one's own, closed to others, passes when the way to it does.
    with tempfile.TemporaryDirectory(dir=os.path.expanduser("~")) as tmp:
        own = Path(tmp)
        own.chmod(0o700)
        if first_untrusted(own.parent, real_me) is None:
            assert first_untrusted(own / APP / "Contents/MacOS/juggler-agent", real_me) is None
            own.chmod(0o777)
            why = first_untrusted(own / APP, real_me)
            assert why == f"other users can change {own}", why
            own.chmod(0o700)
    print("lab-dev-place selftest: OK")
    return 0


def main(argv: list[str]) -> int:
    if argv == ["--selftest"]:
        return selftest()
    if argv == ["path"]:
        print(default_dir() / APP)
        return 0
    if len(argv) == 2 and argv[0] == "check":
        why = first_untrusted(Path(argv[1]), os.getuid())
        if why:
            print(f"lab-dev-place: {argv[1]}: {why}, so AI Juggler Dev won't run its own juggler-agent "
                  "there (AJ-E3013) and no session reports through hooks; keep the Dev copy in a folder "
                  f"of your own ({default_dir()})", file=sys.stderr)
            return 1
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
