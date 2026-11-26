#!/usr/bin/env python3
"""Trim a shared Rust build target safely: routine, not a rescue (the disk fell to 13 GB free on 2026-11-19).

    python3 scripts/trim_target.py <target dir>            dry run: lists what it would delete, and the GB
    python3 scripts/trim_target.py <target dir> --apply    deletes it
    python3 scripts/trim_target.py --free                  prints the free GB of the data volume
    python3 scripts/trim_target.py --selftest

What it deletes (the disk-hygiene rule, safe while a build runs elsewhere):
- in debug/deps and release/deps, files grouped by `<name>-<16-hex hash><ext>`: the newest of each group is kept, older
  copies untouched for more than 3 hours go (--min-age-hours to change);
- incremental session folders untouched for more than 30 minutes.

It applies only while it holds cargo's own build locks (`<profile>/.cargo-lock`, the lock every cargo build of that
target takes): a build already running makes it refuse, and a build that starts meanwhile waits until it's done. It
reads no other process's environment or command line (D-37). It never deletes the target itself, and never touches
anything outside its deps and incremental folders.

Run it after every merge, every RC build and every big agent build (just trim-target <dir>).
"""
import collections, contextlib, fcntl, os, re, shutil, sys, tempfile, time

ARTIFACT = re.compile(r"^(.+)-([0-9a-f]{16})(\..*)?$")
PROFILES = ("debug", "release")
BUILDERS = re.compile(r"(?:^|[\s/])(cargo|rustc|clippy-driver|cargo-nextest|preflight\.sh)(?:\s|$)")


def plan(target, min_age_s=3 * 3600, inc_age_s=1800, now=None):
    """-> (stale artifact paths with sizes, stale incremental dirs with sizes)"""
    now = time.time() if now is None else now
    stale = []
    for prof in PROFILES:
        deps = os.path.join(target, prof, "deps")
        if not os.path.isdir(deps):
            continue
        groups = collections.defaultdict(list)
        for name in os.listdir(deps):
            m = ARTIFACT.match(name)
            if not m:
                continue
            path = os.path.join(deps, name)
            try:
                st = os.lstat(path)
            except FileNotFoundError:                                       # a running build's temporary file, gone
                continue
            groups[(m[1], m[3] or "")].append((st.st_mtime, path, st.st_size))
        for copies in groups.values():
            copies.sort(reverse=True)
            stale += [(path, size) for mtime, path, size in copies[1:] if now - mtime > min_age_s]
    incremental = []
    for prof in PROFILES:
        inc = os.path.join(target, prof, "incremental")
        if not os.path.isdir(inc):
            continue
        for name in os.listdir(inc):
            path = os.path.join(inc, name)
            if now - os.lstat(path).st_mtime > inc_age_s:
                size = sum(os.lstat(os.path.join(r, f)).st_size for r, _, fs in os.walk(path) for f in fs)
                incremental.append((path, size))
    return stale, incremental


@contextlib.contextmanager
def build_locks(target):
    """Holds cargo's build lock of each profile in `target`; yields the profiles a running build holds (empty: all held)."""
    held, busy = [], []
    try:
        for prof in PROFILES:
            lock = os.path.join(target, prof, ".cargo-lock")
            if not os.path.exists(lock):
                continue
            f = open(lock, "a")
            try:
                fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
                held.append(f)
            except BlockingIOError:
                f.close()
                busy.append(prof)
        yield busy
    finally:
        for f in held:
            fcntl.flock(f, fcntl.LOCK_UN)
            f.close()


def free_gb(path="/System/Volumes/Data"):
    path = path if os.path.exists(path) else "/"
    return shutil.disk_usage(path).free / 1e9


def delete(items):
    for path, _ in items:
        if os.path.isdir(path) and not os.path.islink(path):
            shutil.rmtree(path)
        elif os.path.lexists(path):
            os.remove(path)


def selftest():
    now = time.time()
    with tempfile.TemporaryDirectory() as t:
        deps = os.path.join(t, "debug", "deps"); os.makedirs(deps)
        inc = os.path.join(t, "debug", "incremental", "old-sess"); os.makedirs(inc)
        def touch(name, age, size=10):
            p = os.path.join(deps, name)
            with open(p, "wb") as f:
                f.write(b"x" * size)
            os.utime(p, (now - age, now - age))
            return p
        newest = touch("libfoo-0123456789abcdef.rlib", 5 * 3600)          # newest of its group: kept, though old
        old = touch("libfoo-0123456789abcdef.rlib.1", 1)                    # no hash match form: never touched
        a = touch("libbar-aaaaaaaaaaaaaaaa.rlib", 10)                       # newest bar: kept
        b = touch("libbar-bbbbbbbbbbbbbbbb.rlib", 4 * 3600)                 # older bar copy, > 3 h: goes
        c = touch("libbar-cccccccccccccccc.rlib", 3600)                     # older bar copy, < 3 h: kept
        d = touch("libbar-dddddddddddddddd.d", 4 * 3600)                    # other extension, own group: kept
        os.utime(inc, (now - 3600, now - 3600))
        stale, incremental = plan(t, now=now)
        paths = {p for p, _ in stale}
        assert paths == {b}, paths
        assert [p for p, _ in incremental] == [inc], incremental
        assert newest not in paths and old not in paths and a not in paths and c not in paths and d not in paths
        delete(stale + incremental)
        assert not os.path.exists(b) and not os.path.exists(inc) and os.path.exists(a) and os.path.exists(c)
    with tempfile.TemporaryDirectory() as t:
        os.makedirs(os.path.join(t, "debug"))
        lock = os.path.join(t, "debug", ".cargo-lock")
        open(lock, "w").close()
        with build_locks(t) as busy:
            assert busy == [], busy                                         # free: held by us
        other = open(lock, "a")                                             # a running build holds it
        fcntl.flock(other, fcntl.LOCK_EX | fcntl.LOCK_NB)
        try:
            with build_locks(t) as busy:
                assert busy == ["debug"], busy
        finally:
            fcntl.flock(other, fcntl.LOCK_UN); other.close()
    assert free_gb() > 0
    with tempfile.TemporaryDirectory() as t:
        assert main([t, "--min-age-hours", "1"]) == 0, "the hours' value is not a second target"
    print("trim-target selftest: OK")


def main(argv):
    if "--selftest" in argv:
        selftest(); return 0
    if "--free" in argv:
        print(f"{free_gb():.0f}"); return 0
    hours = 3.0
    rest = list(argv)
    if "--min-age-hours" in rest:
        i = rest.index("--min-age-hours")
        hours = float(rest[i + 1])
        del rest[i:i + 2]
    args = [a for a in rest if not a.startswith("--")]
    if len(args) != 1 or not os.path.isdir(args[0]):
        print(__doc__.split("\n\n")[1]); return 2
    target = args[0]
    name = os.path.basename(os.path.normpath(target))
    apply = "--apply" in argv
    with build_locks(target) if apply else contextlib.nullcontext([]) as busy:
        if busy:
            print(f"trim-target: refused, a build is using {name} ({', '.join(busy)}); run it again once that's done")
            return 1
        stale, incremental = plan(target, min_age_s=hours * 3600)          # planned under the locks when applying
        gb = (sum(s for _, s in stale) + sum(s for _, s in incremental)) / 1e9
        print(f"trim-target {name}: {len(stale)} older artifact copies, {len(incremental)} old incremental dirs,"
              f" {gb:.1f} GB ({'deleting' if apply else 'dry run; --apply to delete'}); {free_gb():.0f} GB free")
        if not apply:
            return 0
        delete(stale + incremental)
    print(f"trim-target {name}: freed {gb:.1f} GB; {free_gb():.0f} GB free now")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
