/**
 * AJ-E1704 gallery: AI Juggler's agent tools in the sessions it starts
 * (D-30).
 *
 * - The one-time question, on a pool's page the first time pools are
 *   used: "Let agents in your projects see each other and work together?"
 *   (never a pop-up).
 * - Settings › Agent access › Agent tools in sessions: the switch for
 *   every session, and the fold with a switch per project and profile,
 *   one project switched off.
 * - Keyboard: the question's buttons and the fold work from the keys.
 *
 * Both themes; axe 0 critical/serious on each. Preview data only.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, groupBy, openRailGroup } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/92-agent-tools");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

/** The gate's one clock (axe-gallery.ts). */
const T0 = GATE_NOW;

const POOLS = {
  pools: [{ id: "pool-checkout", name: "Checkout redesign" }],
  members: [{ session_id: "sess-live-1", pool_id: "pool-checkout", role: "lead" }],
};

async function open(page: Page, theme: "dark" | "light", bridge: Record<string, unknown> | null) {
  // The gate's clock seam (axe-gallery.ts): Date.now() and new Date()
  // both read T0, in the gate and in the evidence gallery alike.
  await page.clock.setFixedTime(T0);
  await page.addInitScript(
    ({ theme, pools, bridge }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_POOLS__ = pools;
      if (bridge) w.__AIJ_AGENT_BRIDGE__ = bridge;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, pools: POOLS, bridge },
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

async function poolPage(page: Page) {
  // The rail's Needs you (the fixture's unsigned Personal app) lands after
  // the first paint. Opened before then, the rail held the pool's line
  // still under the pointer and scrolled the notice away behind "1 new
  // above" (rail-hold; CI run 43403942121, 15 of 20 local runs).
  await expect(page.getByTestId("rail-needs-you")).toContainText(/Needs you\s*1/);
  await groupBy(page, "Pool");
  await openRailGroup(page, "Checkout redesign", page.getByTestId("pool-page"));
  await expect(page.getByTestId("pool-page")).toBeVisible();
}

async function agentAccess(page: Page) {
  await goTo(page, "Settings");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Agent access" }).click();
  await expect(page.getByTestId("settings-pane-agent-access")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  test(`the one-time question on a pool's page — ${theme}`, gate("92-agent-tools"), async ({ page }) => {
    await open(page, theme, null);
    await poolPage(page);
    const question = page.getByTestId("agent-tools-question");
    await expect(question).toContainText("Let agents in your projects see each other and work together?");
    await expect(question.getByRole("button", { name: "Yes, let them" })).toBeVisible();
    await expect(question.getByRole("button", { name: "Not now" })).toBeVisible();
    // The rc26 P3/P9 walks: it says it's asked once for every pool, and
    // keeps this pool's own level.
    await expect(question.getByTestId("agent-tools-question-pool")).toContainText("Asked once, for every pool.");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `agent-tools-question-1280x800-${theme}.png`) });
    await axe(page, `the question ${theme}`);
  });

  test(`Settings › Agent access › Agent tools in sessions — ${theme}`, async ({ page }) => {
    await open(page, theme, { asked: true, on: true, off_projects: [], off_profiles: [] });
    await agentAccess(page);
    const card = page.getByTestId("settings-agent-tools");
    await card.scrollIntoViewIfNeeded();
    await expect(card.getByTestId("pref-agent-tools").getByRole("checkbox")).toBeChecked();
    await card.getByTestId("agent-tools-where").locator("> summary").click();
    const first = card.getByRole("group", { name: "Projects" }).getByRole("checkbox").first();
    await expect(first).toBeChecked();
    await first.uncheck();
    await expect(card.getByTestId("agent-tools-where").locator("> summary")).toContainText("1 off");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `settings-agent-tools-1280x800-${theme}.png`) });
    await axe(page, `settings agent tools ${theme}`);
  });
}

test("keyboard: the question answers, the fold opens", async ({ page }) => {
  await open(page, "dark", null);
  await agentAccess(page);
  const yes = page.getByTestId("settings-agent-tools").getByRole("button", { name: "Yes, let them" });
  await yes.focus();
  await page.keyboard.press("Enter");
  const all = page.getByTestId("pref-agent-tools").getByRole("checkbox");
  await expect(all).toBeChecked();
  const fold = page.getByTestId("agent-tools-where").locator("> summary");
  await fold.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("settings-agent-tools").getByRole("group", { name: "Apps" })).toBeVisible();
});
