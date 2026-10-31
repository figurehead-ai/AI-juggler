/**
 * AJ-E805 — the interface at the scale the product promises (10
 * Accounts, 50 Instances, 20 Projects): rail render, inspector
 * selection-to-paint, search and the Cleanup inventory, each timed from
 * the click to the painted result (median of five), against the SPEC
 * budgets. The measured table is written to the AJ-E805 evidence.
 *
 * Regression rule (AJ-E805 Req.4): each median is also held to the
 * committed baseline (`load-baseline.json`) — more than 10 % slower
 * fails, with a 5 ms floor so sub-10 ms readings don't fail on timer
 * jitter, unless the baseline lists a waiver with a task ID and an
 * unexpired date. The baseline is the reference Mac's, so that check —
 * and writing the evidence — runs only under `just load-gate`
 * (`AJ_LOAD_GATE=1`); every other run still holds the SPEC budgets.
 * `AJ_LOAD_BASELINE=update` rewrites the baseline (a baseline change
 * goes in its own PR, explained).
 */
import { expect, test, type Page } from "@playwright/test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { atScale, goTo, expectRail, openRailList } from "./fixtures";

const T0 = 1_728_800_000_000;
const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E805");
const ROWS = '[data-testid="instance-board"] [data-instance-id]';
const results: { name: string; budget_ms: number; median_ms: number; runs: number[] }[] = [];
const BASELINE = resolve(process.cwd(), "tests/visual/load-baseline.json");
type Baseline = { medians: Record<string, number>; waivers: { name: string; task: string; expires: string; reason: string }[]; notes?: Record<string, string> };
const baseline: Baseline = existsSync(BASELINE)
  ? (JSON.parse(readFileSync(BASELINE, "utf8")) as Baseline)
  : { medians: {}, waivers: [] };
const updating = process.env.AJ_LOAD_BASELINE === "update";
const gate = updating || process.env.AJ_LOAD_GATE === "1";

async function open(page: Page, theme: "dark" | "light" = "dark") {
  await page.addInitScript(
    ({ fixture, t0, theme }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      // AJ-E2104: the 50-row board is the Project lens (every row drawn);
      // the place lens draws only what is in view.
      localStorage.setItem("aij.rail.view.v1", JSON.stringify({ group: "project" }));
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: atScale, t0: T0, theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await expect(page.getByTestId("user-menu-button")).toBeVisible();
}

/** Click, then wait until `done` holds; the browser clock measures it. */
async function timed(page: Page, act: () => Promise<void>, done: () => Promise<void>): Promise<number> {
  const before = await page.evaluate(() => performance.now());
  await act();
  await done();
  return (await page.evaluate(() => performance.now())) - before;
}

/** Opens the board; its row count is taken once, then expected every time. */
let rowCount = 0;
async function boardRows(page: Page): Promise<number> {
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await expect(page.locator(ROWS).first()).toBeVisible();
  if (rowCount === 0) {
    await page.waitForTimeout(300);
    rowCount = await page.locator(ROWS).count();
    console.log(`[perf] board paints ${rowCount} rows`);
    expect(rowCount).toBeGreaterThanOrEqual(30);
  }
  return rowCount;
}

/** The Project name an Instance of the fixture belongs to. */
function projectOf(id: string): string {
  const session = atScale.sessions.find((s) => s.id === id);
  return atScale.projects.find((p) => p.id === session?.project_id)!.name;
}

function record(name: string, budget: number, runs: number[]) {
  const sorted = [...runs].sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)]!;
  results.push({ name, budget_ms: budget, median_ms: Math.round(median), runs: runs.map(Math.round) });
  console.log(`[perf] ${name}: median ${median.toFixed(0)} ms (budget ${budget} ms) — ${runs.map((r) => r.toFixed(0)).join(", ")}`);
  expect(median, name).toBeLessThan(budget);
  const before = baseline.medians[name];
  const waiver = baseline.waivers.find((w) => w.name === name && Date.parse(w.expires) > Date.now());
  if (gate && !updating && before !== undefined && !waiver) {
    const limit = Math.max(before * 1.1, before + 5);
    expect(median, `${name}: regressed past the baseline ${before} ms (limit ${limit.toFixed(1)} ms)`).toBeLessThanOrEqual(limit);
  }
}

test.describe.configure({ mode: "serial" });

test("rail renders 50 Instances in under 300 ms", async ({ page }) => {
  await open(page);
  const n = await boardRows(page);
  const runs: number[] = [];
  // AJ-E2104: the repaint measured is a search that matches nothing,
  // cleared: every session back on screen.
  const search = page.getByRole("searchbox", { name: /^Search sessions/ });
  for (let i = 0; i < 5; i++) {
    await search.fill("matches-nothing");
    runs.push(
      await timed(
        page,
        () => search.fill(""),
        () => expect(page.locator(ROWS)).toHaveCount(n),
      ),
    );
  }
  record("rail: painting all 50 sessions (a search cleared)", 300, runs);
});

test("inspector paints a selection in under 200 ms", async ({ page }) => {
  await open(page);
  const n = await boardRows(page);
  await expectRail(page); // D-25: the list, or its icon column when narrow
  const rows = page.locator(ROWS);
  await expect(rows).toHaveCount(n);
  const runs: number[] = [];
  let previous = "";
  for (let i = 0; i < 5; i++) {
    // Start from a different Instance each time, so the paint is a real change.
    const row = rows.nth(i * 7 + 3);
    const id = (await row.getAttribute("data-instance-id")) ?? "";
    const project = projectOf(id);
    expect(project, "each run must change the inspected Project").not.toBe(previous);
    previous = project;
    runs.push(
      await timed(
        page,
        () => row.click(),
        () => expect(page.getByTestId("right-inspector")).toContainText(`project:${project}`),
      ),
    );
  }
  record("inspector: select an Instance to its painted Overview", 200, runs);
});

test("search filters 50 Instances in under 200 ms", async ({ page }) => {
  await open(page);
  const n = await boardRows(page);
  await expectRail(page); // D-25: the list, or its icon column when narrow
  const search = page.getByRole("searchbox").or(page.getByPlaceholder("Search ⌘K")).first();
  const runs: number[] = [];
  for (let i = 0; i < 5; i++) {
    await search.fill("");
    await expect(page.locator(ROWS)).toHaveCount(n);
    runs.push(
      await timed(
        page,
        () => search.fill(`Account 0${i}`),
        () =>
          expect(page.locator(ROWS)).not.toHaveCount(n),
      ),
    );
  }
  record("search: type to a filtered board", 200, runs);
});

// AJ-E2404: the Processes page took Cleanup's place; its first paint (the
// tree's first rows) keeps the inventory's budget and baseline key.
test("the Processes page paints its first rows in under 300 ms", async ({ page }) => {
  await open(page);
  const n = await boardRows(page);
  const runs: number[] = [];
  // Processes sits under More until it's needed (AJ-E1533, after this
  // baseline): the menu opens before the clock starts, so the timing is
  // the click on Processes to its painted rows.
  const main = page.getByRole("navigation", { name: "Main" });
  const cleanup = async () => {
    if ((await main.getByRole("button", { name: "Processes", exact: true }).count()) > 0) {
      return main.getByRole("button", { name: "Processes", exact: true }).first();
    }
    // AJ-E2507: Processes is in your menu now.
    await page.getByTestId("user-menu-button").click();
    return page.getByRole("menu", { name: "Your menu" }).getByRole("menuitem", { name: /^Processes\b/ });
  };
  for (let i = 0; i < 5; i++) {
    await goTo(page, "Home");
    const item = await cleanup();
    await item.waitFor();
    runs.push(
      await timed(
        page,
        () => item.click(),
        () => expect(page.locator('[data-testid^="process-row-"]').first()).toBeVisible(),
      ),
    );
  }
  record("cleanup: open the inventory", 300, runs);
});

test("the webview paints the rail at scale in under 2.5 s", async ({ browser }) => {
  const runs: number[] = [];
  for (let i = 0; i < 5; i++) {
    // A fresh context each run: nothing cached from the previous load.
    const context = await browser.newContext();
    const page = await context.newPage();
    await open(page);
    // The rail's first project header (Home also lists recent sessions).
    await expect(page.getByTestId("rail-project").filter({ hasText: /^project-00/ }).first()).toBeVisible();
    runs.push(await page.evaluate(() => performance.now()));
    await context.close();
  }
  record("cold start: webview navigation to painted rail", 2500, runs);
});

test("the board scrolls to its last Instance in under 300 ms", async ({ page }) => {
  await open(page);
  await boardRows(page);
  const rows = page.locator(`${ROWS}:visible`);
  const n = await rows.count();
  console.log(`[perf] ${n} rows visible without expanding a group`);
  const runs: number[] = [];
  for (let i = 0; i < 5; i++) {
    await rows.first().evaluate((el) => el.scrollIntoView({ block: "start" }));
    await expect(rows.first()).toBeInViewport();
    runs.push(
      await timed(
        page,
        () => rows.nth(n - 1).evaluate((el) => el.scrollIntoView({ block: "end" })),
        () => expect(rows.nth(n - 1)).toBeInViewport(),
      ),
    );
  }
  record("rail: scroll the board to its last Instance", 300, runs);
});

for (const theme of ["dark", "light"] as const) {
  for (const [width, height] of [
    [1440, 900],
    [1024, 680],
  ] as const) {
    test(`rail and inspector hold their layout at scale — ${theme} ${width}x${height}`, async ({ page }) => {
      await open(page, theme);
      await page.setViewportSize({ width, height });
      await expectRail(page); // D-25: narrower than 1100 px the rail folds to icons
      await openRailList(page); // and its own door opens the list (the board)
      await expect(page.locator(ROWS).first()).toBeVisible();
      const row = page.locator(ROWS).nth(3);
      const id = (await row.getAttribute("data-instance-id")) ?? "";
      await row.click();
      await expect(page.getByTestId("right-inspector")).toContainText(`project:${projectOf(id)}`);
      // No layout collapse: nothing pushes the window sideways, rows and
      // the inspector both have room.
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(overflow).toBeLessThanOrEqual(0);
      const box = await page.locator(ROWS).first().boundingBox();
      const inspector = await page.getByTestId("right-inspector").boundingBox();
      expect(box!.width).toBeGreaterThan(150);
      expect(inspector!.width).toBeGreaterThan(280);
      await expect(page.locator('[aria-busy="true"]')).toHaveCount(0);
      if (gate) {
        mkdirSync(resolve(OUT, "visual"), { recursive: true });
        await page.screenshot({ path: resolve(OUT, "visual", `load-${theme}-${width}x${height}.png`) });
      }
    });
  }
}

test.afterAll(() => {
  mkdirSync(OUT, { recursive: true });
  if (!gate || results.length === 0) return;
  writeFileSync(resolve(OUT, "ui-load.json"), JSON.stringify({ fixture: "atScale: 10 Accounts, 50 Instances, 20 Projects", results }, null, 2) + "\n");
  if (updating) {
    const medians = Object.fromEntries(results.map((r) => [r.name, r.median_ms]));
    writeFileSync(BASELINE, JSON.stringify({ ...baseline, medians: { ...baseline.medians, ...medians } }, null, 2) + "\n");
  }
});
