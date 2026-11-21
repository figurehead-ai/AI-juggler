/**
 * AJ-E3011 round 5 (after5-run1.md, with main's correction under D-55):
 * the pool page's layout, measured in a real browser.
 *
 * - No layout shift on Add: the next Add button is where it was after an
 *   Add, in the palette and in its More (Apps).
 * - The inspector: a section opened from a Change is at the top of the
 *   panel with its control focused and fully in view; its menu opens
 *   inside the window; an App's role menu too. At 1024×768 and 1280×800.
 * - At 1280×800 its first section (the pool in short) fits without
 *   scrolling.
 * - An empty pool opens in its guided setup (New pool's own steps), with
 *   Show the diagram.
 */
import { type Locator, type Page } from "@playwright/test";
import { expect, GATE_NOW, test } from "./axe-gallery";
import { goTo } from "./fixtures";
import { canvasSeed } from "./fixtures/pool-canvas";

const POOL = "pool-checkout";

function seed(t0: number) {
  const base = canvasSeed(t0);
  const rule = (member: string, kind: string, role: string) => ({ member, kind, role, level: null, joined_at: t0 - 3_600_000, sessions: 0, session_ids: [] });
  return {
    ...base,
    __AIJ_POOLS__: { ...base.__AIJ_POOLS__, pools: [...base.__AIJ_POOLS__.pools, { id: "pool-docs", name: "Docs crew", rules: [] }] },
    __AIJ_POOL_RULES__: { [POOL]: { members: [rule("app:cli:claude-personal", "app", "reviewer")] } },
    // Two Claude app profiles (Casey's PC1), as places.
    __AIJ_PLACES__: {
      places: ["Work", "Side"].map((name) => ({ id: `app:claude-app:/Users/you/Profiles/${name}`, app_id: "claude-app", app: "Claude", instance: name, profile: name === "Work" ? "Work Max" : "Side", label: `Claude · ${name}`, account_id: name === "Work" ? "claude-work" : "claude-side", app_pid: null, lost: false, keep_alive: false, paused: null, seen_ms: t0 - 3_600_000, cannot_start: null, recent: [], found: false })),
    },
  };
}

async function open(page: Page, size: { width: number; height: number }) {
  await page.clock.setFixedTime(GATE_NOW);
  await page.addInitScript((data) => {
    Object.assign(window, data);
    localStorage.setItem("aij.theme", "dark");
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
  }, seed(GATE_NOW));
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize(size);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function pool(page: Page, name: string) {
  await goTo(page, "Pools");
  await page.getByTestId("pool-card-tile").filter({ hasText: name }).click();
}

/** Its box, rounded (a sub-pixel isn't a shift). */
const box = async (one: Locator) => {
  const at = (await one.boundingBox())!;
  return { x: Math.round(at.x), y: Math.round(at.y), w: Math.round(at.width), h: Math.round(at.height) };
};

/** Whether `one` is wholly inside the window and inside `area`. */
async function inView(page: Page, one: Locator, area: Locator): Promise<boolean> {
  const it = (await one.boundingBox())!;
  const room = (await area.boundingBox())!;
  const view = page.viewportSize()!;
  return it.y >= room.y - 1 && it.y + it.height <= room.y + room.height + 1 && it.y >= 0 && it.y + it.height <= view.height && it.x >= 0 && it.x + it.width <= view.width;
}

/** The newest open menu is inside the window. */
async function menuInside(page: Page): Promise<void> {
  const menu = page.locator('[role="menu"]').last();
  await expect(menu).toBeVisible();
  const it = (await menu.boundingBox())!;
  const view = page.viewportSize()!;
  expect(it.y, "the menu's top is in the window").toBeGreaterThanOrEqual(0);
  expect(it.y + it.height, "the menu's foot is in the window").toBeLessThanOrEqual(view.height + 1);
  expect(it.x + it.width, "the menu's right edge is in the window").toBeLessThanOrEqual(view.width + 1);
  await page.keyboard.press("Escape");
}

test("Add keeps the next Add where it was: the palette and its More", async ({ page }) => {
  await open(page, { width: 1280, height: 800 });
  await pool(page, "Checkout team");
  await expect(page.getByTestId("pool-canvas")).toBeVisible();
  // D-126: a running pool shows its setup again by Edit setup.
  if (await page.getByTestId("pool-edit-setup").isVisible()) await page.getByTestId("pool-edit-setup").click();
  const palette = page.getByTestId("canvas-palette");
  if (!(await page.getByTestId("canvas-palette-body").isVisible())) await palette.getByRole("button", { name: /Add a member/ }).click();
  const adds = page.getByTestId("canvas-palette-add");
  await expect(adds).toHaveCount(2);
  const next = await box(adds.nth(1));
  await adds.nth(0).click();
  await expect(page.getByTestId("canvas-palette-added")).toHaveCount(1);
  expect(await box(page.getByTestId("canvas-palette-add").first()), "the next session's Add didn't move").toEqual(next);

  // The palette's More: the same, in the first kind with two to add.
  const more = page.getByTestId("canvas-palette-more-add");
  for (const kind of ["Sessions in other apps", "Apps", "Accounts", "Tools"]) {
    if ((await more.count()) >= 2) break;
    await page.getByTestId("canvas-palette-kind").click();
    const option = page.getByRole("menuitemradio", { name: new RegExp(`^${kind}`) });
    if ((await option.count()) === 0) {
      await page.keyboard.press("Escape");
      continue;
    }
    await option.click();
  }
  expect(await more.count(), "a kind with two to add").toBeGreaterThanOrEqual(2);
  const second = await box(more.nth(1));
  await more.nth(0).click();
  await expect(page.getByTestId("canvas-palette-more-added")).toHaveCount(1);
  expect(await box(page.getByTestId("canvas-palette-more-add").first()), "the next App's Add didn't move").toEqual(second);
});

for (const size of [
  { width: 1024, height: 768 },
  { width: 1280, height: 800 },
]) {
  test(`a section opened from Change is at the top, its control focused and in view; menus inside the window — ${size.width}×${size.height}`, async ({ page }) => {
    await open(page, size);
    await pool(page, "Checkout team");
    await expect(page.getByTestId("pool-inspector")).toBeVisible();
    const list = page.locator('[data-testid="pool-inspector"] [data-canvas-scroll]').first();
    // (Who's in opens the palette on the diagram, not a section.)
    for (const row of ["lead", "own"] as const) {
      if (await page.getByTestId("inspector-back").isVisible()) await page.getByTestId("inspector-back").click();
      // D-126 §3: the summary in sentences is in the pool's settings.
      // D-126: a running pool shows its setup again by Edit setup.
      if (await page.getByTestId("pool-edit-setup").isVisible()) await page.getByTestId("pool-edit-setup").click();
      await page.getByTestId("inspect-open-settings").click();
      await page.locator(`[data-testid="pool-summary"] [data-row="${row}"] [data-testid$="-change"]`).click();
      const section = page.locator(`#pool-setup-${row}`);
      await expect(section).toBeVisible();
      const focused = page.locator(`#pool-setup-${row} :focus`);
      await expect(focused, `${row}: its control is focused`).toHaveCount(1);
      expect(await inView(page, focused, list), `${row}: its control is fully in view`).toBe(true);
    }
    // Everyone else: its join picker, focused, and its menu inside the window.
    await page.locator("#pool-setup-everyone").scrollIntoViewIfNeeded();
    await page.getByTestId("pool-join-picker").click();
    await menuInside(page);
    // An App's role menu, never off-screen.
    if (await page.getByTestId("inspector-back").isVisible()) await page.getByTestId("inspector-back").click();
    // (Under 1100 px a pool opens as rows: the diagram, chosen.)
    await page.getByTestId("pool-mode").getByRole("radio", { name: "Diagram" }).click();
    await page.locator('[data-rule-ref="app:cli:claude-personal"]').click();
    const role = page.getByTestId("inspect-rule-role");
    expect(await inView(page, role, list), "the App's role picker is in view").toBe(true);
    await role.click();
    await menuInside(page);
  });
}

test("at 1280×800 the inspector's first section fits without scrolling", async ({ page }) => {
  await open(page, { width: 1280, height: 800 });
  await pool(page, "Checkout team");
  const list = page.locator('[data-testid="pool-inspector"] [data-canvas-scroll]').first();
  // D-126 §3: the panel opens on the checklist while it's set up, else
  // on the calm live view.
  const summary = page.locator('[data-testid="pool-checklist"], [data-testid="pool-ready"], [data-testid="pool-running"]').first();
  await expect(summary).toBeVisible();
  expect(await inView(page, summary, list)).toBe(true);
  expect(await list.evaluate((el) => el.scrollTop)).toBe(0);
});

test("an empty pool opens in its guided setup, with Show the diagram", async ({ page }) => {
  await open(page, { width: 1280, height: 800 });
  await pool(page, "Docs crew");
  const setup = page.getByTestId("new-pool");
  await expect(setup).toHaveAttribute("data-embedded", "true");
  // New pool's own steps beside it (D-126: the kit's FlowSteps), on How to start.
  const steps = page.getByTestId("new-pool-steps").getByTestId("flow-step");
  await expect(steps.first()).toHaveAttribute("data-step", "template");
  await expect(steps.first()).toHaveAttribute("data-state", "current");
  expect(await steps.count()).toBeGreaterThanOrEqual(8);
  await expect(page.getByTestId("new-pool-step-template")).toBeVisible();
  await page.getByTestId("new-pool-show-diagram").click();
  await expect(page.getByTestId("pool-canvas").or(page.getByTestId("pool-list"))).toBeVisible();
});

// Casey's PC1 walk: after "Work Max's next session" in Who leads, "No lead
// for now" seemed to keep its dot. One radio checked, ever.
test("Who leads: one choice checked at a time", async ({ page }) => {
  await open(page, { width: 1280, height: 800 });
  await goTo(page, "Pools");
  await page.getByTestId("pools-new").click();
  for (let n = 0; n < 4 && !(await page.getByTestId("new-pool-step-who").isVisible()); n += 1) await page.getByTestId("new-pool-next").click();
  // Both Claude app profiles, as Casey did (D-126: each App a logo card;
  // the rc3 critique: two of one app lead with their profiles, "Claude
  // app · Desktop app" under them).
  const apps = page.getByTestId("new-pool-apps");
  for (const profile of ["Work Max", "Side"]) await apps.getByTestId("new-pool-candidate").filter({ hasText: "Claude app · Desktop app" }).filter({ has: page.getByRole("checkbox", { name: profile, exact: true }) }).locator("label").first().click();
  await page.getByTestId("new-pool-next").click();
  const picker = page.getByTestId("new-pool-step-lead");
  // D-126: each choice a card, its value "next|<App>", "session|<ref>" or "none".
  const radios = picker.locator('[data-testid="new-pool-lead-option"] input[type="radio"]');
  const checked = () => radios.evaluateAll((all) => all.filter((one) => (one as HTMLInputElement).checked).map((one) => (one.closest("[data-value]")?.getAttribute("data-value") ?? "").split("|")[0]));
  await expect(picker).toBeVisible();
  for (const kind of ["next", "none", "next"]) {
    await picker.locator(`[data-testid="new-pool-lead-option"][data-value^="${kind}"]`).first().click();
    await expect.poll(checked).toEqual([kind]);
  }
});

// The aij-test proof: at 1761×1348 the delete-pool confirmation was
// clipped at the panel's foot, its buttons out of reach. Every
// confirmation in the panel shows in view, its first button focused.
for (const size of [
  { width: 1761, height: 1348 },
  { width: 1280, height: 800 },
  { width: 1024, height: 768 },
]) {
  test(`the panel's confirmations stay in view — ${size.width}×${size.height}`, async ({ page }) => {
    await open(page, size);
    // A pool of the user's own (a project's own can't be deleted).
    await pool(page, "Checkout team");
    if (await page.getByTestId("pool-mode").isVisible()) await page.getByTestId("pool-mode").getByRole("radio", { name: "Diagram" }).click();
    // D-126: a running pool's foot says Edit setup (its setup again, whose
    // foot holds the settings); one still being set up says the settings.
    // Wait for whichever its panel draws: a one-off isVisible() raced it.
    const edit = page.getByTestId("pool-edit-setup");
    const settings = page.getByTestId("inspect-open-settings");
    await expect(edit.or(settings).first()).toBeVisible();
    if (await edit.isVisible()) await edit.click();
    await settings.click();
    const list = page.locator('[data-testid="pool-inspector"] [data-canvas-scroll]').first();
    for (const [open, confirm, safe] of [
      ["pool-delete", "pool-delete-confirm", "pool-delete-keep"],
      ["pool-seal-open", "pool-seal-yes", "pool-seal-no"],
    ] as const) {
      if (open === "pool-seal-open") await page.getByTestId("pool-more-fold-toggle").click();
      await page.getByTestId(open).scrollIntoViewIfNeeded();
      await page.getByTestId(open).click();
      const button = page.getByTestId(confirm);
      await expect(button).toBeVisible();
      await expect.poll(() => inView(page, button, list), { message: `${confirm} is in view` }).toBe(true);
      // Its safe choice has focus (Enter never deletes by surprise).
      await expect(page.getByTestId(safe)).toBeFocused();
      await page.getByTestId(safe).click();
    }
  });
}

// Round 6 #4 (Casey's run): the project page's "New pool for web…" went to
// the Pools list, not the setup. It opens New pool, on that project.
test("a project's New pool for … stays put as the tab loads, and opens New pool on that project", async ({ page }) => {
  await open(page, { width: 1280, height: 800 });
  await goTo(page, "Projects");
  await page.locator("main").getByText("storefront", { exact: true }).first().click();
  await page.getByRole("tablist", { name: "Project tabs" }).getByRole("tab", { name: /^Pools/ }).click();
  // Casey's run: the door moved under the pointer as the tab loaded. Its
  // box from first paint is its box once all has loaded.
  const door = page.getByTestId("project-pool-new");
  await expect(door).toBeVisible();
  const first = await box(door);
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(800);
  expect(await box(door), "New pool for … didn't move as the tab loaded").toEqual(first);
  await door.click();
  await expect(page.getByTestId("new-pool")).toBeVisible();
  // Round 6 #7: from a project, Start from scratch is the open path.
  await expect(page.getByTestId("new-pool-templates-fold")).not.toHaveAttribute("data-open", "true");
  await page.getByTestId("new-pool-next").click();
  await expect(page.getByTestId("new-pool-projects")).toContainText("storefront");
  // D-126: each project a card; that project's is chosen.
  await expect(page.getByTestId("new-pool-project").filter({ hasText: "storefront" }).getByRole("checkbox")).toBeChecked();
});

// Round 6 #6 (Casey's run): a note at the window's foot covered New pool's
// Next and Make pool. Toasts sit above any footer at the window's foot that
// carries `data-toast-avoid` (ToastStack's useAboveFooters, held by its unit
// test). 0.4.8: the setup no longer raises a toast of its own (Next waits on
// Where for a project), so this holds the other half: New pool's footer is
// marked and sits at the window's foot, from its portal under the body too.
test("a toast never covers New pool's footer", async ({ page }) => {
  await open(page, { width: 1280, height: 800 });
  await goTo(page, "Pools");
  await page.getByTestId("pools-new").click();
  const next = page.getByTestId("new-pool-next");
  await expect(next).toBeVisible();
  const foot = next.locator("xpath=ancestor::*[@data-toast-avoid][1]");
  await expect(foot).toHaveCount(1);
  const box = (await foot.boundingBox())!;
  expect(box.y + box.height, "the footer sits at the window's foot, where the toasts avoid it").toBeGreaterThanOrEqual(800 - 24);
});
