// Idle re-renders: how often the window's React tree commits, and which
// components render, while nobody touches it (the idle-CPU work, 0.4.1).
//
//   node scripts/qa/idle_renders.mjs <url> [--seconds 60] [--settle 10]
//        [--hidden] [--out file.json]
//
// Run from ui/ (it uses ui's Playwright). A stub React DevTools hook is
// installed before the page loads; React calls it on every commit. Each
// commit's rendered components (function, class, forwardRef and memo
// fibers that did work) are counted by name. The page loads, settles for
// --settle seconds, then is watched for --seconds. --hidden measures the
// window hidden (the page's visibility set to hidden). Names need the
// dev server (a production build minifies them).
import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

const require = createRequire(join(process.cwd(), "package.json"));
const { chromium } = require("@playwright/test");

const args = process.argv.slice(2);
const url = args.find((a) => !a.startsWith("--") && !/^\d+$/.test(a));
const opt = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : fallback;
};
const seconds = Number(opt("seconds", "60"));
const settle = Number(opt("settle", "10"));
const hidden = args.includes("--hidden");
if (!url) {
  console.error("idle_renders: say the page's URL");
  process.exit(2);
}

const hook = () => {
  const counts = { commits: 0, renders: 0, by: {}, origins: {} };
  window.__AIJ_RENDERS__ = counts;
  const nameOf = (type) => (type && (type.displayName || type.name || (type.render && (type.render.displayName || type.render.name)) || (type.type && nameOf(type.type)))) || "?";
  const component = (f) => [0, 1, 11, 14, 15].includes(f.tag);
  // PerformedWork (1): the component's function ran in this commit
  const rendered = (f) => component(f) && (f.flags & 1) === 1 && f.alternate !== null;
  const walk = (fiber) => {
    // [fiber, whether the nearest component above it rendered]
    const stack = [[fiber, false]];
    while (stack.length) {
      const [f, above] = stack.pop();
      if (!f) continue;
      let below = above;
      if (component(f)) {
        const did = rendered(f);
        if (did) {
          counts.renders++;
          const name = nameOf(f.type);
          counts.by[name] = (counts.by[name] || 0) + 1;
          // an origin: it rendered while its parent component didn't (its
          // own state, a context, or a store changed)
          if (!above) counts.origins[name] = (counts.origins[name] || 0) + 1;
        }
        below = did;
      }
      if (f.sibling) stack.push([f.sibling, above]);
      if (f.child) stack.push([f.child, below]);
    }
  };
  window.__REACT_DEVTOOLS_GLOBAL_HOOK__ = {
    supportsFiber: true,
    isDisabled: false,
    renderers: new Map(),
    inject(renderer) {
      this.renderers.set(1, renderer);
      return 1;
    },
    checkDCE() {},
    onScheduleFiberRoot() {},
    onCommitFiberUnmount() {},
    onPostCommitFiberRoot() {},
    onCommitFiberRoot(_id, root) {
      if (!window.__AIJ_WATCHING__) return;
      counts.commits++;
      walk(root.current.child);
    },
  };
};

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.addInitScript(hook);
await page.goto(url);
await page.waitForTimeout(settle * 1000);
if (hidden) {
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" });
    Object.defineProperty(document, "hidden", { configurable: true, get: () => true });
    document.dispatchEvent(new Event("visibilitychange"));
    window.dispatchEvent(new Event("blur"));
  });
}
await page.evaluate(() => {
  window.__AIJ_WATCHING__ = true;
});
await page.waitForTimeout(seconds * 1000);
const counts = await page.evaluate(() => window.__AIJ_RENDERS__);
await browser.close();
const perMinute = (n) => Math.round((n * 60) / seconds);
const top = Object.entries(counts.by).sort((a, b) => b[1] - a[1]).slice(0, 15);
const result = {
  url: url.replace(/#token=[^&]*/, "#token=…"),
  seconds,
  hidden,
  commits_per_minute: perMinute(counts.commits),
  renders_per_minute: perMinute(counts.renders),
  top: top.map(([name, n]) => ({ name, per_minute: perMinute(n) })),
  origins: Object.entries(counts.origins)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 15)
    .map(([name, n]) => ({ name, per_minute: perMinute(n) })),
};
console.log(JSON.stringify(result, null, 2));
const out = opt("out", null);
if (out) writeFileSync(out, JSON.stringify(result, null, 2));
