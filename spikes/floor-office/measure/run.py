#!/usr/bin/env python3
"""Measures the Floor prototype on this Mac, one fresh launch per configuration (as the spike did).
CPU: cumulative CPU time from `ps` for the app and the WebKit processes its main window brings
(launchd starts those, so they are found as new processes), as % of one core. Energy: `top` POWER.
Memory: `footprint`. Usage: python3 measure/run.py > measure/out/results.json"""
import json, os, re, subprocess, sys, threading, time

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(HERE, '..', 'target', 'release', 'floor-office')
SETTLE, WINDOW = 7.0, 20.0

def rows():
    out = subprocess.run(['ps', '-A', '-o', 'pid=,time=,rss=,comm='], capture_output=True, text=True).stdout
    return [dict(zip(('pid', 'time', 'rss', 'comm'), (int(p[0]), p[1], int(p[2]), p[3]))) for p in (l.split(None, 3) for l in out.strip().splitlines()) if len(p) == 4]

def secs(t):
    v = 0.0
    for x in t.split(':'):
        v = v * 60 + float(x)
    return v

def footprint(pid):
    out = subprocess.run(['footprint', '-p', str(pid)], capture_output=True, text=True).stdout
    m = re.search(r'Footprint:\s*([\d.]+)\s*([KMG]B)', out)
    return round(float(m.group(1)) * {'KB': 1 / 1024, 'MB': 1, 'GB': 1024}[m.group(2)], 1) if m else None

def top_power(pids, seconds):
    out = subprocess.run(['top', '-l', '2', '-s', str(int(seconds)), '-stats', 'pid,cpu,power'], capture_output=True, text=True).stdout
    p = 0.0
    for line in out.split('PID')[-1].splitlines()[1:]:
        f = line.split()
        if len(f) >= 3 and f[0].isdigit() and int(f[0]) in pids:
            try: p += float(f[2])
            except ValueError: pass
    return round(p, 2)

def run(label, extra, window=WINDOW, marks=None):
    before = {r['pid'] for r in rows() if 'com.apple.WebKit.' in r['comm']}
    total = SETTLE + window + 3
    proc = subprocess.Popen([BIN, '--on-top', '--exit-after', str(total)] + extra, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    events = []
    th = threading.Thread(target=lambda: [events.append(json.loads(l)) for l in proc.stdout if l.startswith('{')], daemon=True); th.start()
    time.sleep(SETTLE)
    rs = rows()
    pids = {proc.pid} | ({r['pid'] for r in rs if 'com.apple.WebKit.' in r['comm']} - before)
    names = {r['pid']: r['comm'].split('/')[-1].replace('com.apple.WebKit.', 'WebKit ') for r in rs if r['pid'] in pids}
    t0 = {r['pid']: secs(r['time']) for r in rs if r['pid'] in pids}
    mem0 = {names[p]: footprint(p) for p in pids}
    w0 = time.time()
    mem_marks = {}
    if marks:
        for m in marks:
            time.sleep(max(0, w0 + m - time.time()))
            mem_marks[m] = footprint(proc.pid)
        power = top_power(pids, 5)
    else:
        power = top_power(pids, window)
    el = time.time() - w0
    t1 = {r['pid']: secs(r['time']) for r in rows() if r['pid'] in pids}
    by = {f"{names[p]}#{p}": round((t1.get(p, t0[p]) - t0[p]) / el * 100, 2) for p in pids}
    proc.wait(timeout=total + 30)
    st = next((e for e in events if e.get('event') == 'frame_stats'), {})
    return {'label': label, 'cpu_pct': round(sum(by.values()), 2), 'cpu_by_process': by, 'power': power, 'memory_mb': mem0,
            'memory_total_mb': round(sum(v for v in mem0.values() if v), 1), 'app_memory_marks_mb': mem_marks,
            'first_frame_ms': next((round(e['ms']) for e in events if e.get('event') == 'first_frame'), None),
            'frames': st.get('frames'), 'draw_ms_per_frame': round(st['draw_ms'] / st['frames'], 3) if st.get('frames') else None,
            'world': next(({k: e[k] for k in ('ms', 'canvas', 'atlas', 'sprites', 'items', 'actors', 'walkers')} for e in events if e.get('event') == 'world_built'), None)}

def main():
    out = {'when': time.strftime('%Y-%m-%d %H:%M'), 'runs': []}
    plan = [('At rest (nothing moving)', ['--anim', '0']), ('Animating, 8 fps', ['--anim', '8']), ('Animating, 30 fps', ['--anim', '30']),
            ('Animating, fully covered', ['--anim', '8', '--cover-after', '4']), ('Animating, minimised', ['--anim', '8', '--minimize-after', '4'])]
    for label, extra in plan:
        r = run(label, extra)
        out['runs'].append(r)
        print(json.dumps({k: r[k] for k in ('label', 'cpu_pct', 'power', 'memory_total_mb', 'first_frame_ms', 'frames')}), file=sys.stderr, flush=True)
        time.sleep(2)
    if '--quick' not in sys.argv:
        r = run('Animating 8 fps for 5 minutes', ['--anim', '8'], window=300, marks=[30, 150, 290])
        out['runs'].append(r)
        print(json.dumps({k: r[k] for k in ('label', 'cpu_pct', 'app_memory_marks_mb')}), file=sys.stderr, flush=True)
    left = subprocess.run(['pgrep', '-x', 'floor-office'], capture_output=True, text=True).stdout.strip()
    out['left_running'] = left or 'none'
    print(json.dumps(out, indent=1))

if __name__ == '__main__':
    main()
