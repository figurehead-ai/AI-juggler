#!/usr/bin/env python3
"""Does it run, and what does it cost relative to each other, on a GitHub runner (Windows or Linux).

The runners have no GPU, so both options render in software there (WebView2/WebKitGTK compositing in
software; wgpu on WARP or lavapipe). The numbers compare the options with each other on the same
machine; they are not what a laptop with a GPU would see.

For each configuration it launches the spike, waits for it to settle, then reads the CPU time and memory
of the app and every process under it (WebView2's msedgewebview2 processes on Windows, WebKitGTK's
WebKitWebProcess and network process on Linux) over a fixed window. Needs psutil.
"""
import json, os, subprocess, sys, threading, time
import psutil

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(HERE, '..', 'target', 'release', 'floor-spike' + ('.exe' if os.name == 'nt' else ''))
SETTLE, WINDOW = 9.0, 20.0


def tree(pid):
    try:
        p = psutil.Process(pid)
        return [p] + p.children(recursive=True)
    except psutil.NoSuchProcess:
        return []


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
    procs = tree(proc.pid)
    names = sorted({p.name() for p in procs if p.is_running()})
    c0, m0 = cpu_and_mem(procs)
    t0 = time.time()
    time.sleep(WINDOW)
    procs = {p.pid: p for p in procs + tree(proc.pid)}.values()
    c1, m1 = cpu_and_mem(procs)
    secs = time.time() - t0
    try:
        proc.wait(timeout=60)
    except subprocess.TimeoutExpired:
        for p in tree(proc.pid):
            p.kill()
    th.join(timeout=5)
    stats = next((e for e in events if e.get('event') == 'frame_stats'), {})
    return {
        'kind': kind, 'anim': anim, 'processes': names, 'cpu_pct': round((c1 - c0) / secs * 100, 1), 'rss_mb': round(m1, 1),
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
            f.write(f'### Floor spike on {out["runner"]} (no GPU: software rendering)\n\n| Option | fps | CPU % of a core | Memory (RSS, MB) | First frame | Frames | Draw ms/frame | Exit |\n|---|---|---|---|---|---|---|---|\n')
            for r in out['runs']:
                f.write(f'| {r["kind"]} | {r["anim"]} | {r["cpu_pct"]} | {r["rss_mb"]} | {r["first_frame_ms"]} | {r["frames"]} | {r["draw_ms_per_frame"]} | {r["exit_code"]} |\n')
            adapters = {r['adapter'] for r in out['runs'] if r['adapter']}
            f.write('\n' + ''.join(f'- wgpu adapter: `{a}`\n' for a in adapters))


if __name__ == '__main__':
    main()
