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

It refuses to apply while `ps -E` shows a cargo, rustc, clippy-driver or preflight whose environment names that target
(CARGO_TARGET_DIR or AIJ_PREFLIGHT_TARGET): a build that's using it could need an "older" copy back. It never deletes
the target itself, and never touches anything outside its deps and incremental folders.

Run it after every merge, every RC build and every big agent build (just trim-target <dir>).
"""
import collections, os, re, shutil, subprocess, sys, tempfile, time

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
            st = os.lstat(path)
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


def uses_target(line, real):
    """True when one `ps -Eww` line is a build whose environment names the target `real` (a realpath)."""
    if not BUILDERS.search(line.split(" CARGO_")[0].split(" AIJ_")[0]):
        return False
    for var in ("CARGO_TARGET_DIR=", "AIJ_PREFLIGHT_TARGET="):
        i = line.find(var)
        if i >= 0 and os.path.realpath(line[i + len(var):].split(" ")[0]) == real:
            return True
    return False


def builders_using(target):
    """The commands of running builds whose environment names this target (ps -E shows the environment)."""
    out = subprocess.run(["ps", "-Eww", "-o", "pid=,command="], capture_output=True, text=True).stdout
    real = os.path.realpath(target)
    return [line[:160] for line in out.splitlines() if uses_target(line, real)]


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
    real = os.path.realpath("/tmp/aij-t")
    assert uses_target("123 cargo nextest run PATH=/x CARGO_TARGET_DIR=/tmp/aij-t HOME=/h", real)
    assert uses_target("123 /opt/bin/rustc --crate-name x AIJ_PREFLIGHT_TARGET=/tmp/aij-t", real)
    assert not uses_target("123 cargo build CARGO_TARGET_DIR=/tmp/other", real)
    assert not uses_target("123 vim notes.txt CARGO_TARGET_DIR=/tmp/aij-t", real)
    assert free_gb() > 0
    print("trim-target selftest: OK")


def main(argv):
    if "--selftest" in argv:
        selftest(); return 0
    if "--free" in argv:
        print(f"{free_gb():.0f}"); return 0
    args = [a for a in argv if not a.startswith("--")]
    if len(args) != 1 or not os.path.isdir(args[0]):
        print(__doc__.split("\n\n")[1]); return 2
    target = args[0]
    hours = 3.0
    if "--min-age-hours" in argv:
        hours = float(argv[argv.index("--min-age-hours") + 1])
    stale, incremental = plan(target, min_age_s=hours * 3600)
    gb = (sum(s for _, s in stale) + sum(s for _, s in incremental)) / 1e9
    name = os.path.basename(os.path.normpath(target))
    print(f"trim-target {name}: {len(stale)} older artifact copies, {len(incremental)} old incremental dirs, {gb:.1f} GB"
          f" ({'deleting' if '--apply' in argv else 'dry run; --apply to delete'}); {free_gb():.0f} GB free")
    if "--apply" not in argv:
        return 0
    busy = builders_using(target)
    if busy:
        print(f"trim-target: refused, a build is using {name}:")
        for line in busy[:5]:
            print("   ", line)
        return 1
    delete(stale + incremental)
    print(f"trim-target {name}: freed {gb:.1f} GB; {free_gb():.0f} GB free now")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
