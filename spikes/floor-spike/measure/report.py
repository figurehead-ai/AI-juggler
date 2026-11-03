#!/usr/bin/env python3
"""Builds the results page from the measured JSON and the captured frames.
Usage: python3 measure/report.py OUT.html CI_DIR [CI_DIR ...]
Every number on the page is read from measure/out/*.json or the runners' ci-*.json."""
import base64, glob, json, os, sys, html

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'out')
mac = json.load(open(os.path.join(OUT, 'results-mac.json')))
follow = json.load(open(os.path.join(OUT, 'results-mac-followup.json')))
covered = json.load(open(os.path.join(OUT, 'results-mac-covered.json')))
slow = json.load(open(os.path.join(OUT, 'results-mac-slowfps.json')))
runs = {r['label']: r for r in mac['runs'] + follow['runs'] + covered['runs'] + slow['runs']}
base = runs['none anim=0']


def r(label):
    return runs[label]


def frames(prefix):
    return [base64.b64encode(open(f, 'rb').read()).decode() for f in sorted(glob.glob(os.path.join(OUT, 'cap', f'{prefix}-[0-9].png')))]


def pct(x):
    return f'{x:.1f} %' if x is not None else '–'


def mb(x):
    return f'{x:.0f} MB'


def delta_mb(run):
    return run['mem_total_start'] - base['mem_total_start']


rows = [
    ('At rest (nothing moving)', 'web anim=0', 'native anim=0'),
    ('Animating, 6 fps', 'web anim=6', 'native anim=6'),
    ('Animating, 8 fps', 'web anim=8', 'native anim=8'),
    ('Animating, 10 fps', 'web anim=10', 'native anim=10'),
    ('Animating, 30 fps', 'web anim=30', 'native anim=30'),
    ('Animating 10 fps for 10 minutes', 'web anim=10, 10 minutes', 'native anim=10, 10 minutes'),
    ('Fully covered by another window <span class=muted>(our own full-screen window, whose processes are counted too)</span>', 'web anim=10, fully covered', 'native anim=10, fully covered'),
    ('Minimised', 'web anim=10 --minimize-after 4', 'native anim=10 --minimize-after 4'),
]


def cell(run, what):
    if what == 'cpu':
        return pct(run['cpu_pct'])
    if what == 'power':
        return f"{run['top_power']:.1f}"
    if what == 'mem':
        return f"{mb(run['mem_total_start'])} <span class=muted>(+{delta_mb(run):.0f})</span>"


mac_table = ''.join(
    f'<tr><th scope=row>{name}</th><td class=num>{cell(r(w), "cpu")}</td><td class=num><b>{cell(r(n), "cpu")}</b></td>'
    f'<td class=num>{cell(r(w), "power")}</td><td class=num><b>{cell(r(n), "power")}</b></td>'
    f'<td class=num>{cell(r(w), "mem")}</td><td class=num><b>{cell(r(n), "mem")}</b></td></tr>'
    for name, w, n in rows)

web10, nat10 = r('web anim=10, 10 minutes'), r('native anim=10, 10 minutes')
marks = lambda run: ' → '.join(f"{sum(v.values()):.0f}" for k, v in sorted(run['mem_marks'].items(), key=lambda kv: int(kv[0])))
ff = lambda kind: sorted(round(x['first_frame_ms']) for x in mac['runs'] if x['kind'] == kind and x['first_frame_ms'] and x['anim'])
procs = lambda run: ', '.join(sorted({v.replace('com.apple.WebKit.', 'WebKit ') for v in run['processes'].values()}))
by_proc = lambda run: ', '.join(f"{k.split('#')[0].replace('com.apple.WebKit.', 'WebKit ')} {v:.2f}" for k, v in sorted(run['cpu_by_process'].items(), key=lambda kv: -kv[1]) if v >= 0.05)

ci = []
for d in sys.argv[2:]:
    for f in sorted(glob.glob(os.path.join(d, '*', 'ci-*.json'))):
        ci.append((d, json.load(open(f))))


def ci_rows(data):
    out = []
    for x in data['runs']:
        if x['kind'] == 'none':
            continue
        out.append(f"<tr><td>{'Canvas 2D' if x['kind'] == 'web' else 'native wgpu'}</td><td class=num>{x['anim']}</td><td class=num>{pct(x.get('machine_cpu_pct'))}</td><td class=num>{x['frames']}</td><td class=num>{x['draw_ms_per_frame']}</td><td class=num>{x['first_frame_ms']}</td></tr>")
    return ''.join(out)


ci_html = ''
for d, data in ci:
    run_id = os.path.basename(os.path.normpath(d))
    osname = 'Windows Server 2022' if data['os'] == 'win32' else 'Ubuntu 24.04'
    adapter = next((x['adapter'] for x in data['runs'] if x.get('adapter')), '') or ''
    name = adapter.split('name: "')[1].split('"')[0] if 'name: "' in adapter else ''
    ci_html += f'<div class=card><h3>{osname} <span class=muted>· {html.escape(run_id)}</span></h3><p class=muted style="font-size:.84rem">wgpu adapter: {html.escape(name)} (software)</p><div class=tablewrap><table><thead><tr><th>Option</th><th class=num>fps</th><th class=num>Machine CPU, % of a core</th><th class=num>Frames drawn</th><th class=num>ms per frame</th><th class=num>First frame, ms</th></tr></thead><tbody>{ci_rows(data)}</tbody></table></div></div>'

web_frames, nat_frames = frames('web'), frames('native')
TEMPLATE = open(os.path.join(HERE, 'report-template.html')).read()
page = TEMPLATE
for k, v in {
    '{{MAC_TABLE}}': mac_table,
    '{{BASE_CPU}}': pct(base['cpu_pct']), '{{BASE_MEM}}': mb(base['mem_total_start']), '{{BASE_PROCS}}': procs(base),
    '{{WEB_PROCS}}': procs(r('web anim=10')), '{{NAT_PROCS}}': procs(r('native anim=10')),
    '{{WEB_BYPROC}}': by_proc(r('web anim=10 --cover-after 4')) if 'cpu_by_process' in r('web anim=10 --cover-after 4') else '',
    '{{WEB_MARKS}}': marks(web10), '{{NAT_MARKS}}': marks(nat10),
    '{{WEB_FF}}': ', '.join(map(str, ff('web'))), '{{NAT_FF}}': ', '.join(map(str, ff('native'))),
    '{{GPU_IDLE}}': f"{mac['gpu_idle_baseline']:.0f}",
    '{{CI}}': ci_html,
    '{{WEB_FRAMES}}': json.dumps(web_frames), '{{NAT_FRAMES}}': json.dumps(nat_frames),
    '{{WHEN}}': mac['when'],
    'CANVAS_10': pct(r('web anim=10')['cpu_pct']), 'NATIVE_10': pct(r('native anim=10')['cpu_pct']),
    'CANVAS_30': pct(r('web anim=30')['cpu_pct']), 'NATIVE_30': pct(r('native anim=30')['cpu_pct']),
    'CANVAS_MEM': mb(r('web anim=10')['mem_total_start']), 'NATIVE_MEM': mb(r('native anim=10')['mem_total_start']),
    'VERDICT_HEAD': open(os.path.join(HERE, 'verdict-head.txt')).read().strip(),
    'VERDICT_BODY': open(os.path.join(HERE, 'verdict-body.html')).read(),
}.items():
    page = page.replace(k, v)
open(sys.argv[1], 'w').write(page)
print('wrote', sys.argv[1], len(page) // 1024, 'KB')
