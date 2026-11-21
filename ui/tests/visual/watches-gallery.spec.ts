/**
 * AJ-E3816 ("Visual testing"): the Watches view in every state of the
 * approved mock-up (`?s=`): the list with a row's menu open, New watch
 * with the picker open, Edit watch with a second session added, no
 * watches yet, Ended with an Undo after a removal, and at the limit of
 * 50 once New watch is pressed; plus the doors (a watch alert's Edit
 * watch and the notification centre's link). Both themes, at 1280 and
 * 1024, each frame tall enough for the whole page; then axe on each (0
 * critical or serious). Mocked data only: nothing is written.
 *
 * Captures go to `AIJ_WATCHES_OUT` when it names a folder; without it
 * (the a11y gate in CI) the states are walked and scanned, nothing written.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { STATES, WATCHES_CATALOGUE, WATCHES_FIXTURE, WATCHES_JOBS, WATCHES_NOW, WATCHES_POOLS, WATCHES_STORED_APPS, watchesTools, type WatchesPreviewState } from "../../src/preview/watches";

const OUT = process.env.AIJ_WATCHES_OUT ? resolve(process.env.AIJ_WATCHES_OUT) : null;

test.beforeAll(() => {
  if (OUT) mkdirSync(OUT, { recursive: true });
});

async function open(page: Page, state: WatchesPreviewState, theme: "dark" | "light", width: number, extra: Record<string, unknown> = {}, height = 800): Promise<void> {
  await page.addInitScript(
    ({ fixture, theme: t, now, watches, catalogue, jobs, pools, tools, storedApps, extra: more }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_TOOLS__ = tools;
      w.__AIJ_RECIPES__ = (fixture as { profiles: unknown }).profiles;
      w.__AIJ_STORED_APPS__ = storedApps;
      w.__AIJ_WATCH_VIEW__ = { watches, catalogue, calls: [] };
      w.__AIJ_AI_JOBS__ = jobs;
      w.__AIJ_POOLS__ = pools;
      for (const [key, value] of Object.entries(more)) w[key] = value;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      const real = Date.now;
      const start = real();
      // A steady clock that still moves (toasts and debounces count).
      Date.now = () => now + (real() - start);
    },
    { fixture: WATCHES_FIXTURE, theme, now: WATCHES_NOW, watches: state.watches, catalogue: WATCHES_CATALOGUE, jobs: WATCHES_JOBS, pools: WATCHES_POOLS, tools: watchesTools(), storedApps: WATCHES_STORED_APPS, extra },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height });
  await page.goto("/");
  // The shell listens once it has drawn: ask until the page opens.
  await expect
    .poll(async () => {
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "watches" } })));
      return page.getByTestId("watches-page").or(page.getByTestId("watch-form")).count();
    })
    .toBeGreaterThan(0);
  await expect(page.getByTestId("watches-skeleton")).toHaveCount(0);
}

/** As tall as the page (and `room` more), so one frame holds all of it. */
async function wholePage(page: Page, width: number, room = 0): Promise<void> {
  const extra = await page.evaluate(() => {
    const scroller = document.querySelector("[data-testid=watches-page]") ?? document.querySelector("[data-testid=flow-scroll]");
    return scroller ? scroller.scrollHeight - scroller.clientHeight : 0;
  });
  const height = page.viewportSize()?.height ?? 800;
  await page.setViewportSize({ width, height: Math.max(800, height + Math.max(extra, 0) + room) });
}

async function axeClean(page: Page, what: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`), `axe on ${what}`).toEqual([]);
}

/** Each state: what to do before the capture, and how much room below. */
const STEPS: Record<string, { before?: (page: Page) => Promise<void>; after?: (page: Page) => Promise<void>; room?: number; fixed?: boolean }> = {
  list: {
    after: async (page) => {
      await page.getByTestId("watch-more").first().click();
      await expect(page.getByTestId("watch-menu")).toBeVisible();
    },
    room: 40,
  },
  create: {
    before: async (page) => {
      await page.getByTestId("watches-new").click();
      await expect(page.getByTestId("watch-form")).toBeVisible();
    },
    after: async (page) => {
      await page.getByTestId("watch-target-add").click();
      await page.locator("[data-testid=watch-pick-option][data-key='session:s-api']").first().click();
      await expect(page.getByTestId("watch-preview-echo")).toContainText("finishes");
      await page.getByTestId("watch-picker-search").focus();
    },
    fixed: true,
  },
  edit: {
    before: async (page) => {
      await page.getByTestId("watch-edit").first().click();
      await expect(page.getByTestId("watch-form")).toBeVisible();
      await page.getByTestId("watch-target-add").click();
      await page.locator("[data-testid=watch-pick-option][data-key='session:s-web']").first().click();
      await page.keyboard.press("Escape");
      await expect(page.getByTestId("watch-preview-was")).toBeVisible();
    },
  },
  empty: {
    before: async (page) => {
      await expect(page.getByTestId("watch-starter")).toHaveCount(3);
    },
  },
  ended: {
    before: async (page) => {
      await page.getByTestId("watches-ended-fold").click();
      await expect(page.getByTestId("watch-ended-row")).toHaveCount(5);
    },
    after: async (page) => {
      await page.locator("[data-watch-id='w-x1']").getByTestId("watch-more").click();
      await page.getByTestId("watch-menu-remove").click();
      await expect(page.getByTestId("watch-undo-toast")).toBeVisible();
    },
  },
  many: {
    before: async (page) => {
      await page.getByTestId("watches-new").click();
      await expect(page.getByTestId("watches-limit")).toBeVisible();
    },
  },
};

for (const theme of ["dark", "light"] as const) {
  for (const [name, state] of Object.entries(STATES)) {
    for (const width of [1280, 1024]) {
      test(`watches ${name} — ${theme} — ${width}`, async ({ page }) => {
        const step = STEPS[name] ?? {};
        // The picker open over the form, in the mock's 900 px frame.
        await open(page, state, theme, width, {}, name === "create" ? 900 : 800);
        await step.before?.(page);
        if (!step.fixed) await wholePage(page, width, step.room ?? 0);
        await step.after?.(page);
        if (OUT) await page.screenshot({ path: `${OUT}/${name}-${theme}-${width}.png` });
        if (width === 1280) await axeClean(page, `${name} ${theme}`);
      });
    }
  }
}

/** The doors: a watch alert's Edit watch, and the centre's Watches link. */
const ALERT = {
  app: [],
  earlier: [
    {
      id: 41,
      kind: "watch",
      needs: false,
      title: "“api” finished",
      detail: "Your watch",
      project_id: "proj-shop",
      project: "shop",
      target: { session_id: "s-api", conversation_id: null, account_id: null, project_id: "proj-shop", page: null, task_id: null, pool_id: null },
      facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null, told: "finished" },
      action: "open",
      meter: null,
      created_ms: WATCHES_NOW - 12 * 60_000,
      unread: true,
      subject: "watch:w-a1:s-api:finishes:1795121000000",
      snoozed: false,
      snoozed_until_ms: null,
      muted: false,
      dismissed: false,
    },
  ],
};

for (const theme of ["dark", "light"] as const) {
  for (const width of [1280, 1024]) {
    test(`watches doors — ${theme} — ${width}`, async ({ page }) => {
      await open(page, STATES.list!, theme, width, { __AIJ_NOTICES__: ALERT });
      await page.locator("[data-notification-bell]").first().click();
      await expect(page.getByTestId("notification-centre")).toBeVisible();
      await expect(page.getByTestId("notices-watches")).toBeVisible();
      await expect(page.getByTestId("notice-edit-watch")).toBeVisible();
      if (OUT) await page.screenshot({ path: `${OUT}/doors-${theme}-${width}.png` });
      if (width === 1280) await axeClean(page, `doors ${theme}`);
      await page.getByTestId("notice-edit-watch").click();
      await expect(page.getByTestId("watch-form")).toHaveAttribute("data-mode", "edit");
    });
  }
}

/** The App page's door: an App watched by its App id shows "Watched · 1". */
for (const theme of ["dark", "light"] as const) {
  for (const width of [1280, 1024]) {
    test(`watches app-page — ${theme} — ${width}`, async ({ page }) => {
      await open(page, STATES.list!, theme, width);
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-app", { detail: { id: "cli:acct-home" } })));
      await expect(page.getByTestId("watch-door")).toHaveText(/Watched · 1/);
      if (OUT) await page.screenshot({ path: `${OUT}/app-page-${theme}-${width}.png` });
      if (width === 1280) await axeClean(page, `app page ${theme}`);
      await page.getByTestId("watch-door").click();
      await expect(page.getByTestId("watches-only")).toHaveText(/Agent app two/);
    });
  }
}
