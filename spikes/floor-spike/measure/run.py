#!/usr/bin/env python3
"""Measures the two Floor spikes on this Mac, one fresh launch per configuration.

For every run it finds the processes the app brings with it: the app itself and the WebKit processes
(WebContent, GPU, Networking) that macOS starts for its webviews through launchd, so they are not its
children. It then reads, over the same window of time:
  - CPU: cumulative CPU time from `ps` (all those processes), as percent of one core;
  - energy: the POWER column of `top` (second sample, averaged over the window);
  - memory: `footprint` (physical footprint) per process, and RSS from `ps`;
  - GPU: "Device Utilization %" from `ioreg` (system-wide; the app-closed baseline is reported too).
No sudo, nothing outside this app's own processes is touched, and every run ends with the app quitting
itself (--exit-after); the script checks nothing is left running.

Usage: python3 measure/run.py [--quick] > measure/out/results.json
"""
import json, os, re, subprocess, sys, time, statistics, threading

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(HERE, '..', 'target', 'release', 'floor-spike')
WINDOW = 20.0      # seconds measured per configuration
SETTLE = 7.0       # seconds after launch before measuring
QUICK = '--quick' in sys.argv


def ps_rows():
    out = subprocess.run(['ps', '-A', '-o', 'pid=,ppid=,time=,rss=,comm='], capture_output=True, text=True).stdout
    rows = []
    for line in out.strip().splitlines():
        p = line.split(None, 4)
        if len(p) == 5:
            rows.append({'pid': int(p[0]), 'ppid': int(p[1]), 'time': p[2], 'rss': int(p[3]), 'comm': p[4]})
    return rows


def secs(t):
    parts = [float(x) for x in t.split(':')]
    v = 0.0
    for x in parts:
        v = v * 60 + x
    return v


def webkit_pids(rows):
    return {r['pid'] for r in rows if 'com.apple.WebKit.' in r['comm']}


def gpu_util():
    out = subprocess.run(['ioreg', '-r', '-d', '1', '-w', '0', '-c', 'IOAccelerator'], capture_output=True, text=True).stdout
    m = re.search(r'"Device Utilization %"=(\d+)', out)
    return int(m.group(1)) if m else None


def footprint_mb(pids):
    res = {}
    for pid in pids:
        out = subprocess.run(['footprint', '-p', str(pid)], capture_output=True, text=True).stdout
        m = re.search(r'Footprint:\s*([\d.]+)\s*([KMG]B)', out)
        if m:
            v = float(m.group(1)) * {'KB': 1 / 1024, 'MB': 1, 'GB': 1024}[m.group(2)]
            res[pid] = round(v, 1)
    return res


def top_power(pids, seconds):
    out = subprocess.run(['top', '-l', '2', '-s', str(int(seconds)), '-stats', 'pid,cpu,power'], capture_output=True, text=True).stdout
    sample = out.split('PID')[-1]  # the second sample covers the whole interval
    cpu = power = 0.0
    for line in sample.splitlines()[1:]:
        p = line.split()
        if len(p) >= 3 and p[0].isdigit() and int(p[0]) in pids:
            try:
                cpu += float(p[1]); power += float(p[2])
            except ValueError:
                pass
    return round(cpu, 2), round(power, 2)


def run(kind, anim, extra=(), window=WINDOW, settle=SETTLE, label=None, long_samples=None):
    before = webkit_pids(ps_rows())
    args = [BIN, '--anim', str(anim), '--on-top'] + (['--open', kind] if kind != 'none' else []) + list(extra)
    total = settle + window + 3
    args += ['--exit-after', str(total)]
    proc = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    events = []
    def reader():
        for line in proc.stdout:
            try:
                events.append(json.loads(line))
            except ValueError:
                pass
    th = threading.Thread(target=reader, daemon=True); th.start()
    time.sleep(settle)
    rows = ps_rows()
    pids = {proc.pid} | (webkit_pids(rows) - before)
    names = {r['pid']: r['comm'].split('/')[-1] for r in rows if r['pid'] in pids}
    mem_start = footprint_mb(pids)
    rss_start = {r['pid']: r['rss'] for r in rows if r['pid'] in pids}
    t_start = {r['pid']: secs(r['time']) for r in rows if r['pid'] in pids}
    gpu = []
    stop = threading.Event()
    def gpu_sampler():
        while not stop.is_set():
            g = gpu_util()
            if g is not None:
                gpu.append(g)
            stop.wait(1.0)
    gt = threading.Thread(target=gpu_sampler, daemon=True); gt.start()
    w0 = time.time()
    mem_mid = None
    if long_samples:
        # long run: CPU by top over the whole window would be too coarse; sample memory at marks.
        marks = {}
        for mark in long_samples:
            time.sleep(max(0, w0 + mark - time.time()))
            marks[mark] = footprint_mb(pids)
        mem_mid = marks
        top_cpu, top_pow = top_power(pids, 10)
    else:
        top_cpu, top_pow = top_power(pids, window)
    elapsed = time.time() - w0
    rows2 = ps_rows()
    t_end = {r['pid']: secs(r['time']) for r in rows2 if r['pid'] in pids}
    mem_end = footprint_mb(pids)
    stop.set()
    by_proc = {f'{names.get(p, "?")}#{p}': round((t_end.get(p, t_start.get(p, 0)) - t_start.get(p, 0)) / elapsed * 100, 2) for p in pids}
    cpu = sum(by_proc.values())
    proc.wait(timeout=total + 30)
    th.join(timeout=2)
    first = next((e['ms'] for e in events if e.get('event') == 'first_frame'), None)
    stats = [e for e in events if e.get('event') == 'frame_stats']
    return {
        'label': label or f'{kind} anim={anim} {" ".join(extra)}'.strip(),
        'kind': kind, 'anim': anim, 'extra': list(extra), 'seconds': round(elapsed, 1),
        'processes': {str(p): names.get(p, '?') for p in sorted(pids)},
        'cpu_pct': round(cpu, 2), 'cpu_by_process': by_proc, 'top_cpu_pct': top_cpu, 'top_power': top_pow,
        'mem_mb_start': mem_start, 'mem_mb_end': mem_end, 'mem_total_start': round(sum(mem_start.values()), 1), 'mem_total_end': round(sum(mem_end.values()), 1),
        'rss_mb_start': round(sum(rss_start.values()) / 1024, 1), 'mem_marks': mem_mid,
        'gpu_util_avg': round(statistics.mean(gpu), 1) if gpu else None, 'first_frame_ms': first, 'frame_stats': stats,
    }


def leftovers():
    out = subprocess.run(['pgrep', '-fl', 'floor-spike'], capture_output=True, text=True).stdout.strip()
    return out


def main():
    results = {'when': time.strftime('%Y-%m-%d %H:%M'), 'window_s': WINDOW, 'runs': []}
    gpu_idle = [gpu_util() for _ in range(5) if not time.sleep(1)]
    results['gpu_idle_baseline'] = round(statistics.mean([g for g in gpu_idle if g is not None]), 1)
    plan = [('none', 0, ())]
    if '--followup' in sys.argv:
        # an opaque cover, per-process CPU, and the native window drawing at the art's own size
        plan = [('web', 10, ('--cover-after', '4')), ('native', 10, ('--cover-after', '4')), ('native', 10, ('--minimize-after', '4')),
                ('native', 0, ('--art-surface',)), ('native', 10, ('--art-surface',)), ('native', 30, ('--art-surface',)),
                ('native', 10, ('--art-surface', '--cover-after', '4')), ('native', 10, ('--art-surface', '--minimize-after', '4'))]
    else:
        for kind in ('web', 'native'):
            plan += [(kind, 0, ()), (kind, 10, ()), (kind, 30, ()), (kind, 10, ('--cover-after', '4')), (kind, 10, ('--minimize-after', '4'))]
    for kind, anim, extra in plan:
        r = run(kind, anim, extra)
        results['runs'].append(r)
        print(json.dumps({k: r[k] for k in ('label', 'cpu_pct', 'top_power', 'mem_total_start', 'gpu_util_avg', 'first_frame_ms')}), file=sys.stderr, flush=True)
        time.sleep(2)
    if not QUICK and '--followup' not in sys.argv:
        for kind in ('web', 'native'):
            r = run(kind, 10, (), window=600, label=f'{kind} anim=10, 10 minutes', long_samples=[60, 300, 590])
            results['runs'].append(r)
            print(json.dumps({k: r[k] for k in ('label', 'cpu_pct', 'mem_total_start', 'mem_marks')}), file=sys.stderr, flush=True)
            time.sleep(2)
    results['leftovers'] = leftovers()
    print(json.dumps(results, indent=1))


if __name__ == '__main__':
    main()
