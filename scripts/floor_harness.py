#!/usr/bin/env python3
"""AJ-E3513 req. 3: the Floor's budgets (SPEC §9.7), read on the release build.

  floor_harness.py build                 the release build with the harness
  floor_harness.py run --home H [--quick] runs it on the replica in H and
                                         writes docs/qa/floor/budgets-<date>.md
                                         (--quick: shorter cases, written to
                                         the scratch folder given by --out)
  floor_harness.py report RESULTS.json   writes the page from earlier results
  floor_harness.py --selftest

The harness is the `floor-harness` feature of juggler-app (never in a shipped
build): it opens the Floor itself, walks the cases and quits; this script
only starts it and judges the numbers. Run it with the cargo environment
of the worktree (CARGO_TARGET_DIR, RUSTFLAGS) and nothing else heavy on
the Mac. The replica's numbers only are written: no names.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "docs" / "qa" / "floor"
RUN_LIMIT_S = 900

# SPEC §9.7 Budgets: (what, budget in words, check on the results).
CPU_REST = 0.1  # % of one core the Floor adds, open and still
CPU_FULL = 2.0  # % of one core, animating Full
DRAW_MS = 1.5  # CPU a frame
FIRST_MS = 400
CHANGE_MS = 100
MEM_STILL = 150
MEM_ANIM = 250
BUILD_MS = 50
LEAK_MB = 15  # what three more opens and closes may add (the allocator's churn)


def metal(data: dict) -> float:
    """Metal's own working memory while animating ("Owned physical
    footprint (unmapped) (graphics)"), MB; 0 when not recorded."""
    return next((m["mb"] for m in data.get("memory_animating", [])
                 if m["category"] == "Owned physical footprint (unmapped) (graphics)"), 0.0)


def target_dir() -> Path:
    return Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))


def build() -> int:
    cmd = [
        "cargo", "tauri", "build", "--no-bundle",
        "--features", "floor-harness",
        "--config", "crates/juggler-app/tauri.dev.conf.json",
    ]
    print("floor-harness:", " ".join(cmd))
    return subprocess.call(["nice", "-n", "10", *cmd], cwd=ROOT)


def run(home: Path, quick: bool = False, out: Path | None = None) -> int:
    binary = target_dir() / "release" / "juggler-app"
    if not binary.exists():
        print(f"floor-harness: no release build at {binary}; run `build` first", file=sys.stderr)
        return 1
    if not (home / "Library").exists():
        print(f"floor-harness: no replica in {home}; run scripts/replica.py build --home {home}", file=sys.stderr)
        return 1
    with tempfile.TemporaryDirectory() as tmp:
        results = Path(tmp) / "results.json"
        env = {
            "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
            "HOME": str(home),
            "TMPDIR": tmp,
            "AIJ_FLOOR_HARNESS": str(results),
        }
        if quick:
            env["AIJ_FLOOR_HARNESS_QUICK"] = "1"
        print(f"floor-harness: running (about {4 if quick else 12} minutes) on the replica in {home}")
        try:
            log = (out or Path(tmp)) / "harness-app.log" if quick else Path(os.devnull)
            if quick and out:
                out.mkdir(parents=True, exist_ok=True)
            with open(log, "w") as sink:
                subprocess.run([str(binary)], env=env, timeout=RUN_LIMIT_S, check=False,
                               stdout=subprocess.DEVNULL, stderr=sink)
        except subprocess.TimeoutExpired:
            print("floor-harness: the run didn't finish in time", file=sys.stderr)
            return 1
        if not results.exists():
            print("floor-harness: the app wrote no results", file=sys.stderr)
            return 1
        data = json.loads(results.read_text())
    where = out if (quick and out) else OUT_DIR
    keep = where / f"budgets-{dt.date.today().isoformat()}{'-quick' if quick else ''}.json"
    where.mkdir(parents=True, exist_ok=True)
    data["machine"] = machine()
    keep.write_text(json.dumps(data, indent=2) + "\n")
    return report(keep)


def machine() -> dict:
    def sysctl(name: str) -> str:
        return subprocess.run(["/usr/sbin/sysctl", "-n", name], capture_output=True, text=True).stdout.strip()

    mem = sysctl("hw.memsize")
    return {
        "model": sysctl("hw.model"),
        "chip": sysctl("machdep.cpu.brand_string"),
        "memory_gb": round(int(mem) / 2**30) if mem.isdigit() else None,
        "macos": platform.mac_ver()[0],
    }


def own(case: dict, base: float) -> float:
    """The Floor's own time (drawing, building scenes, the feed) as a share
    of one core; the whole app's change where the run didn't record it."""
    return case["floor_pct"] if "floor_pct" in case else case["cpu_pct"] - base


def verdicts(data: dict) -> list[tuple[str, str, str, bool]]:
    """(case, budget, measured, within) for each SPEC §9.7 row."""
    cases = {c["name"]: c for c in data["cases"]}
    base = cases["Never opened (the app alone)"]["cpu_pct"]
    rest = cases["Open, at rest (Motion Off)"]
    full = cases["Animating, Full (5 minutes)"]
    calm = cases["Animating, Calm"]
    covered = cases["Covered by another window"]
    mini = cases["Minimised"]
    closed = cases["Closed"]
    before = data["footprint_before_mb"]
    anim_peak = max([m["footprint_mb"] for m in data["full_memory_marks"]] + [full["footprint_mb"]])
    changes = [float("inf") if c is None else c for c in data["change_to_screen_ms"]] or [float("inf")]
    worst_change = max(changes)
    undrawn = sum(1 for c in data["change_to_screen_ms"] if c is None)
    rows = [
        ("Open, nothing moving", "0 frames after the first, but for data news",
         f"{rest['frames']} frames in {rest['secs']:.0f} s, {rest.get('snapshots', 0)} for data news",
         rest["frames"] - rest.get("snapshots", 0) <= 0),
        ("Open, nothing moving", f"< {CPU_REST}% of one core",
         f"{own(rest, base):.3f}% the Floor's own (the whole app {rest['cpu_pct'] - base:+.2f}% over a window alone)",
         own(rest, base) < CPU_REST),
        ("Animating, Full", f"≤ {CPU_FULL}% of one core",
         f"{own(full, base):.2f}% the Floor's own (the whole app {full['cpu_pct'] - base:+.2f}% over)", own(full, base) <= CPU_FULL),
        ("Animating, Full", f"≤ {DRAW_MS} ms of CPU a frame",
         f"{full['draw_ms_per_frame']:.2f} ms ({full['frames']} frames)", full["draw_ms_per_frame"] <= DRAW_MS),
        ("Animating, Calm", f"≤ {CPU_FULL}% of one core",
         f"{own(calm, base):.2f}% the Floor's own ({calm['frames']} frames)", own(calm, base) <= CPU_FULL),
        ("Covered", "0 frames within 1 s",
         f"{covered['frames'] - covered['first_second_frames']} after the first second "
         f"({covered['first_second_frames']} in it)", covered["frames"] - covered["first_second_frames"] == 0),
        ("Minimised", "0 frames within 1 s",
         f"{mini['frames'] - mini['first_second_frames']} after the first second "
         f"({mini['first_second_frames']} in it)", mini["frames"] - mini["first_second_frames"] == 0),
        ("Closed", "0 frames", f"{closed['frames']}", closed["frames"] == 0),
        ("Opening", f"first frame ≤ {FIRST_MS} ms", f"{data['first_frame_ms']} ms",
         0 < data["first_frame_ms"] <= FIRST_MS),
        ("A data change", f"on screen ≤ {CHANGE_MS} ms",
         f"{undrawn} of {len(changes)} never drawn" if undrawn else f"worst {worst_change:.1f} ms of {len(changes)}",
         worst_change <= CHANGE_MS),
        ("Memory added, open and still", f"≤ {MEM_STILL} MB",
         f"{data['footprint_open_still_mb'] - before:+.0f} MB", data["footprint_open_still_mb"] - before <= MEM_STILL),
        ("Memory added, animating", f"≤ {MEM_ANIM} MB",
         f"{anim_peak - before:+.0f} MB at its highest"
         + (f", of which Metal's own working memory {metal(data):.0f} MB" if metal(data) else ""),
         anim_peak - before <= MEM_ANIM),
        ("Memory once closed", "0 added",
         f"{data['footprint_closed_mb'] - before:+.0f} MB (the system's allocator keeps freed blocks for reuse)",
         data["footprint_closed_mb"] - before <= 10),
    ]
    if data.get("footprint_after_each_more_mb"):
        each = data["footprint_after_each_more_mb"]
        # A leak grows every time; the allocator's cache settles. Judged on
        # the last three of six more opens and closes.
        late = each[-1] - each[-4] if len(each) >= 4 else each[-1] - each[0]
        rows.append(("Opened and closed 6 more times", f"the last 3 add ≤ {LEAK_MB} MB (no leak)",
                     " → ".join(f"{mb:.0f}" for mb in each) + f" MB ({late:+.0f} over the last 3)",
                     late <= LEAK_MB))
    elif "footprint_closed_after_3_more_mb" in data:
        grew = data["footprint_closed_after_3_more_mb"] - data["footprint_closed_mb"]
        rows.append(("Opened and closed 3 more times", f"grows ≤ {LEAK_MB} MB (no leak)",
                     f"{grew:+.0f} MB", grew <= LEAK_MB))
    rows += [
        ("Building the scene", f"≤ {BUILD_MS} ms", f"{data['scene_build_ms_mean']:.2f} ms on average",
         data["scene_build_ms_mean"] <= BUILD_MS),
    ]
    return rows


def report(results: Path) -> int:
    data = json.loads(results.read_text())
    rows = verdicts(data)
    m = data.get("machine", {})
    date = results.stem.removeprefix("budgets-")
    lines = [
        f"# The Floor's budgets, {date}",
        "",
        "Read by `scripts/floor_harness.py` on the release build with the harness",
        "(AJ-E3513 req. 3), on the replica company: "
        f"{data['apps']} Apps, {data['sessions']} sessions. Budgets: SPEC §9.7.",
        "",
        f"Machine: {m.get('model', '?')}, {m.get('chip', '?')}, {m.get('memory_gb', '?')} GB, macOS {m.get('macos', '?')}.",
        "CPU is the whole app's, less the app alone; memory is its footprint (the GPU's",
        "surfaces included) less before the Floor opened.",
        "",
        "| Case | Budget | Measured | |",
        "|---|---|---|---|",
    ]
    for case, budget, measured, ok in rows:
        lines.append(f"| {case} | {budget} | {measured} | {'within' if ok else '**over**'} |")
    lines += ["", "## Every case", "", "| Case | Seconds | CPU (% of one core) | Frames | In the first second | Drawing a frame | Footprint |",
              "|---|---|---|---|---|---|---|"]
    for c in data["cases"]:
        lines.append(f"| {c['name']} | {c['secs']:.0f} | {c['cpu_pct']:.3f} | {c['frames']} | "
                     f"{c['first_second_frames']} | {c['draw_ms_per_frame']:.2f} ms | {c['footprint_mb']:.0f} MB |")
    marks = ", ".join(f"{m_['at_s']} s: {m_['footprint_mb']:.0f} MB" for m_ in data["full_memory_marks"])
    lines += ["", f"Memory while animating Full: {marks} (before opening: {data['footprint_before_mb']:.0f} MB).",
              f"A data change on screen, 10 tries: {', '.join(str(x) for x in data['change_to_screen_ms'])} ms.", ""]
    if data.get("open_steps"):
        lines += ["Opening, step by step (ms after it was asked for): "
                  + ", ".join(f"{st['step']} {st['at_ms']}" for st in data["open_steps"]) + ".", ""]
    if "footprint_closed_after_3_more_mb" in data:
        lines += [f"After three more opens and closes: {data['footprint_closed_after_3_more_mb']:.0f} MB "
                  f"(after the first close: {data['footprint_closed_mb']:.0f} MB).", ""]
    for key, words in (("memory_before", "before opening"), ("memory_still", "open and still"), ("memory_animating", "animating"), ("memory_closed", "closed"), ("memory_closed_after_3_more", "closed after three more opens")):
        if data.get(key):
            lines.append(f"Memory {words}, biggest parts: "
                         + ", ".join(f"{m_['category']} {m_['mb']:.0f} MB" for m_ in data[key][:6]) + ".")
    if "cases" in data and "visible_answer" in data["cases"][0]:
        seen = {0: "no answer yet", 1: "seen", 2: "not seen"}
        lines += ["", "What the window said at the end of each case: "
                  + "; ".join(f"{c['name']}: {seen.get(c['visible_answer'], '?')}"
                              f"{', minimised' if c.get('minimised_now') else ''}" for c in data["cases"]) + ".", ""]
    page = results.with_suffix(".md")
    page.write_text("\n".join(lines))
    over = [r for r in rows if not r[3]]
    shown = page.resolve().relative_to(ROOT) if page.resolve().is_relative_to(ROOT) else page
    print(f"floor-harness: {shown}: {len(rows) - len(over)} within, {len(over)} over")
    for case, budget, measured, _ in over:
        print(f"  over: {case}: {measured} (budget {budget})")
    return 0


def selftest() -> int:
    sample = {
        "cases": [
            {"name": n, "secs": 60, "cpu_pct": cpu, "frames": f, "first_second_frames": f1,
             "draw_ms_per_frame": 0.7, "footprint_mb": 300}
            for n, cpu, f, f1 in [
                ("Never opened (the app alone)", 0.5, 0, 0), ("Open, at rest (Motion Off)", 0.55, 0, 0),
                ("Animating, Calm", 1.0, 240, 4), ("Animating, Full (5 minutes)", 1.5, 2400, 8),
                ("Covered by another window", 0.5, 1, 1), ("Minimised", 0.5, 0, 0), ("Closed", 0.5, 0, 0)]
        ],
        "apps": 7, "sessions": 30, "first_frame_ms": 120, "footprint_before_mb": 200,
        "footprint_open_still_mb": 300, "footprint_closed_mb": 205,
        "full_memory_marks": [{"at_s": 30, "footprint_mb": 380}], "change_to_screen_ms": [12.0, 30.5],
        "scene_build_ms_mean": 9.0,
    }
    rows = verdicts(sample)
    assert all(ok for *_, ok in rows), [r for r in rows if not r[3]]
    sample["cases"][4]["frames"] = 5
    assert not verdicts(sample)[5][3], "frames after the first second while covered are over"
    sample["change_to_screen_ms"] = [12.0, None]
    assert not verdicts(sample)[9][3], "a change never drawn is over"
    print("floor-harness selftest: OK")
    return 0


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("build")
    r = sub.add_parser("run")
    r.add_argument("--home", type=Path, required=True)
    r.add_argument("--quick", action="store_true")
    r.add_argument("--out", type=Path)
    p = sub.add_parser("report")
    p.add_argument("results", type=Path)
    args = parser.parse_args()
    if args.cmd == "build":
        return build()
    if args.cmd == "run":
        return run(args.home, args.quick, args.out)
    return report(args.results)


if __name__ == "__main__":
    sys.exit(main())
