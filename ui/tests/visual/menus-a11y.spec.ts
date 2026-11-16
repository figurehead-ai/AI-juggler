/**
 * AJ-E3011 round 4 (main's ruling, app-wide): every menu and popover the
 * app opens, on every screen, keeps a press for one of its options on that
 * option. In the Dev window an accessibility press on "Its role" › Reviewer
 * reached "Take out of the pool" under the open menu.
 *
 * The rail's and header's (on every screen) once, then each screen's own
 * content, every Settings pane, and a pool's page (its diagram, an App
 * selected, its settings): every visible button that opens a menu or
 * popover (`aria-haspopup`) is opened in turn, a popover's own menus one
 * level down, and while it's open:
 * - its options are exposed as menu items or options (a dialog: anything);
 * - a hit at the middle of each option lands inside it (no content under
 *   it takes the press);
 * - the button that opened it, and everything else under it, is inert;
 * then Escape closes it, and nothing it made inert stays so. What was
 * covered is written to docs/qa/evidence/AJ-E3011/menus-a11y.json.
 */
import { type Locator, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, GATE_NOW, test } from "./axe-gallery";
import { goTo, settingsPane } from "./fixtures";
import { canvasSeed } from "./fixtures/pool-canvas";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E3011");
const covered: { screen: string; opener: string; kind: string; options: number }[] = [];

async function open(page: Page, seed: Record<string, unknown> = canvasSeed(GATE_NOW)) {
  await page.clock.setFixedTime(GATE_NOW);
  await page.addInitScript((seed) => {
    Object.assign(window, seed);
    localStorage.setItem("aij.theme", "dark");
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
  }, seed);
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

/** Open menus and popovers (a menu inside a menu counts once). */
const PANELS = '[role="menu"]:not([role="menu"] [role="menu"]), dialog[open], [role="dialog"]';
const OPENERS = '[aria-haspopup="menu"], [aria-haspopup="dialog"], [aria-haspopup="listbox"], [aria-haspopup="true"]';

/** Opens each button in `within` that opens a menu or popover, checks it,
 * closes it. In an open popover (`nested`), its own menus are checked the
 * same way, one level down. */
async function auditScreen(page: Page, screen: string, within: Locator, nested = false): Promise<void> {
  const root = nested ? within : undefined;
  const openers = within.locator(OPENERS);
  const count = await openers.count();
  for (let at = 0; at < count; at += 1) {
    const opener = openers.nth(at);
    if (!(await opener.isVisible()) || !(await opener.isEnabled())) continue;
    const name = ((await opener.getAttribute("aria-label")) ?? (await opener.textContent()) ?? "").trim().slice(0, 60);
    const inertBefore = await page.locator("[inert]").count();
    const panelsBefore = await page.locator(PANELS).count();
    await opener.click();
    const newest = page.locator(PANELS).last();
    if ((await page.locator(PANELS).count()) <= panelsBefore || !(await newest.isVisible())) {
      // It opened something else (a page, a sheet), or nothing: back out.
      await page.keyboard.press("Escape");
      continue;
    }
    // Pinned: "the newest" changes as menus open and close inside it.
    const mark = root ? "inner" : "outer";
    await newest.evaluate((el, value) => el.setAttribute("data-audit-panel", value), mark);
    const panel = page.locator(`[data-audit-panel="${mark}"]`);
    const kind = (await panel.getAttribute("role")) ?? (await panel.evaluate((el) => el.tagName.toLowerCase()));
    const options = panel.locator('[role="menuitem"], [role="menuitemradio"], [role="menuitemcheckbox"], [role="option"]');
    const n = await options.count();
    if (kind === "menu") expect(n, `${screen} › ${name}: a menu exposes its options as menu items`).toBeGreaterThan(0);
    // A press at each option's middle lands on it, never under it.
    for (let i = 0; i < Math.min(n, 12); i += 1) {
      const option = options.nth(i);
      if (!(await option.isVisible())) continue;
      const box = (await option.boundingBox())!;
      const inside = await option.evaluate((el, { x, y }) => {
        const hit = document.elementFromPoint(x, y);
        const own = el.closest('[role="menu"], dialog, [role="dialog"]');
        return !!hit && !!own && own.contains(hit);
      }, { x: box.x + box.width / 2, y: box.y + box.height / 2 });
      expect(inside, `${screen} › ${name}: option ${i + 1} takes its own press`).toBe(true);
    }
    // What's under it is inert, the button that opened it too (a modal
    // <dialog> makes the page inert itself).
    expect(await opener.evaluate((el) => el.closest("[inert]") !== null || document.querySelector("dialog:modal") !== null), `${screen} › ${name}: the page under it is inert`).toBe(true);
    covered.push({ screen, opener: name, kind, options: n });
    // A popover's own menus, one level down.
    if (!root && kind !== "menu") await auditScreen(page, `${screen} › ${name}`, panel, true);
    await page.keyboard.press("Escape");
    if (await panel.isVisible()) await page.mouse.click(2, 2);
    await expect(panel, `${screen} › ${name}: Escape closes it`).toBeHidden();
    if (root) await expect(root, `${screen} › ${name}: the popover it was in stays open`).toBeVisible();
    // A panel kept in the page, hidden, gives its mark back.
    await panel.evaluateAll((els) => els.forEach((el) => el.removeAttribute("data-audit-panel")));
    // Closed: nothing it made inert stays so.
    await expect(page.locator("[inert]"), `${screen} › ${name}: closed, the page under it is back`).toHaveCount(inertBefore);
  }
}

test.describe.configure({ mode: "serial" });

test.afterAll(() => {
  mkdirSync(OUT, { recursive: true });
  writeFileSync(resolve(OUT, "menus-a11y.json"), `${JSON.stringify({ spec: "ui/tests/visual/menus-a11y.spec.ts", menus: covered.length, covered }, null, 2)}\n`);
});

/** A screen's own content, once it has loaded (screens load lazily). */
async function screenOf(page: Page): Promise<Locator> {
  const main = page.locator("main");
  await expect.poll(() => main.evaluate((el) => (el.textContent ?? "").trim().length), { timeout: 15_000 }).toBeGreaterThan(0);
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(400);
  return main;
}

test("the rail and the header's menus and popovers (on every screen), and Home's own", async ({ page }) => {
  await open(page);
  await screenOf(page);
  await auditScreen(page, "Rail, header and Home", page.locator("body"));
});

for (const screen of ["Projects", "Pools", "Tasks", "Accounts", "Usage", "History", "Processes"]) {
  test(`every menu and popover on ${screen} keeps a press on its own options, and the page under it inert`, async ({ page }) => {
    await open(page);
    await goTo(page, screen);
    await auditScreen(page, screen, await screenOf(page));
  });
}

/** A pane loads its parts lazily (Accounts' date fields, Switching's keys
 * picker): wait until its openers stop appearing, or the audit's count
 * varies from run to run (75, 77, 78). */
async function settled(page: Page, within: Locator): Promise<void> {
  await page.waitForLoadState("networkidle");
  let last = -1;
  await expect
    .poll(
      async () => {
        const now = await within.locator(OPENERS).count();
        const same = now === last;
        last = now;
        return same;
      },
      { intervals: [300, 300, 500, 500, 800], timeout: 8_000 },
    )
    .toBe(true);
}

test("every menu and popover in each Settings pane", async ({ page }) => {
  await open(page);
  await goTo(page, "Settings");
  const main = await screenOf(page);
  const panes = page.getByRole("navigation", { name: "Settings" }).getByRole("button");
  for (const pane of await panes.allTextContents()) {
    await settingsPane(page, pane.trim());
    await settled(page, main);
    await auditScreen(page, `Settings › ${pane.trim()}`, main);
  }
});

/** The canvas pool with an App member (its role menu sits over Take out). */
const withApp = () => {
  const seed = canvasSeed(GATE_NOW);
  return { ...seed, __AIJ_POOL_RULES__: { "pool-checkout": { members: [{ member: "app:cli:claude-personal", kind: "app", role: "reviewer", level: null, joined_at: GATE_NOW - 3_600_000, sessions: 0, session_ids: [] }] } } };
};

test("a pool's page: the diagram, an App's role menu over Take out of the pool, and its settings", async ({ page }) => {
  await open(page, withApp());
  await goTo(page, "Pools");
  await page.getByTestId("pool-card-tile").filter({ hasText: "Checkout team" }).click();
  await expect(page.getByTestId("pool-canvas")).toBeVisible();
  const main = await screenOf(page);
  await auditScreen(page, "Pool page", main);
  // The walk's case: the App's role menu open over "Take out of the pool".
  await page.locator('[data-rule-ref="app:cli:claude-personal"]').click();
  await auditScreen(page, "Pool page › an App", main);
  await page.getByTestId("inspect-rule-role").click();
  const leave = page.getByTestId("inspect-rule-leave");
  expect(await leave.evaluate((el) => el.closest("[inert]") !== null)).toBe(true);
  await page.getByRole("menuitemradio", { name: /^Observer/ }).click();
  await expect(page.getByTestId("pool-take-out")).toHaveCount(0);
  await page.getByTestId("inspector-back").click();
  await page.getByTestId("inspect-open-settings").click();
  await auditScreen(page, "Pool page › its settings", main);
});
