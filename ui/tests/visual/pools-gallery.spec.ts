/**
 * AJ-E1702 / AJ-E1703 gallery: pools.
 *
 * - The rail's Pool lens (the View's Group by › Pool, offered once a pool
 *   of yours exists): Open now by pool, a pool's header opens its page.
 * - A session's "Move to pool…" sheet, and the pool sheet with what
 *   agents may do unfolded (D-28 levels).
 * - The pool page: the members' context cards, one unfolded (the agent's
 *   own words marked; Unavailable and why), and its Agent access tab
 *   (hop limit, rules, "Include my paired Macs").
 * - Settings › Agent access (its own pane): every pool's level, and the
 *   rules you made.
 * - Keyboard: a pool's header Open opens its page, a card unfolds with Enter,
 *   the level moves with the arrow keys.
 *
 * Both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, openFilters } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/91-pools");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

/** The gate's one clock (axe-gallery.ts). */
const T0 = GATE_NOW;

/** "Checkout redesign": the live session leads, a finished one works in
 * it; "Docs" is made, nobody in it yet. One rule the user made. */
const POOLS = {
  pools: [
    {
      id: "pool-checkout",
      name: "Checkout redesign",
      rules: [{ id: 1, pool_id: "pool-checkout", action: "assign_task", created_at: T0 - 3_600_000, made_by: "user" }],
    },
    { id: "pool-docs", name: "Docs" },
  ],
  members: [
    { session_id: "sess-live-1", pool_id: "pool-checkout", role: "lead" },
    { session_id: "sess-done-1", pool_id: "pool-checkout", role: "member" },
  ],
};

/** What the transcripts said (the tick's view). */
const ACTIVITY = {
  "sess-live-1": { files: ["src/cart.ts", "src/checkout/steps.ts"], said: "Split checkout into three steps" },
};

async function open(page: Page, theme: "dark" | "light") {
  // The gate's clock seam (axe-gallery.ts): Date.now() and new Date()
  // both read T0, in the gate and in the evidence gallery alike.
  await page.clock.setFixedTime(T0);
  await page.addInitScript(
    ({ theme, pools, activity }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_POOLS__ = pools;
      w.__AIJ_POOL_ACTIVITY__ = activity;
      // AJ-E1704: the one-time question answered (its own gallery is
      // agent-tools-gallery.spec.ts), so these captures stay as they were.
      w.__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, pools: POOLS, activity: ACTIVITY },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

/** The View's Group by › Pool, then the View closed. */
async function groupByPool(page: Page) {
  await openFilters(page);
  const pool = page.getByTestId("rail-view").getByRole("radio", { name: /^Pool/ });
  await expect(pool).toBeEnabled();
  await pool.check();
  await page.keyboard.press("Escape");
}

/** A pool's header: its Open button. */
const poolOpen = (page: Page, name: string) => page.getByTestId("place-header").getByRole("button", { name: new RegExp(`^Open ${name}`) });

async function poolPage(page: Page) {
  await groupByPool(page);
  await poolOpen(page, "Checkout redesign").click();
  await expect(page.getByTestId("pool-page")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  test(`the rail's Pool lens — ${theme}`, gate("91-pools"), async ({ page }) => {
    await open(page, theme);
    await groupByPool(page);
    await expect(poolOpen(page, "Checkout redesign")).toBeVisible();
    // No pools section and no Home picker: a pool's view is its page.
    await expect(page.getByTestId("home-pool-filter")).toHaveCount(0);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `rail-by-pool-1280x800-${theme}.png`) });
    await axe(page, `rail by pool ${theme}`);
  });

  test(`a session's Move to pool — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await page.locator('[data-instance-id="sess-live-1"]').click({ button: "right" });
    await page.getByRole("menuitem", { name: "Move to pool…" }).click();
    const sheet = page.getByTestId("pool-move-sheet");
    await expect(sheet.getByTestId("pool-move-choice")).toHaveCount(3);
    await expect(sheet.locator('[aria-current="true"]')).toContainText("Checkout redesign");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `move-to-pool-1280x800-${theme}.png`) });
    await axe(page, `move to pool ${theme}`);
  });

  test(`the pool sheet, with what agents may do — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await poolPage(page);
    await page.getByTestId("pool-page-edit").click();
    const sheet = page.getByTestId("pool-sheet");
    await expect(sheet.getByTestId("pool-member")).toHaveCount(2);
    await sheet.getByTestId("pool-access-fold").locator("> summary").click();
    await expect(sheet.getByTestId("pool-level-work_together").getByRole("radio")).toBeChecked();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `pool-sheet-1280x800-${theme}.png`) });
    await axe(page, `pool sheet ${theme}`);
  });

  test(`the pool page's cards — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await poolPage(page);
    const cards = page.getByTestId("pool-card");
    await expect(cards).toHaveCount(2);
    await cards.first().getByTestId("pool-card-more").click();
    await expect(page.getByTestId("pool-card-said")).toContainText("said by the agent");
    await expect(page.getByTestId("pool-card-files")).toContainText("src/cart.ts");
    await cards.nth(1).getByTestId("pool-card-more").click();
    await expect(cards.nth(1).getByTestId("pool-card-files")).toContainText("Unavailable — ");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `pool-cards-1280x800-${theme}.png`) });
    await axe(page, `pool cards ${theme}`);
  });

  test(`the pool page's agent access — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await poolPage(page);
    await page.getByRole("tab", { name: "Agent access" }).click();
    await expect(page.getByTestId("pool-rule")).toContainText("Give a task to someone else");
    await page.getByTestId("pool-delivery").locator("summary").click();
    await expect(page.getByTestId("pool-hop-limit")).toHaveAttribute("data-value", "5");
    await expect(page.getByText("Message and task content will travel to your other Macs.")).toBeVisible();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `pool-access-1280x800-${theme}.png`) });
    await axe(page, `pool access ${theme}`);
  });

  test(`Settings › Agent access — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Settings");
    await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Agent access" }).click();
    await expect(page.getByTestId("settings-pane-agent-access")).toBeVisible();
    await expect(page.getByTestId("agent-access-rule")).toContainText("Give a task to someone else");
    const card = page.getByTestId("settings-agent-access");
    await expect(card.getByTestId("agent-access-pool")).toHaveCount(5);
    await card.getByTestId("agent-access-pool").nth(1).getByTestId("agent-access-level").click();
    await card.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `settings-agent-access-1280x800-${theme}.png`) });
    await axe(page, `settings agent access ${theme}`);
  });
}

test("keyboard: a pool's header, a card's disclosure, the level", async ({ page }) => {
  await open(page, "dark");
  await groupByPool(page);
  const header = poolOpen(page, "Checkout redesign");
  await header.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("pool-page")).toBeVisible();
  const more = page.getByTestId("pool-card-more").first();
  await more.focus();
  await page.keyboard.press("Enter");
  await expect(more).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Enter");
  await expect(more).toHaveAttribute("aria-expanded", "false");
  await page.getByRole("tab", { name: "Agent access" }).focus();
  await page.keyboard.press("Enter");
  const work = page.getByTestId("pool-level-work_together").getByRole("radio");
  await work.focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByTestId("pool-level-full").getByRole("radio")).toBeChecked();
});
