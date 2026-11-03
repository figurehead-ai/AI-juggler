#!/usr/bin/env python3
"""Does it run, and what does it cost relative to each other, on a GitHub runner (Windows or Linux).

The runners have no GPU, so both options render in software there (WebView2/WebKitGTK compositing in
software; wgpu on WARP or lavapipe). The numbers compare the options with each other on the same
machine; they are not what a laptop with a GPU would see.

For each configuration it launches the spike, waits for it to settle, then reads over a fixed window:
  - the CPU time and memory of every process that appeared after launch (the app, and WebView2's or
    WebKitGTK's helper processes, which a sandbox can re-parent away from the app), and
  - the whole machine's busy CPU time, since the runner is otherwise idle (this also catches the
    X server or compositor doing work on the app's behalf).
Needs psutil.
"""
import json, os, subprocess, sys, threading, time
import psutil

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(HERE, '..', 'target', 'release', 'floor-spike' + ('.exe' if os.name == 'nt' else ''))
SETTLE, WINDOW = 9.0, 20.0


def new_processes(before):
    out = []
    for p in psutil.process_iter():
        if p.pid not in before:
            out.append(p)
    return out


def busy():
    t = psutil.cpu_times()
    idle = t.idle + getattr(t, 'iowait', 0.0)
    return sum(t) - idle


def cpu_and_mem(procs):
    cpu = mem = 0.0
    for p in procs:
        try:
            t = p.cpu_times()
            cpu += t.user + t.system
            mem += p.memory_info().rss
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            pass
    return cpu, mem / 1048576


def run(kind, anim):
    before = {p.pid for p in psutil.process_iter()}
    args = [BIN, '--anim', str(anim), '--exit-after', str(SETTLE + WINDOW + 4)] + (['--open', kind] if kind != 'none' else [])
    proc = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    events, other = [], []
    def reader():
        for line in proc.stdout:
            try:
                events.append(json.loads(line))
            except ValueError:
                other.append(line.rstrip())
    th = threading.Thread(target=reader, daemon=True)
    th.start()
    time.sleep(SETTLE)
    procs = new_processes(before)
    names = sorted({p.name() for p in procs if p.is_running()})
    c0, m0 = cpu_and_mem(procs)
    b0, t0 = busy(), time.time()
    time.sleep(WINDOW)
    c1, m1 = cpu_and_mem(procs)
    b1, secs = busy(), time.time() - t0
    try:
        proc.wait(timeout=60)
    except subprocess.TimeoutExpired:
        proc.kill()
    th.join(timeout=5)
    stats = next((e for e in events if e.get('event') == 'frame_stats'), {})
    return {
        'kind': kind, 'anim': anim, 'processes': names, 'cpu_pct': round((c1 - c0) / secs * 100, 1), 'machine_cpu_pct': round((b1 - b0) / secs * 100, 1), 'rss_mb': round(m1, 1),
        'first_frame_ms': next((round(e['ms']) for e in events if e.get('event') == 'first_frame'), None),
        'frames': stats.get('frames'), 'draw_ms_per_frame': round(stats['draw_ms'] / stats['frames'], 2) if stats.get('frames') else None,
        'adapter': next((e['info'] for e in events if e.get('event') == 'adapter'), None), 'exit_code': proc.returncode, 'log_tail': other[-6:],
    }


def main():
    out = {'os': sys.platform, 'runner': os.environ.get('RUNNER_OS'), 'runs': []}
    for kind, anim in [('none', 0), ('web', 0), ('web', 10), ('web', 30), ('native', 0), ('native', 10), ('native', 30)]:
        r = run(kind, anim)
        out['runs'].append(r)
        print(json.dumps(r), flush=True)
    json.dump(out, open(os.path.join(HERE, f'ci-{sys.platform}.json'), 'w'), indent=1)
    summary = os.environ.get('GITHUB_STEP_SUMMARY')
    if summary:
        with open(summary, 'a') as f:
            f.write(f'### Floor spike on {out["runner"]} (no GPU: software rendering)\n\n| Option | fps | CPU % of a core (its processes) | Whole machine, % of a core | Memory (RSS, MB) | First frame | Frames | Draw ms/frame | Exit |\n|---|---|---|---|---|---|---|---|---|\n')
            for r in out['runs']:
                f.write(f'| {r["kind"]} | {r["anim"]} | {r["cpu_pct"]} | {r["machine_cpu_pct"]} | {r["rss_mb"]} | {r["first_frame_ms"]} | {r["frames"]} | {r["draw_ms_per_frame"]} | {r["exit_code"]} |\n')
            adapters = {r['adapter'] for r in out['runs'] if r['adapter']}
            f.write('\n' + ''.join(f'- wgpu adapter: `{a}`\n' for a in adapters))


if __name__ == '__main__':
    main()
