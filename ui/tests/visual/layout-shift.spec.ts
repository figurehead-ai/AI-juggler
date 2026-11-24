/**
 * AJ-E1551 — nothing moves after it appears. Cumulative layout shift,
 * measured by the browser (Chromium's `layout-shift` entries), on the
 * three moments the maintainer named: opening Home, opening a session
 * (the subagent tree used to arrive a moment later and push the page
 * over), and switching a session's tabs. Every shift counts here, even
 * one right after the click (the browser's own CLS forgives those; the
 * eye doesn't): the budget is about 0.
 *
 * A session the rail knows has subagents (the fixture's sess-1) has its
 * tree's column reserved from the first frame; the others have none.
 * Writes the numbers to `docs/qa/evidence/AJ-E1551/layout-shift.json`.
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, openRailSession, railViewAs } from "./fixtures";
import { knownPlacesPreview, placesConversations } from "../../src/preview/places";

const T0 = 1_728_800_000_000;
/** About zero: a shift a person could notice is ~0.05 and more. */
const BUDGET = 0.01;
const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E1551");
const results: Record<string, { cls: number; shifts: number; worst: string | null }> = {};

async function seed(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      (window as unknown as Record<string, unknown>).__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
      // Every shift from the first paint on, with what moved.
      const store = { cls: 0, shifts: 0, worst: null as string | null, worstValue: 0 };
      (window as unknown as Record<string, unknown>).__AIJ_SHIFTS__ = store;
      new PerformanceObserver((list) => {
        for (const entry of list.getEntries() as unknown as Array<{ value: number; sources?: Array<{ node?: Node | null }> }>) {
          store.cls += entry.value;
          store.shifts += 1;
          if (entry.value > store.worstValue) {
            store.worstValue = entry.value;
            const node = entry.sources?.[0]?.node;
            const el = node instanceof Element ? node : node?.parentElement;
            store.worst = el ? `${el.tagName.toLowerCase()}${el.getAttribute("data-testid") ? `[data-testid=${el.getAttribute("data-testid")}]` : ""} ${(el.textContent ?? "").trim().slice(0, 40)}` : null;
          }
        }
      }).observe({ type: "layout-shift", buffered: true });
    },
    { fixture: allStatuses, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
}

/** Settles (two frames and a quiet moment), then reads and resets the count. */
async function shifts(page: Page): Promise<{ cls: number; shifts: number; worst: string | null }> {
  await page.waitForTimeout(800);
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  return page.evaluate(() => {
    const store = (window as unknown as { __AIJ_SHIFTS__: { cls: number; shifts: number; worst: string | null; worstValue: number } }).__AIJ_SHIFTS__;
    const out = { cls: Math.round(store.cls * 10_000) / 10_000, shifts: store.shifts, worst: store.worst };
    Object.assign(store, { cls: 0, shifts: 0, worst: null, worstValue: 0 });
    return out;
  });
}

/** AJ-E1554 R1: what each rail row's parts measured at rest and under the pointer. */
const HOVER_OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E1554");
const hovers: Record<string, { cls: number; moved: string[] }> = {};

test.afterAll(() => {
  mkdirSync(OUT, { recursive: true });
  writeFileSync(resolve(OUT, "layout-shift.json"), JSON.stringify({ budget: BUDGET, viewport: "1280x800", results }, null, 2) + "\n");
  if (Object.keys(hovers).length === 0) return;
  mkdirSync(HOVER_OUT, { recursive: true });
  writeFileSync(resolve(HOVER_OUT, "hover-hold-still.json"), JSON.stringify({ budget: BUDGET, viewport: "1280x800", hovers }, null, 2) + "\n");
});

/** Each part's box, by its test id, inside `row`. */
async function boxes(row: ReturnType<Page["locator"]>, parts: string[]): Promise<Record<string, string>> {
  const out: Record<string, string> = {};
  for (const part of parts) {
    const box = await row.locator(part).first().boundingBox();
    out[part] = box ? [box.x, box.y, box.width, box.height].map((n) => Math.round(n * 10) / 10).join(",") : "none";
  }
  return out;
}

/** Measures `row`'s parts at rest, under the pointer and under the
 * keyboard; none may move and nothing may shift. */
async function holdsStill(page: Page, name: string, row: ReturnType<Page["locator"]>, parts: string[]): Promise<void> {
  await expect(row).toBeVisible();
  const rest = await boxes(row, parts);
  // Under the pointer.
  await row.hover();
  const hovered = await boxes(row, parts);
  const hover = await shifts(page);
  // Under the keyboard (focus on its own way in).
  await page.mouse.move(0, 0);
  await row.locator("button").first().focus();
  await page.keyboard.press("Shift+Tab");
  await page.keyboard.press("Tab");
  const focused = await boxes(row, parts);
  const focus = await shifts(page);
  const moved = parts.filter((part) => hovered[part] !== rest[part] || focused[part] !== rest[part]);
  hovers[name] = { cls: Math.max(hover.cls, focus.cls), moved };
  expect(moved, `${name}: moved under the pointer or focus`).toEqual([]);
  expect(Math.max(hover.cls, focus.cls), `${name}: shifted (${hover.worst ?? focus.worst})`).toBeLessThanOrEqual(BUDGET);
  await page.mouse.move(0, 0);
  await page.locator("body").focus().catch(() => undefined);
}

for (const theme of ["dark", "light"] as const) {
  test(`opening Home holds still (${theme})`, async ({ page }) => {
    await seed(page, theme);
    // A first run has nothing kept yet; Home's places are sized from the
    // last look (D-103.6, kept in local storage). Opening Home is the
    // second launch's first frame, as the user opens the app each day.
    await page.goto("/");
    await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Home", exact: true }).waitFor();
    await shifts(page);
    await page.reload();
    await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Home", exact: true }).waitFor();
    const home = await shifts(page);
    results[`home-open-${theme}`] = home;
    expect(home.cls, `Home moved after it appeared: ${home.worst}`).toBeLessThanOrEqual(BUDGET);
  });

  test(`opening a session and switching its tabs hold still (${theme})`, async ({ page }) => {
    await seed(page, theme);
    await page.goto("/");
    await expect(page.getByTestId("rail")).toBeVisible();
    await shifts(page);
    for (const id of ["sess-1", "sess-2"]) {
      await openRailSession(page, id);
      const opened = await shifts(page);
      results[`session-open-${id}-${theme}`] = opened;
      expect(opened.cls, `opening ${id} moved: ${opened.worst}`).toBeLessThanOrEqual(BUDGET);
    }
    const tabs = page.getByRole("tablist").first().getByRole("tab");
    const count = await tabs.count();
    for (let n = 1; n < count; n += 1) {
      const name = (await tabs.nth(n).textContent())?.trim() ?? String(n);
      await tabs.nth(n).click();
      const switched = await shifts(page);
      results[`session-tab-${name}-${theme}`] = switched;
      expect(switched.cls, `the ${name} tab moved: ${switched.worst}`).toBeLessThanOrEqual(BUDGET);
    }
    // Back to Home from a session.
    await goTo(page, "Home");
    const back = await shifts(page);
    results[`home-from-session-${theme}`] = back;
    expect(back.cls, `Home moved after it appeared: ${back.worst}`).toBeLessThanOrEqual(BUDGET);
  });

  test(`a rail row, an App header and a Project lens row hold still under the pointer and the keyboard (${theme})`, async ({ page }) => {
    await seed(page, theme);
    await page.goto("/");
    const board = page.getByTestId("place-board");
    await expect(board).toBeVisible();
    await shifts(page);
    const cases: Array<[string, ReturnType<Page["locator"]>, string[]]> = [
      ["session-row", board.locator("[data-rail-row]").filter({ has: page.getByTestId("row-title") }).first(), ["[data-testid='row-title']", "[data-state-dot]", "[data-testid='row-end']", "[data-testid='row-slots']"]],
      ["app-header", board.getByTestId("place-header").first(), ["[data-testid='place-page']", "[data-testid='place-name-line']"]],
      // 0.5.1 F42: a Needs you row's tools at its why line's end; its title, why and where stay put.
      ["needs-row", page.getByTestId("rail").locator("[data-rail-row]").filter({ has: page.getByTestId("need-word") }).first(), ["[data-testid='place-mark']", "[data-testid='need-word']", "[data-testid='row-end']", "[data-testid='row-slots']", "[data-testid='row-slots'] span.truncate.font-semibold"]],
    ];
    for (const [name, row, parts] of cases) await holdsStill(page, `${name}-${theme}`, row, parts);
    // 0.5.1, F11: the Project lens's row, its ⋯ at the row's end (AJ-E1554 R1).
    await railViewAs(page, { group: "project" });
    await shifts(page);
    // Its own row (the state mark first), not a Needs you row drawn by SessionRow.
    const projectRow = page.getByTestId("rail-list").locator("[data-rail-row]:has(> button[data-instance-id] > span[aria-hidden='true'])").first();
    await holdsStill(page, `project-row-${theme}`, projectRow, ["[data-instance-id]", "[data-instance-id] > span[aria-hidden]", "[data-testid='row-tools']", "button[aria-label^='Session actions']"]);
  });

  // 0.5.1 F22: the logo sat under the window's traffic lights (macOS draws
  // them over the top 28 px, to about 78 px from the left), and folded
  // Inactive apps wrapped onto two lines with its summary cut
  // ("5 apps · 16 sessi…"), at the rail's default width and its narrowest.
  test(`the rail's head clears the traffic lights and folded Inactive apps holds one line (${theme})`, async ({ page }) => {
    await seed(page, theme);
    await page.goto("/");
    const head = page.getByTestId("rail-header");
    await expect(head).toBeVisible();
    const logo = await head.getByRole("button").first().boundingBox();
    expect(logo?.x ?? 0, "the logo starts past the traffic lights").toBeGreaterThanOrEqual(78);
    // Apps with nothing open: the places desktop with its sessions ended.
    await page.addInitScript(
      ({ known, conversations }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_PLACES__ = known;
        w.__AIJ_FIXTURE__ = { ...(w.__AIJ_FIXTURE__ as object), conversations };
      },
      { known: knownPlacesPreview(T0), conversations: placesConversations(T0).map((row) => ({ ...row, live: false, places: [], pid: null, state: null })) },
    );
    await page.reload();
    const idle = page.getByTestId("place-board").locator("div:has(> [data-section='idle'])");
    await expect(idle).toBeVisible();
    const fold = idle.getByTestId("rail-section-fold");
    if ((await fold.getAttribute("aria-expanded")) === "true") await fold.click();
    const summary = idle.getByTestId("section-summary");
    await expect(summary).toBeVisible();
    for (const width of [264, 208]) {
      await page.evaluate((w) => {
        const rail = document.querySelector<HTMLElement>("[data-testid='place-board']")?.closest<HTMLElement>("[style*='width']");
        if (rail) rail.style.width = `${w}px`;
      }, width);
      await page.waitForTimeout(300);
      const line = await idle.evaluate((element) => {
        const button = element.querySelector<HTMLElement>("[data-testid='rail-section-fold']")!;
        const words = element.querySelector<HTMLElement>("[data-testid='section-summary']")!;
        const height = parseFloat(getComputedStyle(button).lineHeight) || 16;
        return { lines: Math.round(button.getBoundingClientRect().height / height), cut: words.scrollWidth > words.clientWidth + 0.5, inside: words.getBoundingClientRect().right <= button.getBoundingClientRect().right + 0.5 };
      });
      expect(line, `at ${width} px: one line, the summary whole`).toEqual({ lines: 1, cut: false, inside: true });
    }
  });
}
