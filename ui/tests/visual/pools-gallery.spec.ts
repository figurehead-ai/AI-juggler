/**
 * AJ-E1702 / AJ-E1703 gallery: pools.
 *
 * - The rail's Pool lens (the View's Group by › Pool, offered once a pool
 *   of yours exists): Open now by pool, a pool's header opens its page.
 * - A session's "Move to pool…" sheet.
 * - Settings › Agent access (its own pane): every pool's level, and the
 *   rules you made.
 * - The pool's own page is its canvas (0.3): pool-canvas-gallery.spec.ts.
 *
 * Both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, groupBy, hoverRailGroup } from "./fixtures";

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

async function open(page: Page, theme: "dark" | "light", size: Size = SIZES[0]) {
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
  await page.setViewportSize({ width: size.width, height: size.height });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

/** The rail's switch › Pool (0.3: no longer in the View). */
async function groupByPool(page: Page) {
  await groupBy(page, "Pool");
}


// AJ-E3305: both sizes the 14-line review scores (1024 was missing).
type Size = { width: number; height: number; name: string };
const SIZES: Size[] = [
  { width: 1280, height: 800, name: "1280x800" },
  { width: 1024, height: 700, name: "1024x700" },
];

for (const size of SIZES)
for (const theme of ["dark", "light"] as const) {
  const at = size.name === "1280x800" ? theme : `${size.name} — ${theme}`;
  test(`the rail's Pool lens — ${at}`, gate("91-pools"), async ({ page }) => {
    test.skip(size.name !== "1280x800", "below 1100 px the rail folds to its icons: its lens is the wide rail's");
    await open(page, theme, size);
    // The fixture's unsigned Personal app is a Needs you notice, sent after
    // the rail's first paint; it lands above the pool headers and moves them
    // ~90 px. Hover only once it's there, or the pointer is left on another
    // line and the hover-only Open hides (#218 run 43068843764, visual 4/4).
    await expect(page.getByRole("heading", { name: /^Needs you/ })).toBeVisible();
    await groupByPool(page);
    // Its Open shows on hover (0.4); the helper hovers until it does.
    await hoverRailGroup(page, "Checkout redesign");
    // No pools section and no Home picker: a pool's view is its page.
    await expect(page.getByTestId("home-pool-filter")).toHaveCount(0);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `rail-by-pool-${size.name}-${theme}.png`) });
    await axe(page, `rail by pool ${theme}`);
  });

  test(`a session's Move to pool — ${at}`, async ({ page }) => {
    test.skip(size.name !== "1280x800", "below 1100 px the rail folds to its icons: Move to pool is the wide rail's menu");
    await open(page, theme, size);
    await page.getByTestId("rail").locator('[data-instance-id="sess-live-1"]').click({ button: "right" });
    await page.getByRole("menuitem", { name: "Move to pool…" }).click();
    const sheet = page.getByTestId("pool-move-sheet");
    await expect(sheet.getByTestId("pool-move-choice")).toHaveCount(3);
    await expect(sheet.locator('[aria-current="true"]')).toContainText("Checkout redesign");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `move-to-pool-${size.name}-${theme}.png`) });
    await axe(page, `move to pool ${theme}`);
  });

  test(`Settings › Agent access — ${at}`, async ({ page }) => {
    await open(page, theme, size);
    await goTo(page, "Settings");
    await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Agent access" }).click();
    await expect(page.getByTestId("settings-pane-agent-access")).toBeVisible();
    await expect(page.getByTestId("agent-access-rule")).toContainText("Give a task to someone else");
    const card = page.getByTestId("settings-agent-access");
    await expect(card.getByTestId("agent-access-pool")).toHaveCount(5);
    await card.getByTestId("agent-access-pool").nth(1).getByTestId("agent-access-level").click();
    await card.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `settings-agent-access-${size.name}-${theme}.png`) });
    await axe(page, `settings agent access ${theme}`);
  });
}
