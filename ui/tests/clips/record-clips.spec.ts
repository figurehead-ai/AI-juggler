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
    setup: async (page) => main(page),
    play: async (page) => {
      await page.getByTestId("rail").locator("button[data-instance-id]").first().click().catch(() => undefined);
      await pause(page, 900);
      const go = main(page).getByRole("button", { name: /Go to it|Open/ }).first();
      await go.hover().catch(() => undefined);
      await pause(page, 800);
      await go.click().catch(() => undefined);
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
