/**
 * AJ-E1546 — records each clip (`src/lib/clips.ts`) in both themes from
 * the gallery's generic fixtures (never real names): a scripted loop on
 * the real UI, recorded with Playwright's `recordVideo`, then cut, framed
 * and encoded with Playwright's own cached ffmpeg (VP9 in WebM, no sound)
 * into `public/clips/<id>-<theme>.webm`, with its last frame as the still
 * poster (`.png`) Reduce Motion shows.
 *
 * Run by hand: `scripts/record_clips.sh` (it finds the cached ffmpeg and
 * never downloads anything). Not part of any gate.
 */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { allStatuses, goTo, openRailSession } from "../visual/fixtures";
import { STATES as BOT_STATES } from "../../src/preview/bot";
import type { ClipId } from "../../src/lib/clips";

const OUT = resolve(process.cwd(), "public/clips");
const RAW = resolve(process.env.AIJ_CLIPS_RAW ?? join(process.cwd(), "test-results", "clips-raw"));
const FFMPEG = process.env.AIJ_FFMPEG ?? join(homedir(), "Library/Caches/ms-playwright/ffmpeg-1013/ffmpeg-mac");
const VIEW = { width: 1200, height: 750 };
/** The clip's size: 16:10, shown at up to about 360 px (twice that for a
 * sharp picture on a Retina screen). */
const SIZE = { width: 640, height: 400 };
const NOW = 1_728_800_000_000;

/** A question and its preset, cited answer (the gallery's Bot fixture). */
const QUESTION = "What changed in shop yesterday?";
const ANSWER = (() => {
  const turn = BOT_STATES.beta!.bot.turns!.find((t) => t.question === QUESTION)!;
  const { id: _id, at_ms: _at, question: _q, ...rest } = turn;
  return rest;
})();

/** Two setups found on a computer that already has Claude Code. */
const SCAN = {
  installs: [
    ["/Users/you/.claude", "vendor-default", "Main", "Active this week"],
    ["/Users/you/Library/Application Support/ClaudeCodeProfiles/work", "manager-tool", "Work", "Dormant 12 d"],
  ].map(([path, origin, purpose, words]) => ({
    harness: "claude-code",
    path,
    origin,
    login: "logged_in",
    bytes: 30_000_000,
    last_used_secs: Math.floor(NOW / 1000) - 86_400,
    managed: false,
    notes: [],
    inference: {
      purpose,
      purpose_confidence: "likely",
      login: "signed-in",
      login_confidence: "found",
      activity: words!.startsWith("Active") ? "active" : "dormant",
      activity_confidence: "found",
      activity_words: words,
      keychain_bound: false,
    },
    signals: [origin],
  })),
  probes: [{ label: "Your standard setup (~/.claude)", health: "ok", detail: "1 found", reason: null }],
  seeds: [],
  desktop_apps: ["Claude"],
  cancelled: false,
};

interface Recipe {
  /** What the window starts with. */
  fixture?: unknown;
  seams?: Record<string, unknown>;
  firstRun?: boolean;
  /** Gets the page to the loop's first frame; returns what to frame. */
  setup: (page: Page) => Promise<Locator | null>;
  /** The loop itself, about 4–5 s. */
  play: (page: Page) => Promise<void>;
  /** The still: the loop's last frame, or its first (a loop that ends on another page). */
  poster?: "start" | "end";
}

const pause = (page: Page, ms: number) => page.waitForTimeout(ms);

/** Moves the pointer over each of `rows`, resting on each. */
async function hoverRows(page: Page, rows: Locator, count: number, rest: number): Promise<void> {
  const n = Math.min(count, await rows.count());
  for (let at = 0; at < n; at += 1) {
    await rows.nth(at).hover().catch(() => undefined);
    await pause(page, rest);
  }
}

const main = (page: Page) => page.locator("main").first();

/** 0.5.1 F3 (AJ-E1546): perm-focus shows the effect itself: AI Juggler's
 * real window (the app, in a frame, on the gallery's fixtures) in front of
 * a session's own app window and a terminal; Go to it on that session, and
 * its own window comes to the front. Generic names only (the fixture's
 * "Fake Desk" app and its sessions). */
const FOCUS_SESSION = "sess-desktop-1";
const FOCUS_TITLES = ["Sketch the pricing page", "Summarise the support inbox", "Rename the photo exports"];

function focusScene(theme: "dark" | "light"): string {
  const dark = theme === "dark";
  const c = dark
    ? { desk: "linear-gradient(135deg,#1d2330,#2a2440)", win: "#1b1f27", bar: "#232833", ink: "#e7e9ee", muted: "#9aa3b2", line: "#2e3440", side: "#171a21", pick: "#2b3346", bubble: "#262c38", mine: "#36406a" }
    : { desk: "linear-gradient(135deg,#dfe6f2,#e9e1f3)", win: "#ffffff", bar: "#eef0f4", ink: "#1d2430", muted: "#5d6675", line: "#dde1e8", side: "#f5f6f8", pick: "#e3e8f5", bubble: "#eef0f4", mine: "#dfe4fb" };
  const lights = `<span class="l"></span><span class="l"></span><span class="l"></span>`;
  const rows = FOCUS_TITLES.map((title, at) => `<div class="row${at === 0 ? " pick" : ""}">${title}</div>`).join("");
  return `<!doctype html><html><head><meta charset="utf-8"><style>
  html,body{margin:0;height:100%;overflow:hidden;font:13px -apple-system,BlinkMacSystemFont,"Helvetica Neue",sans-serif;color:${c.ink}}
  #stage{position:absolute;inset:0;background:${c.desk}}
  .win{position:absolute;border-radius:10px;overflow:hidden;background:${c.win};box-shadow:0 10px 30px rgba(0,0,0,${dark ? 0.55 : 0.18}),0 0 0 1px rgba(0,0,0,${dark ? 0.5 : 0.08});transition:transform .45s cubic-bezier(.2,.8,.2,1),box-shadow .45s,filter .45s}
  .bar{height:30px;display:flex;align-items:center;gap:8px;padding:0 12px;background:${c.bar};border-bottom:1px solid ${c.line};font-size:12px;color:${c.muted}}
  .bar b{font-weight:600;color:${c.ink};margin-left:8px}
  .l{width:11px;height:11px;border-radius:50%;background:${dark ? "#4a5060" : "#cdd1d8"}}
  .front .l:nth-child(1){background:#ff5f57}.front .l:nth-child(2){background:#febc2e}.front .l:nth-child(3){background:#28c840}
  .back{filter:saturate(.85) brightness(${dark ? 0.85 : 0.97})}
  #aij{left:40px;top:60px;width:760px;height:505px;z-index:3}
  #aij iframe{border:0;width:1150px;height:719px;transform:scale(.661);transform-origin:0 0}
  #desk{left:470px;top:170px;width:690px;height:450px;z-index:2;display:flex;flex-direction:column}
  #desk .body{flex:1;display:flex;min-height:0}
  #desk .side{width:210px;background:${c.side};border-right:1px solid ${c.line};padding:12px 8px}
  #desk .row{padding:8px 10px;border-radius:7px;color:${c.muted};margin-bottom:2px}
  #desk .row.pick{background:${c.pick};color:${c.ink};font-weight:600}
  #desk .chat{flex:1;padding:18px 22px;display:flex;flex-direction:column;gap:12px}
  #desk h3{margin:0 0 4px;font-size:15px}
  .msg{max-width:78%;padding:10px 12px;border-radius:12px;background:${c.bubble};line-height:1.4}
  .msg.me{align-self:flex-end;background:${c.mine}}
  #term{left:620px;top:40px;width:520px;height:250px;z-index:1;background:#0b0d12}
  #term pre{margin:0;padding:12px 14px;color:#c9d1d9;font:12px/1.5 ui-monospace,Menlo,monospace}
  #desk.forward{z-index:4;transform:translateY(-6px) scale(1.01);box-shadow:0 18px 44px rgba(0,0,0,${dark ? 0.65 : 0.26}),0 0 0 2px #7c83ff}
  </style></head><body><div id="stage">
  <div class="win back" id="term"><div class="bar">${lights}<b>Terminal</b> — storefront</div><pre>~/storefront $ npm test\n  ✓ cart totals (12 ms)\n  ✓ checkout flow (48 ms)\n~/storefront $ </pre></div>
  <div class="win back" id="desk"><div class="bar">${lights}<b>Fake Desk</b> — ${FOCUS_TITLES[0]}</div><div class="body"><div class="side">${rows}</div>
    <div class="chat"><h3>${FOCUS_TITLES[0]}</h3><div class="msg me">Three plans, monthly or yearly, with the middle one marked.</div><div class="msg">Here's a first layout: the plans side by side, yearly prices under the switch, and a short FAQ below.</div></div></div></div>
  <div class="win front" id="aij"><div class="bar">${lights}<b>AI Juggler</b></div><iframe src="/" title="AI Juggler"></iframe></div>
  </div></body></html>`;
}

/** 0.5.1 AJ-E1545 R3: perm-notify shows the effect: the user works in
 * another app; a session needs a reply; AI Juggler's notice slides in at
 * the top right, in the words AI Juggler sends (`needs.rs`: "“<name>” needs
 * your reply", its place · account under it); a click brings AI Juggler
 * forward on that session. Generic names only (the fixture's). */
function notifyScene(theme: "dark" | "light"): string {
  const dark = theme === "dark";
  const c = dark
    ? { desk: "linear-gradient(135deg,#1d2330,#2a2440)", win: "#1b1f27", bar: "#232833", ink: "#e7e9ee", muted: "#9aa3b2", line: "#2e3440", note: "rgba(40,44,54,.92)" }
    : { desk: "linear-gradient(135deg,#dfe6f2,#e9e1f3)", win: "#ffffff", bar: "#eef0f4", ink: "#1d2430", muted: "#5d6675", line: "#dde1e8", note: "rgba(246,247,250,.94)" };
  const lights = `<span class="l"></span><span class="l"></span><span class="l"></span>`;
  return `<!doctype html><html><head><meta charset="utf-8"><style>
  html,body{margin:0;height:100%;overflow:hidden;font:13px -apple-system,BlinkMacSystemFont,"Helvetica Neue",sans-serif;color:${c.ink}}
  #stage{position:absolute;inset:0;background:${c.desk}}
  .win{position:absolute;border-radius:10px;overflow:hidden;background:${c.win};box-shadow:0 10px 30px rgba(0,0,0,${dark ? 0.55 : 0.18}),0 0 0 1px rgba(0,0,0,${dark ? 0.5 : 0.08});transition:transform .45s cubic-bezier(.2,.8,.2,1),box-shadow .45s,filter .45s}
  .bar{height:30px;display:flex;align-items:center;gap:8px;padding:0 12px;background:${c.bar};border-bottom:1px solid ${c.line};font-size:12px;color:${c.muted}}
  .bar b{font-weight:600;color:${c.ink};margin-left:8px}
  .l{width:11px;height:11px;border-radius:50%;background:${dark ? "#4a5060" : "#cdd1d8"}}
  .front .l:nth-child(1){background:#ff5f57}.front .l:nth-child(2){background:#febc2e}.front .l:nth-child(3){background:#28c840}
  .back{filter:saturate(.85) brightness(${dark ? 0.85 : 0.97})}
  #aij{left:40px;top:60px;width:760px;height:505px;z-index:1}
  #aij iframe{border:0;width:1150px;height:719px;transform:scale(.661);transform-origin:0 0}
  #term{left:300px;top:150px;width:700px;height:420px;z-index:2;background:#0b0d12}
  #term pre{margin:0;padding:14px 16px;color:#c9d1d9;font:12px/1.6 ui-monospace,Menlo,monospace}
  #aij.forward{z-index:4;transform:translateY(-6px) scale(1.01);box-shadow:0 18px 44px rgba(0,0,0,${dark ? 0.65 : 0.26}),0 0 0 2px #7c83ff}
  #note{position:absolute;z-index:5;top:16px;right:16px;width:340px;display:flex;gap:10px;padding:11px 12px;border-radius:14px;background:${c.note};backdrop-filter:blur(18px);box-shadow:0 8px 28px rgba(0,0,0,${dark ? 0.5 : 0.18}),0 0 0 1px rgba(0,0,0,${dark ? 0.45 : 0.06});transform:translateX(380px);transition:transform .5s cubic-bezier(.2,.8,.2,1),opacity .35s;cursor:default}
  #note.in{transform:translateX(0)}
  #note.gone{opacity:0}
  #note .icon{width:34px;height:34px;border-radius:8px;flex:none;background:linear-gradient(135deg,#7c83ff,#b57cff)}
  #note .what{flex:1;min-width:0}
  #note .app{font-size:11px;color:${c.muted};display:flex;justify-content:space-between}
  #note .title{font-weight:600;margin:2px 0 1px}
  #note .detail{color:${c.muted};font-size:12px}
  </style></head><body><div id="stage">
  <div class="win back" id="aij"><div class="bar">${lights}<b>AI Juggler</b></div><iframe src="/" title="AI Juggler"></iframe></div>
  <div class="win front" id="term"><div class="bar">${lights}<b>Terminal</b> — storefront</div><pre>~/storefront $ npm run build\n  building for production…\n  ✓ 214 modules transformed\n  dist/index.html   0.46 kB\n~/storefront $ git status\n  On branch main\n  nothing to commit, working tree clean\n~/storefront $ </pre></div>
  <div id="note" role="button" aria-label="Notice"><div class="icon"></div><div class="what"><div class="app"><span>AI JUGGLER</span><span>now</span></div>
    <div class="title">“${FOCUS_TITLES[0]}” needs your reply</div><div class="detail">Fake Desk · Personal</div></div></div>
  </div></body></html>`;
}

const RECIPES: Record<ClipId, Recipe> = {
  "what-bot": {
    seams: { __AIJ_BOT__: { turns: [], answers: { [QUESTION]: ANSWER } }, __AIJ_AI_JOBS__: BOT_STATES.beta!.jobs },
    setup: async (page) => {
      await page.getByTestId("header-bot").click();
      await expect(page.getByTestId("bot-panel")).toBeVisible();
      return page.getByTestId("bot-panel");
    },
    play: async (page) => {
      await page.getByTestId("bot-input").click();
      await page.keyboard.type(QUESTION, { delay: 40 });
      await pause(page, 300);
      await page.keyboard.press("Enter");
      await expect(page.getByTestId("bot-turn").last()).toBeVisible();
      await pause(page, 1600);
      await page.getByTestId("bot-turn").last().locator("button, a").first().hover().catch(() => undefined);
      await pause(page, 1200);
    },
  },
  "what-sessions": {
    setup: async (page) => page.getByTestId("rail"),
    play: async (page) => {
      await hoverRows(page, page.getByTestId("rail").locator("[data-instance-id]"), 5, 550);
      await page.getByTestId("rail").locator("button[data-instance-id]").nth(1).click().catch(() => undefined);
      await pause(page, 1600);
    },
  },
  "what-usage": {
    setup: async (page) => {
      await goTo(page, "Usage");
      await pause(page, 600);
      return main(page);
    },
    play: async (page) => {
      const box = await main(page).boundingBox();
      if (box) {
        for (let step = 0; step <= 12; step += 1) {
          await page.mouse.move(box.x + 80 + step * 40, box.y + 220);
          await pause(page, 220);
        }
      }
      await page.mouse.wheel(0, 260);
      await pause(page, 1300);
    },
  },
  "what-mac": {
    setup: async (page) => {
      await goTo(page, "This Mac");
      await pause(page, 600);
      return main(page);
    },
    play: async (page) => {
      await hoverRows(page, main(page).locator("li, tr, [role='row']"), 6, 450);
      await page.mouse.wheel(0, 240);
      await pause(page, 1200);
    },
  },
  "what-setups": {
    setup: async (page) => {
      await goTo(page, "Accounts");
      await pause(page, 600);
      return main(page);
    },
    play: async (page) => {
      await hoverRows(page, main(page).locator("li, [role='row'], button"), 6, 500);
      await pause(page, 1200);
    },
  },
  "how-apps": {
    fixture: { accounts: [], profiles: [], sessions: [], projects: [] },
    seams: { __AIJ_SCAN_REPORT__: SCAN, __AIJ_OS_ACCESS__: [] },
    firstRun: true,
    poster: "start",
    setup: async (page) => {
      await expect(page.getByTestId("first-run-found")).toBeVisible();
      return page.getByTestId("flow-form");
    },
    play: async (page) => {
      await hoverRows(page, page.getByTestId("first-run-found").locator("label"), 3, 500);
      await page.getByTestId("first-run-bring-in").hover();
      await pause(page, 500);
      await page.getByTestId("first-run-bring-in").click();
      await pause(page, 2400);
    },
  },
  "how-start": {
    setup: async (page) => main(page),
    play: async (page) => {
      await page.getByTestId("header-new-session").hover();
      await pause(page, 500);
      await page.getByTestId("header-new-session").click();
      await pause(page, 1200);
      const dialog = page.getByRole("dialog").first();
      await hoverRows(page, dialog.locator("button[role='radio'], [role='option'], label"), 3, 500);
      await pause(page, 2200);
    },
  },
  "how-juggle": {
    setup: async (page) => {
      await openRailSession(page, "sess-2");
      await pause(page, 600);
      return main(page);
    },
    play: async (page) => {
      const button = page.getByTestId("switch-account").first();
      await button.hover().catch(() => undefined);
      await pause(page, 600);
      await button.click().catch(() => undefined);
      await pause(page, 1300);
      await hoverRows(page, page.getByRole("dialog").first().locator("button, [role='radio']"), 3, 550);
      await pause(page, 600);
    },
  },
  "perm-focus": {
    setup: async (page) => {
      const theme = (await page.evaluate(() => localStorage.getItem("aij.theme"))) === "light" ? "light" : "dark";
      await page.route("**/focus-scene", (route) => route.fulfill({ contentType: "text/html", body: focusScene(theme) }));
      await page.goto("/focus-scene");
      const app = page.frameLocator("#aij iframe");
      await expect(app.getByTestId("user-menu-button")).toBeVisible();
      const row = app.getByTestId("rail").locator(`button[data-instance-id="${FOCUS_SESSION}"]`).first();
      await row.click();
      await expect(app.getByRole("button", { name: "Go to it" }).first()).toBeVisible();
      return page.locator("#stage");
    },
    play: async (page) => {
      const go = page.frameLocator("#aij iframe").getByRole("button", { name: "Go to it" }).first();
      await pause(page, 700);
      await go.hover();
      await pause(page, 700);
      await go.click();
      // What the system does on Go to it: the session's own window comes forward.
      await page.evaluate(() => {
        document.getElementById("aij")!.classList.replace("front", "back");
        const desk = document.getElementById("desk")!;
        desk.classList.replace("back", "front");
        desk.classList.add("forward");
        setTimeout(() => {
          desk.style.boxShadow = "";
          desk.classList.remove("forward");
          desk.style.zIndex = "4";
        }, 900);
      });
      await pause(page, 2400);
    },
  },
  "perm-notify": {
    setup: async (page) => {
      const theme = (await page.evaluate(() => localStorage.getItem("aij.theme"))) === "light" ? "light" : "dark";
      await page.route("**/notify-scene", (route) => route.fulfill({ contentType: "text/html", body: notifyScene(theme) }));
      await page.goto("/notify-scene");
      await expect(page.frameLocator("#aij iframe").getByTestId("user-menu-button")).toBeVisible();
      return page.locator("#stage");
    },
    play: async (page) => {
      await pause(page, 600);
      // What the system shows: AI Juggler's notice slides in.
      await page.evaluate(() => document.getElementById("note")!.classList.add("in"));
      await pause(page, 1400);
      await page.locator("#note").hover();
      await pause(page, 500);
      // A click on it: AI Juggler comes forward on that session.
      await page.evaluate(() => {
        document.getElementById("note")!.classList.add("gone");
        document.getElementById("term")!.classList.replace("front", "back");
        const aij = document.getElementById("aij")!;
        aij.classList.replace("back", "front");
        aij.classList.add("forward");
        setTimeout(() => {
          aij.style.boxShadow = "";
          aij.classList.remove("forward");
          aij.style.zIndex = "4";
        }, 900);
      });
      await page.frameLocator("#aij iframe").getByTestId("rail").locator(`button[data-instance-id="${FOCUS_SESSION}"]`).first().click();
      await pause(page, 2000);
    },
  },
  "perm-preview": {
    setup: async (page) => {
      await openRailSession(page, "sess-desktop-1").catch(() => undefined);
      await pause(page, 900);
      const preview = page.getByTestId("desktop-preview");
      return (await preview.count()) > 0 ? preview : main(page);
    },
    play: async (page) => {
      const preview = page.getByTestId("desktop-preview");
      if ((await preview.count()) > 0) {
        await preview.hover().catch(() => undefined);
        await pause(page, 1500);
      }
      await pause(page, 2500);
    },
  },
};

/** A 16:10 box around `box`, inside the window, at least the clip's size. */
function frameOf(box: { x: number; y: number; width: number; height: number } | null): { x: number; y: number; width: number; height: number } {
  if (!box) return { x: 0, y: 0, width: VIEW.width, height: VIEW.height };
  let width = Math.max(box.width, SIZE.width);
  let height = Math.max(box.height, SIZE.height);
  if (width / height > 1.6) height = width / 1.6;
  else width = height * 1.6;
  width = Math.min(width, VIEW.width);
  height = Math.min(Math.round(width / 1.6), VIEW.height);
  width = Math.round(height * 1.6);
  // From the element's left edge (a page's content, not the list beside it).
  const x = Math.max(0, Math.min(box.x, VIEW.width - width));
  const y = Math.max(0, Math.min(box.y, VIEW.height - height));
  // Even numbers, as the encoder wants them.
  const even = (n: number) => Math.floor(n / 2) * 2;
  return { x: even(x), y: even(y), width: even(width), height: even(height) };
}

function encode(raw: string, start: number, length: number, crop: ReturnType<typeof frameOf>, name: string, still: "start" | "end"): void {
  const filter = `crop=${crop.width}:${crop.height}:${crop.x}:${crop.y},scale=${SIZE.width}:${SIZE.height}`;
  const video = join(OUT, `${name}.webm`);
  const poster = join(OUT, `${name}.png`);
  execFileSync(FFMPEG, ["-y", "-loglevel", "error", "-i", raw, "-ss", start.toFixed(2), "-t", length.toFixed(2), "-vf", filter, "-an", "-c:v", "libvpx-vp9", "-b:v", "0", "-crf", "40", "-deadline", "good", "-cpu-used", "2", "-row-mt", "1", video]);
  execFileSync(FFMPEG, ["-y", "-loglevel", "error", "-i", raw, "-ss", (still === "start" ? start + 0.4 : Math.max(start, start + length - 0.3)).toFixed(2), "-frames:v", "1", "-vf", filter, poster]);
}

test.beforeAll(() => {
  if (!existsSync(FFMPEG)) throw new Error(`Playwright's cached ffmpeg isn't at ${FFMPEG}; nothing is downloaded.`);
  mkdirSync(OUT, { recursive: true });
  mkdirSync(RAW, { recursive: true });
});

const ONLY = process.env.AIJ_CLIPS_ONLY?.split(",");

for (const theme of ["dark", "light"] as const) {
  for (const [id, recipe] of Object.entries(RECIPES) as [ClipId, Recipe][]) {
    if (ONLY && !ONLY.includes(id)) continue;
    test(`${id} — ${theme}`, async ({ browser }) => {
      const dir = join(RAW, `${id}-${theme}`);
      rmSync(dir, { recursive: true, force: true });
      const context = await browser.newContext({ viewport: VIEW, deviceScaleFactor: 1, colorScheme: theme, reducedMotion: "no-preference", recordVideo: { dir, size: VIEW } });
      const born = Date.now();
      const page = await context.newPage();
      await page.clock.setFixedTime(NOW);
      await page.addInitScript(
        ({ fixture, seams, t, firstRun }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          for (const [key, value] of Object.entries(seams)) w[key] = value;
          localStorage.setItem("aij.theme", t);
          if (!firstRun) {
            localStorage.setItem("aij.onboarding.dismissed", "1");
            sessionStorage.setItem("aij.ready.skipped", "1");
          }
        },
        { fixture: recipe.fixture ?? allStatuses, seams: recipe.seams ?? {}, t: theme, firstRun: recipe.firstRun ?? false },
      );
      await page.goto("/");
      if (!recipe.firstRun) await expect(page.getByTestId("user-menu-button")).toBeVisible();
      // F20 (0.5.1): a new install opens on the Welcome; Get started opens the wizard.
      else await page.getByTestId("welcome-get-started").click();
      await page.mouse.move(VIEW.width - 4, VIEW.height - 4);
      const frame = await recipe.setup(page);
      await pause(page, 500);
      const crop = frameOf(frame ? await frame.boundingBox() : null);
      const start = (Date.now() - born) / 1000;
      await recipe.play(page);
      const length = Math.min(6, Math.max(3, (Date.now() - born) / 1000 - start));
      await context.close();
      const raw = await page.video()!.path();
      encode(raw, start, length, crop, `${id}-${theme}`, recipe.poster ?? "end");
    });
  }
}
