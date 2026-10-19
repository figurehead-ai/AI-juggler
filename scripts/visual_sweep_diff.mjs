#!/usr/bin/env node
// D-21 / AJ-E811: the full sweep's pairwise diff. Compares the renders of
// the whole gallery (scripts/visual_sweep.sh render) state by state and
// writes the report — summary.md, summary.json and index.html — with
// base/head/diff images for every state that changed.
//
//   node scripts/visual_sweep_diff.mjs --ui <dir with node_modules> SWEEP REPORT
//
// SWEEP holds base/ and head/, and normally base-recheck/ and
// head-recheck/: each side rendered a second time. Both sides come from
// one pinned container, so a settled screen renders byte-for-byte the
// same; a state is CHANGED when base and head differ by any pixel and
// each side rendered it identically twice. A state whose own side
// differed between its two renders is UNSTABLE (a capture taken
// mid-update): listed apart, with its images, so it is neither hidden
// nor mistaken for the change. Decoding and the diff images use
// Chromium's canvas (Playwright is already a dependency).
// The report is for review, not a gate: it exits 0 whatever changed,
// and 1 only when a side has no captures at all.
import { createRequire } from "node:module";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

const argv = process.argv.slice(2);
let ui = process.cwd();
const pos = [];
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === "--ui") ui = argv[++i];
  else pos.push(argv[i]);
}
if (pos.length !== 2) {
  console.error("usage: visual_sweep_diff.mjs --ui <ui dir> SWEEP REPORT");
  process.exit(2);
}
const [sweepDir, reportDir] = pos.map((p) => resolve(p));
const require = createRequire(resolve(ui, "package.json"));
const { chromium } = require("@playwright/test");

function pngs(root) {
  const out = [];
  const walk = (dir) => {
    if (!existsSync(dir)) return;
    for (const e of readdirSync(dir, { withFileTypes: true })) {
      const p = join(dir, e.name);
      if (e.isDirectory() && e.name !== "_render") walk(p);
      else if (e.isFile() && e.name.endsWith(".png")) out.push(relative(root, p));
    }
  };
  walk(root);
  return new Set(out);
}

// Each render (or render shard) leaves _render/<id>.meta.json — the ref,
// the Playwright exit code — and its JSON report. A test that failed
// even on retry means some states were not captured on that side.
function renderMeta(root) {
  const meta = { sha: "unknown", ref: "", exit: [], failed: [] };
  const dir = join(root, "_render");
  if (!existsSync(dir)) return meta;
  for (const f of readdirSync(dir).filter((f) => f.endsWith(".meta.json")).sort()) {
    const m = JSON.parse(readFileSync(join(dir, f), "utf8"));
    meta.sha = m.sha ?? meta.sha;
    meta.ref = m.ref ?? meta.ref;
    meta.exit.push(m.exit);
    const reportPath = join(dir, f.replace(".meta.json", ".playwright.json"));
    if (!existsSync(reportPath)) continue;
    const walk = (suite) => {
      for (const spec of suite.specs ?? []) if (!spec.ok) meta.failed.push(`${spec.file} › ${spec.title}`);
      for (const s of suite.suites ?? []) walk(s);
    };
    try {
      for (const s of JSON.parse(readFileSync(reportPath, "utf8")).suites ?? []) walk(s);
    } catch {
      meta.failed.push(`(unreadable Playwright report ${f})`);
    }
  }
  meta.failed = [...new Set(meta.failed)].sort();
  return meta;
}

// Runs in the page: decode both PNGs, count differing pixels, draw the
// diff (unchanged pixels as a faint grey of the first, changed ones red).
async function compare([a, b]) {
  const load = async (b64) =>
    createImageBitmap(await (await fetch(`data:image/png;base64,${b64}`)).blob(), {
      colorSpaceConversion: "none",
      premultiplyAlpha: "none",
    });
  const [ia, ib] = await Promise.all([load(a), load(b)]);
  const w = Math.max(ia.width, ib.width);
  const h = Math.max(ia.height, ib.height);
  const pixels = (im) => {
    const ctx = new OffscreenCanvas(w, h).getContext("2d", { willReadFrequently: true });
    ctx.drawImage(im, 0, 0);
    return ctx.getImageData(0, 0, w, h).data;
  };
  const pa = pixels(ia);
  const pb = pixels(ib);
  const out = new ImageData(w, h);
  const d = out.data;
  let n = 0;
  let x0 = w, y0 = h, x1 = -1, y1 = -1;
  for (let i = 0; i < pa.length; i += 4) {
    if (pa[i] !== pb[i] || pa[i + 1] !== pb[i + 1] || pa[i + 2] !== pb[i + 2] || pa[i + 3] !== pb[i + 3]) {
      n++;
      d[i] = 255; d[i + 1] = 0; d[i + 2] = 0; d[i + 3] = 255;
      const p = i / 4, x = p % w, y = (p - x) / w;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
    } else {
      const l = 0.299 * pa[i] + 0.587 * pa[i + 1] + 0.114 * pa[i + 2];
      const v = 255 - (255 - l) * 0.1 * (pa[i + 3] / 255);
      d[i] = v; d[i + 1] = v; d[i + 2] = v; d[i + 3] = 255;
    }
  }
  const c = new OffscreenCanvas(w, h);
  c.getContext("2d").putImageData(out, 0, 0);
  const bytes = new Uint8Array(await (await c.convertToBlob({ type: "image/png" })).arrayBuffer());
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return {
    pixels: n,
    total: w * h,
    sizes: [`${ia.width}x${ia.height}`, `${ib.width}x${ib.height}`],
    bbox: n ? [x0, y0, x1 - x0 + 1, y1 - y0 + 1] : null,
    diff: btoa(s),
  };
}

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
const short = (sha) => (sha || "unknown").slice(0, 12);

const dirs = {
  base: join(sweepDir, "base"),
  head: join(sweepDir, "head"),
  baseRecheck: join(sweepDir, "base-recheck"),
  headRecheck: join(sweepDir, "head-recheck"),
};
const base = renderMeta(dirs.base);
const head = renderMeta(dirs.head);
const set = Object.fromEntries(Object.entries(dirs).map(([k, d]) => [k, pngs(d)]));
const rechecked = set.baseRecheck.size > 0 && set.headRecheck.size > 0;
if (set.base.size === 0 || set.head.size === 0) {
  console.error(`visual sweep: no captures (base ${set.base.size}, head ${set.head.size}) — the render failed`);
  process.exit(1);
}

mkdirSync(reportDir, { recursive: true });
// Empty it, not remove it: the report directory may be a mount point.
for (const e of readdirSync(reportDir)) rmSync(join(reportDir, e), { recursive: true, force: true });
const place = (rel, name, from) => {
  const to = join(reportDir, "states", rel.replace(/\.png$/, `.${name}.png`));
  mkdirSync(dirname(to), { recursive: true });
  if (typeof from === "string") copyFileSync(from, to);
  else writeFileSync(to, Buffer.from(from.diff, "base64"));
  return relative(reportDir, to);
};

const changed = [];
const unstable = [];
const added = [];
const removed = [];
let unchanged = 0;
const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  const diff = async (a, b) => {
    const ba = readFileSync(a);
    const bb = readFileSync(b);
    if (ba.equals(bb)) return null;
    const r = await page.evaluate(compare, [ba.toString("base64"), bb.toString("base64")]);
    return r.pixels ? r : null; // same pixels, different encoding
  };
  // A side is settled for a state when its recheck rendered it the same;
  // a missing recheck capture (its test failed) can't confirm it.
  const settled = async (rel, side, recheck) =>
    set[recheck].has(rel) ? !(await diff(join(dirs[side], rel), join(dirs[recheck], rel))) : null;

  for (const rel of [...new Set([...set.base, ...set.head])].sort()) {
    const a = set.base.has(rel) ? join(dirs.base, rel) : null;
    const b = set.head.has(rel) ? join(dirs.head, rel) : null;
    if (!a) {
      added.push({ state: rel, head: place(rel, "head", b) });
      continue;
    }
    if (!b) {
      removed.push({ state: rel, base: place(rel, "base", a) });
      continue;
    }
    const r = await diff(a, b);
    if (!r) {
      unchanged++;
      continue;
    }
    const entry = {
      state: rel,
      pixels: r.pixels,
      ratio: r.pixels / r.total,
      size: r.sizes[0] === r.sizes[1] ? r.sizes[0] : `${r.sizes[0]} → ${r.sizes[1]}`,
      bbox: r.bbox,
      base: place(rel, "base", a),
      head: place(rel, "head", b),
      diff: place(rel, "diff", r),
    };
    if (!rechecked) {
      changed.push(entry);
      continue;
    }
    const baseSettled = await settled(rel, "base", "baseRecheck");
    const headSettled = await settled(rel, "head", "headRecheck");
    if (baseSettled === false || headSettled === false) {
      // Show what moved between the unsettled side's two renders.
      const [side, recheck] = headSettled === false ? ["head", "headRecheck"] : ["base", "baseRecheck"];
      const noise = await diff(join(dirs[side], rel), join(dirs[recheck], rel));
      unstable.push({ ...entry, unsettled: [baseSettled === false && "base", headSettled === false && "head"].filter(Boolean), noise: place(rel, `${side}-recheck-diff`, noise) });
    } else {
      changed.push({ ...entry, unconfirmed: baseSettled === null || headSettled === null });
    }
  }
} finally {
  await browser.close();
}
const byPixels = (x, y) => y.pixels - x.pixels || x.state.localeCompare(y.state);
changed.sort(byPixels);
unstable.sort(byPixels);

const summary = {
  base: { sha: base.sha, ref: base.ref, captures: set.base.size, playwrightExit: base.exit, failedTests: base.failed },
  head: { sha: head.sha, ref: head.ref, captures: set.head.size, playwrightExit: head.exit, failedTests: head.failed },
  rechecked,
  states: new Set([...set.base, ...set.head]).size,
  unchanged,
  changed,
  unstable,
  added,
  removed,
};
writeFileSync(join(reportDir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);

const pct = (r) => `${(r * 100).toFixed(r < 0.0001 ? 4 : 2)} %`;
const md = [];
md.push(`## Visual full sweep (D-21)`, "");
md.push(`Base \`${short(base.sha)}\`${base.ref ? ` (${base.ref})` : ""} → head \`${short(head.sha)}\`${head.ref ? ` (${head.ref})` : ""}.`);
md.push(
  `${summary.states} states: **${changed.length} changed**, ${unstable.length} unstable, ${added.length} only at head, ${removed.length} only at base, ${unchanged} unchanged.` +
    (rechecked ? "" : " (No rechecks: unstable states are not told apart from changed ones.)"),
  "",
);
for (const [name, side] of [["base", base], ["head", head]]) {
  if (side.failed.length) {
    md.push(`**${side.failed.length} gallery test(s) failed at ${name}** — their states are missing from that side:`, "");
    for (const t of side.failed) md.push(`- ${t}`);
    md.push("");
  }
}
if (changed.length) {
  md.push("| Changed state | Differing pixels | Share | Size | Changed region (x, y, w, h) |", "|---|---:|---:|---|---|");
  for (const c of changed)
    md.push(`| \`${c.state}\`${c.unconfirmed ? " (not rechecked)" : ""} | ${c.pixels} | ${pct(c.ratio)} | ${c.size} | ${c.bbox.join(", ")} |`);
  md.push("");
}
if (unstable.length) {
  md.push(
    "Unstable — rendered differently twice on the same commit, so the sweep can't tell whether the change touched them (images in the report):",
    "",
  );
  for (const u of unstable) md.push(`- \`${u.state}\` (${u.unsettled.join(" and ")}; base→head ${u.pixels} px)`);
  md.push("");
}
for (const [title, list] of [["Only at head (new states)", added], ["Only at base (states no longer captured)", removed]]) {
  if (!list.length) continue;
  md.push(`${title}:`, "");
  for (const s of list) md.push(`- \`${s.state}\``);
  md.push("");
}
md.push("Images: `states/<screen>/<state>.{base,head,diff}.png`; open `index.html` to review them side by side.");
writeFileSync(join(reportDir, "summary.md"), `${md.join("\n")}\n`);

const fig = (src, cap) =>
  `<figure><figcaption>${cap}</figcaption><a href="${esc(src)}"><img src="${esc(src)}" loading="lazy" alt="${cap}"></a></figure>`;
const html = [];
html.push(
  `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Visual full sweep</title>`,
  `<style>body{font:13px/1.5 system-ui;background:#0a0d13;color:#e8ecf3;margin:24px}h2{margin-top:32px;font-size:15px}h3{font-size:13px;margin-top:20px}code{color:#9d9dfb}`,
  `.row{display:flex;gap:12px;flex-wrap:wrap}figure{margin:0;flex:1 1 300px;max-width:480px}figcaption{color:#8b95a7;font-size:11px}`,
  `img{width:100%;border:1px solid #232a38;border-radius:6px;background:#fff}.warn{color:#ffb86b}</style></head><body>`,
  `<h1>Visual full sweep (D-21)</h1>`,
  `<p>Base <code>${esc(short(base.sha))}</code> ${esc(base.ref)} → head <code>${esc(short(head.sha))}</code> ${esc(head.ref)}.<br>`,
  `${summary.states} states: <strong>${changed.length} changed</strong>, ${unstable.length} unstable, ${added.length} only at head, ${removed.length} only at base, ${unchanged} unchanged.</p>`,
);
for (const [name, side] of [["base", base], ["head", head]]) {
  if (side.failed.length)
    html.push(`<p class="warn">${side.failed.length} gallery test(s) failed at ${name}:</p><ul>${side.failed.map((t) => `<li>${esc(t)}</li>`).join("")}</ul>`);
}
if (changed.length) html.push("<h2>Changed</h2>");
for (const c of changed) {
  html.push(
    `<h3><code>${esc(c.state)}</code> — ${c.pixels} px (${pct(c.ratio)}), region ${c.bbox.join(", ")}${c.unconfirmed ? " — not rechecked" : ""}</h3>`,
    `<div class="row">${fig(c.base, "base")}${fig(c.head, "head")}${fig(c.diff, "diff")}</div>`,
  );
}
if (unstable.length) html.push(`<h2 class="warn">Unstable (rendered differently twice on the same commit)</h2>`);
for (const u of unstable) {
  html.push(
    `<h3><code>${esc(u.state)}</code> — unstable at ${u.unsettled.join(" and ")}; base→head ${u.pixels} px</h3>`,
    `<div class="row">${fig(u.base, "base")}${fig(u.head, "head")}${fig(u.diff, "base→head diff")}${fig(u.noise, `${u.unsettled.at(-1)}: its two renders`)}</div>`,
  );
}
if (added.length) html.push("<h2>Only at head</h2>");
for (const s of added) html.push(`<h3><code>${esc(s.state)}</code></h3><div class="row">${fig(s.head, "head")}</div>`);
if (removed.length) html.push("<h2>Only at base</h2>");
for (const s of removed) html.push(`<h3><code>${esc(s.state)}</code></h3><div class="row">${fig(s.base, "base")}</div>`);
html.push("</body></html>");
writeFileSync(join(reportDir, "index.html"), `${html.join("\n")}\n`);

console.log(
  `visual sweep: ${summary.states} states — ${changed.length} changed, ${unstable.length} unstable, ${added.length} added, ${removed.length} removed, ${unchanged} unchanged → ${reportDir}`,
);
