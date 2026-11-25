/**
 * AJ-E1552: nothing loads forever. Every screen of the header and your
 * menu, each Accounts filter and tab, and every Settings pane is opened in
 * turn. Each must have no skeleton left on screen within 15 s of opening
 * (a loading state's deadline is 12 s, after which it is an error with
 * Retry, not a skeleton). A skeleton is any `bg-skeleton` element, a
 * placeholder row (`aria-busy`), or a `LoadingState` still loading.
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses } from "./fixtures";

const T0 = 1_728_800_000_000;
/** The longest a skeleton may stay on screen. */
const LIMIT_MS = 15_000;

const SCREENS = ["projects", "pools", "tasks", "floor", "history", "accounts", "usage", "fleet", "cleanup", "thismac", "watches", "bot", "settings"] as const;

/** The skeletons on screen now, each named by its nearest test id. */
function skeletonsNow(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const shown = (el: Element) => {
      const box = el.getBoundingClientRect();
      if (box.width === 0 || box.height === 0) return false;
      const style = getComputedStyle(el);
      return style.visibility !== "hidden" && style.display !== "none" && Number(style.opacity) > 0;
    };
    const found = [...document.querySelectorAll('.bg-skeleton, [aria-busy="true"], [data-testid="loading-state"][data-state="loading"]')].filter(shown);
    return found.map((el) => el.closest("[data-testid]")?.getAttribute("data-testid") ?? el.tagName.toLowerCase());
  });
}

/** Waits until no skeleton is on screen; fails naming those left after LIMIT_MS. */
async function settles(page: Page, where: string): Promise<number> {
  const start = Date.now();
  let left: string[] = [];
  while (Date.now() - start < LIMIT_MS) {
    left = await skeletonsNow(page);
    if (left.length === 0) return Date.now() - start;
    await page.waitForTimeout(250);
  }
  expect(left, `${where}: skeletons still on screen after ${LIMIT_MS / 1000} s`).toEqual([]);
  return LIMIT_MS;
}

async function open(page: Page) {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      Date.now = () => t0;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

const screen = (page: Page, id: string, filter?: string) =>
  page.evaluate(({ id, filter }) => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: filter ? { id, filter } : { id } })), { id, filter });

test.describe("AJ-E1552: no skeleton stays", () => {
  test.setTimeout(10 * 60_000);

  test("every screen, filter and Settings pane settles within 15 s", async ({ page }) => {
    await open(page);
    const took: Record<string, number> = { home: await settles(page, "Home") };
    for (const id of SCREENS) {
      await screen(page, id);
      took[id] = await settles(page, id);
    }
    // Accounts: each filter, and its Accounts and Apps tabs.
    for (const filter of ["out", "low"]) {
      await screen(page, "accounts", filter);
      took[`accounts:${filter}`] = await settles(page, `Accounts, filter ${filter}`);
    }
    await screen(page, "accounts");
    for (const tab of [/^Accounts/, /^Apps/]) {
      const radio = page.getByRole("radio", { name: tab });
      if ((await radio.count()) === 0) continue;
      await radio.first().click();
      took[`accounts:${String(tab)}`] = await settles(page, `Accounts, tab ${String(tab)}`);
    }
    // Every Settings pane.
    await screen(page, "settings");
    const nav = page.getByRole("navigation", { name: "Settings" });
    await expect(nav).toBeVisible();
    const panes = await nav.getByRole("button").allInnerTexts();
    expect(panes.length).toBeGreaterThan(5);
    for (const pane of panes) {
      await nav.getByRole("button", { name: pane, exact: true }).first().click();
      took[`settings:${pane}`] = await settles(page, `Settings › ${pane}`);
    }
    test.info().annotations.push({ type: "settled", description: JSON.stringify(took) });
  });
});
