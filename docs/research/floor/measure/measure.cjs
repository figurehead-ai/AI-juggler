// Measures the Office prototype's CPU in a headless browser, one scenario at a time.
// Usage: NODE_PATH=<a ui/node_modules with @playwright/test> node measure.cjs [webkit|chromium] [seconds] [headed]
// headed opens a real window (GPU compositing, closer to the app); headless rasterises in software.
// CPU is read from the OS (ps cumulative CPU time of the browser's process tree), so it
// includes layout, raster and compositing, not just our script. The page is the same
// file published as the prototype, opened with ?measure so the test drives it.
'use strict';
const { execFileSync } = require('child_process');
const path = require('path');
const pw = require('@playwright/test');

const engine = process.argv[2] || 'webkit';
const SECS = Number(process.argv[3] || 15);
const HEADED = process.argv[4] === 'headed';
const page_url = 'file://' + path.resolve(__dirname, '..', 'prototype.html') + '?measure=1';

function psRows() {
  const out = execFileSync('ps', ['-A', '-o', 'pid=,ppid=,time=,comm='], { encoding: 'utf8' });
  return out.trim().split('\n').map(l => l.trim().split(/\s+/)).map(([pid, ppid, t, ...cmd]) => ({ pid: +pid, ppid: +ppid, t, cmd: cmd.join(' ') }));
}
// WebKit starts its WebContent, GPU and Networking processes through launchd (parent pid 1),
// so they are not children of the browser. They are found as the Playwright WebKit XPC
// processes that appeared after this browser was launched.
const xpcOf = rows => new Set(rows.filter(r => /ms-playwright\/webkit-[^/]+\/com\.apple\.WebKit\./.test(r.cmd)).map(r => r.pid));
let extraPids = new Set();
function cpuSeconds(rootPid) {
  const rows = psRows();
  const kids = new Map();
  for (const r of rows) { if (!kids.has(r.ppid)) kids.set(r.ppid, []); kids.get(r.ppid).push(r); }
  const tree = []; const stack = [rootPid];
  while (stack.length) { const p = stack.pop(); for (const r of kids.get(p) || []) { tree.push(r); stack.push(r.pid); } }
  const self = rows.find(r => r.pid === rootPid); if (self) tree.push(self);
  for (const r of rows) if (extraPids.has(r.pid)) tree.push(r);
  const secs = t => { const parts = t.split(':').map(Number); return parts.reduce((a, v) => a * 60 + v, 0); };
  return tree.reduce((a, r) => a + secs(r.t), 0);
}

(async () => {
  const before = xpcOf(psRows());
  const server = await pw[engine].launchServer({ headless: !HEADED });
  const pid = server.process().pid;
  const browser = await pw[engine].connect(server.wsEndpoint());
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  await page.goto('about:blank');
  if (engine === 'webkit') { extraPids = new Set([...xpcOf(psRows())].filter(p => !before.has(p))); console.error('webkit xpc pids', [...extraPids]); }
  const wait = ms => new Promise(r => setTimeout(r, ms));
  const results = [];
  async function measure(name, setup) {
    if (setup) await setup();
    await wait(2500);
    const s0 = await page.evaluate(() => window.floorTest ? window.floorTest.stats() : null);
    const c0 = cpuSeconds(pid), t0 = Date.now();
    await wait(SECS * 1000);
    const c1 = cpuSeconds(pid), t1 = Date.now();
    const s1 = await page.evaluate(() => window.floorTest ? window.floorTest.stats() : null);
    const sec = (t1 - t0) / 1000;
    const row = { name, cpuPct: +(100 * (c1 - c0) / sec).toFixed(2) };
    if (s0 && s1) {
      const frames = s1.frames - s0.frames;
      Object.assign(row, { fps: +(frames / sec).toFixed(2), drawMsPerFrame: frames ? +((s1.drawMs - s0.drawMs) / frames).toFixed(2) : 0, scriptDrawPct: +(100 * (s1.drawMs - s0.drawMs) / 1000 / sec).toFixed(2), level: s1.level, floors: s1.floors, people: s1.chars, bakeMs: +(s1.bakeMs || 0).toFixed(1) });
    }
    results.push(row); console.error(JSON.stringify(row));
  }
  const ft = (fn, ...a) => page.evaluate(([f, args]) => window.floorTest[f](...args), [fn, a]);

  await page.goto('about:blank');
  await measure('Browser with a blank page (baseline)');
  await page.goto(page_url); await page.waitForFunction(() => window.floorTest);
  await measure('At rest: floor plan, no data changes', async () => { await ft('zoom', 'far'); await ft('motion', 'full'); });
  await measure('Live data every 2.4 s: floor plan', async () => { await ft('sim', true); });
  await measure('At rest: desks, motion off', async () => { await ft('sim', false); await ft('motion', 'off'); await ft('zoom', 'mid'); });
  await measure('Animating: desks, Calm motion (4 fps)', async () => { await ft('motion', 'calm'); });
  await measure('Animating: desks, Full motion (10 fps)', async () => { await ft('motion', 'full'); });
  await measure('Animating + live data: desks, Full', async () => { await ft('sim', true); });
  await measure('Animating: up close with monitor cards', async () => { await ft('zoom', 'near'); });
  await measure('Red alert + App on fire: floor plan, Full', async () => { await ft('zoom', 'far'); await ft('overload', true); await ft('mac', true); });
  await measure('Page hidden (visibilitychange)', async () => {
    await page.evaluate(() => { Object.defineProperty(document, 'hidden', { configurable: true, get: () => true }); document.dispatchEvent(new Event('visibilitychange')); });
  });
  await page.goto(page_url); await page.waitForFunction(() => window.floorTest);
  await measure('Big company (20 Apps, 150 sessions, 700 subagents): desks, Full + live', async () => { await ft('preset', 'big'); await ft('zoom', 'mid'); await ft('motion', 'full'); await ft('sim', true); });
  let dragging = null;
  await measure('Panning continuously (drag at 60 Hz)', async () => {
    await ft('sim', false);
    const drag = async () => { await page.mouse.move(500, 450); await page.mouse.down(); for (let i = 0; i < 60 * (SECS + 3); i++) { await page.mouse.move(500 + 200 * Math.sin(i / 40), 450 + 120 * Math.cos(i / 50)); await wait(16); } await page.mouse.up(); };
    dragging = drag();
  });
  await dragging.catch(() => {});
  console.log(JSON.stringify({ engine, headed: HEADED, seconds: SECS, processes: engine === 'webkit' ? 'UI process + WebContent, GPU and Networking XPC processes' : 'browser process tree', viewport: '1440x900 @2x', when: new Date().toISOString(), results }, null, 2));
  await browser.close(); await server.close();
})().catch(e => { console.error(e); process.exit(1); });
