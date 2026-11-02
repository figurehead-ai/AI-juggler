/**
 * 0.3.5 gallery (ADR-0024): a Claude Code session's conversation as the
 * Claude app draws it, followed live: every text in full with each run of
 * work as one titled line (the running one in its "-ing" form), narration,
 * a message sent mid-turn; new words arriving while the reader is
 * scrolled up (they stay put, a "New messages" pill offers the way down);
 * a problem reading the latest changes as a line over the turns. Both
 * themes, axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses } from "./fixtures";
import { ARRIVING, feedTurns } from "./fixtures/conversation-feed";

const DIR = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-conversation-feed");
const T0 = 1_728_800_000_000;

test.beforeAll(() => mkdirSync(DIR, { recursive: true }));

const session = {
  id: "feed-1",
  tool: "claude-code",
  account_id: "claude-work",
  account_label: "Work Max",
  project_id: "proj-1",
  cwd: "/Users/you/Dev/web-app",
  title: "Fix the flaky checkout test",
  git_branch: "main",
  started_ms: T0 - 3_600_000,
  last_activity_ms: T0 - 30_000,
  tokens: { input: 52_000, output: 9_400, cache_read: 210_000, cache_write: 4_100 },
  live: true,
  session_id: null,
  places: [{ app: "Claude", app_id: "claude-app", profile: "Work Max", instance: "Work", pid: 4242, label: "Claude · Work" }],
  pid: 4242,
  state: "working",
  viewed_ms: T0 - 120_000,
  background: false,
  inactive: false,
};

const FIXTURE = {
  ...allStatuses,
  conversations: [session],
  last_messages: { "feed-1": { state: "shown", redacted: 0, turns: feedTurns(T0) } },
};

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_APP_SETTINGS__ = {};
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: FIXTURE, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
  await page.getByTestId("rail").locator("[data-conversation-id]").filter({ hasText: "Fix the flaky checkout test" }).first().click();
  await expect(page.getByTestId("conversation-page")).toBeVisible();
  await expect(page.getByTestId("last-turn")).toHaveCount(4);
}

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.getByTestId("last-messages").screenshot({ path: resolve(DIR, `${name}-${theme}.png`) });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

/** Plays the feed: new words for the session (the preview's seam). */
async function arrive(page: Page, detail: Record<string, unknown>) {
  await page.evaluate((detail) => window.dispatchEvent(new CustomEvent("aij:preview-feed", { detail })), { conversationId: "feed-1", ...detail });
}

for (const theme of ["dark", "light"] as const) {
  test(`a Claude session's conversation, as the Claude app draws it — ${theme}`, async ({ page }) => {
    await open(page, theme);
    const folds = page.getByTestId("last-turn-fold-toggle");
    await expect(folds).toHaveCount(3);
    await expect(folds.nth(0)).toHaveText("Ran the checkout test 40 times");
    await expect(folds.nth(1)).toHaveText("Read 2 files, searched code");
    // The run still going says what it's doing.
    await expect(folds.nth(2)).toHaveText("Running the unit tests");
    await expect(page.getByTestId("last-turn-work-toggle")).toHaveCount(0);
    await expect(page.getByTestId("last-turn").nth(2)).toHaveAttribute("data-speaker", "you");
    await folds.nth(1).click();
    await expect(page.getByTestId("last-turn-step-lines")).toContainText("Read payment-mock.ts");
    await shoot(page, "stretches", theme);
  });

  test(`opened, the newest is in view, its run going — ${theme}`, async ({ page }) => {
    await open(page, theme);
    const scroll = page.getByTestId("last-messages-scroll");
    expect(await scroll.evaluate((box) => box.scrollHeight - box.scrollTop - box.clientHeight)).toBeLessThan(2);
    await expect(page.locator("[data-testid='last-turn-fold'][data-running='true']")).toContainText("Running the unit tests");
    await shoot(page, "running", theme);
  });

  test(`new words while scrolled up: nothing moves, a pill offers the way down — ${theme}`, async ({ page }) => {
    await open(page, theme);
    const scroll = page.getByTestId("last-messages-scroll");
    await scroll.evaluate((box) => {
      box.scrollTop = 0;
      box.dispatchEvent(new Event("scroll"));
    });
    const before = await scroll.evaluate((box) => box.scrollTop);
    await arrive(page, { turns: [{ speaker: "assistant", blocks: ARRIVING }], grow: true });
    await expect(page.getByTestId("last-messages-new")).toBeVisible();
    expect(await scroll.evaluate((box) => box.scrollTop)).toBe(before);
    await expect(page.getByTestId("last-turn")).toHaveCount(4);
    await shoot(page, "new-messages", theme);
    await page.getByTestId("last-messages-new").click();
    await expect(page.getByTestId("last-messages-new")).toBeHidden();
    await expect(page.getByTestId("last-turn").last()).toContainText("All green: 214 passed.");
  });

  test(`a problem reading the latest changes is a line over the turns — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await arrive(page, { status: "unreadable" });
    await expect(page.getByTestId("last-messages-banner")).toHaveText("Couldn't read the latest changes. Trying again.");
    await expect(page.getByTestId("last-turn")).toHaveCount(4);
    // The line takes room: the newest stays in view.
    const scroll = page.getByTestId("last-messages-scroll");
    expect(await scroll.evaluate((box) => box.scrollHeight - box.scrollTop - box.clientHeight)).toBeLessThan(2);
    await shoot(page, "banner", theme);
  });
}
