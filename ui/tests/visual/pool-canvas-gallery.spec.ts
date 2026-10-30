/**
 * 0.3 gallery (redesign §5): Pools, one live canvas.
 *
 * - The Pools screen: every pool as a card with its canvas in small, what
 *   waits on you, tasks moving; a project's pool labelled.
 * - A pool's canvas: the lead on top, members around it, edges with what
 *   passes between them, an ask answered on its node, a message waiting
 *   for Deliver and a chain paused at the hop limit on their edges, the
 *   palette of sessions to add, the live activity in the inspector.
 * - The inspector on a member (its asks, tasks, who it talks to, usage),
 *   on an edge (the conversation), on the pool (what agents may do,
 *   rules, delivery, notifications).
 * - View as list; Tasks and Messages over the canvas; a new, empty pool.
 * - Semantic zoom (0.3 polish): at fit the nodes draw their words at
 *   screen size (compact), a node's waiting badge opens its ask to answer
 *   in place, an edge's waiting badge opens Deliver / Let it continue;
 *   at 100 % the full cards. Under 1100 px a pool opens in the List view
 *   until you choose (the choice is kept for that pool).
 * - Interaction: drag a session from the palette in, drag a node (its
 *   place kept), the keyboard (arrows between nodes, Enter, Delete with
 *   Undo, Option+arrows), an ask answered in place.
 *
 * 1280×800 and 1024×700, both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";
import { canvasSeed } from "./fixtures/pool-canvas";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/97-pool-canvas");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

const SIZES = [
  [1280, 800],
  [1024, 700],
] as const;

async function open(page: Page, theme: "dark" | "light", width: number, height: number) {
  await page.clock.setFixedTime(GATE_NOW);
  await page.addInitScript(
    ({ theme, seed }) => {
      Object.assign(window, seed);
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, seed: canvasSeed(GATE_NOW) },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function poolsScreen(page: Page) {
  await goTo(page, "Pools");
  await expect(page.getByTestId("pools-screen")).toBeVisible();
  await expect(page.getByTestId("pool-card-tile").first()).toBeVisible();
}

async function canvas(page: Page) {
  await poolsScreen(page);
  await page.getByTestId("pool-card-tile").filter({ hasText: "Checkout team" }).click();
  // Under 1100 px a pool opens as rows until you choose the canvas.
  if ((page.viewportSize()?.width ?? 1280) < 1100) {
    await expect(page.getByTestId("pool-card").first()).toBeVisible();
    await page.getByTestId("pool-mode-canvas").click();
  }
  await expect(page.getByTestId("pool-canvas")).toBeVisible();
  await expect(page.getByTestId("canvas-node")).toHaveCount(6);
}

/** Back to 100 %: the full cards. */
async function fullCards(page: Page) {
  await page.getByTestId("canvas-zoom").click();
  await expect(page.getByTestId("canvas-zoom")).toHaveText("100%");
  await expect(page.locator("[data-form='compact']")).toHaveCount(0);
}

/** The palette, unfolded (it starts folded on a narrow canvas). */
async function palette(page: Page) {
  const fold = page.getByTestId("canvas-palette").getByRole("button", { name: /Add a session/ });
  if ((await fold.getAttribute("aria-expanded")) !== "true") await fold.click();
  await expect(page.getByTestId("canvas-palette-item").first()).toBeVisible();
}

const calls = (page: Page, seam: string) => page.evaluate((name) => (window as unknown as Record<string, string[] | undefined>)[name] ?? [], seam);

for (const theme of ["dark", "light"] as const) {
  for (const [width, height] of SIZES) {
    const size = `${width}x${height}`;

    test(`the Pools screen — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      await poolsScreen(page);
      const team = page.getByTestId("pool-card-tile").filter({ hasText: "Checkout team" });
      await expect(team.getByTestId("pool-card-words")).toHaveText("6 members · 4 need you · 2 tasks moving");
      await expect(page.getByTestId("pool-card-tile").filter({ hasText: "project pool" })).toHaveCount(2);
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `pools-screen-${size}-${theme}.png`) });
      await axe(page, `pools screen ${size} ${theme}`);
    });

    test(`a pool's canvas — ${size} ${theme}`, width === 1280 ? gate("97-pool-canvas") : {}, async ({ page }) => {
      await open(page, theme, width, height);
      await canvas(page);
      // At fit, under 100 %: every node compact, its words at screen size.
      await expect(page.locator("[data-form='full']")).toHaveCount(0);
      await expect(page.locator('[data-node="s-plan"]').getByTestId("node-lead")).toBeVisible();
      await expect(page.locator('[data-node="s-copy"]').getByTestId("node-badge")).toBeVisible();
      await expect(page.getByTestId("edge-badge").first()).toBeVisible();
      await expect(page.getByTestId("canvas-palette").getByRole("button", { name: /Add a session/ })).toBeVisible();
      await expect(page.getByTestId("inspect-stream")).toContainText("Planner gave «Checkout copy» to Checkout copy pass");
      // Nothing on a node drawn under 12 px.
      const smallest = await page.locator("[data-node] *").evaluateAll((all) =>
        Math.min(...all.filter((el) => el.childNodes.length > 0 && [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent!.trim())).map((el) => parseFloat(getComputedStyle(el).fontSize) * (el.getBoundingClientRect().height / ((el as HTMLElement).offsetHeight || el.getBoundingClientRect().height)))),
      );
      // (Measured through the transforms: a hair under 12 is rounding.)
      expect(smallest, "node text 12 px or more on screen").toBeGreaterThanOrEqual(11.5);
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `canvas-${size}-${theme}.png`) });
      await axe(page, `canvas ${size} ${theme}`);
    });

    test(`a node's waiting badge opened, its ask answered in place — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      await canvas(page);
      await page.locator('[data-node="s-copy"]').getByTestId("node-badge").click();
      const panel = page.locator('[data-node="s-copy"]').getByTestId("node-popover");
      await expect(panel.getByTestId("node-ask")).toContainText("wants to message Planner");
      await expect(panel.getByTestId("node-ask-once")).toBeFocused();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `canvas-badge-open-${size}-${theme}.png`) });
      await axe(page, `node badge open ${size} ${theme}`);
    });

    test(`an edge's waiting badge opened — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      await canvas(page);
      const paused = page.getByTestId("edge-badge").filter({ hasText: "Paused" });
      await paused.click();
      await expect(page.getByTestId("edge-badge-panel").getByTestId("edge-deliver")).toHaveText("Let it continue");
      await expect(page.getByTestId("edge-badge-panel").getByTestId("edge-dont-deliver")).toBeVisible();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `canvas-edge-badge-open-${size}-${theme}.png`) });
      await axe(page, `edge badge open ${size} ${theme}`);
    });

    test(`the canvas at 100 %: full cards — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      await canvas(page);
      await fullCards(page);
      await expect(page.locator('[data-node="s-copy"]').getByTestId("node-ask")).toContainText("wants to message Planner");
      await expect(page.getByTestId("edge-pending").first()).toBeVisible();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `canvas-full-${size}-${theme}.png`) });
      await axe(page, `canvas full ${size} ${theme}`);
    });

    test(`a member in the inspector — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      await canvas(page);
      await page.locator('[data-node-name="s-copy"]').click();
      const inspector = page.getByTestId("inspect-node");
      await expect(inspector.getByTestId("ask-card")).toHaveCount(1);
      await expect(inspector.getByTestId("inspect-node-edges")).toContainText("To Planner");
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `inspect-member-${size}-${theme}.png`) });
      await axe(page, `inspect member ${size} ${theme}`);
    });
  }

  test(`an edge's conversation — 1280x800 ${theme}`, async ({ page }) => {
    await open(page, theme, 1280, 800);
    await canvas(page);
    await page.getByTestId("pool-mode-list").click();
    await page.getByTestId("pool-list-edges").getByRole("button", { name: /Planner → Fix the cart totals/ }).click();
    await page.getByTestId("pool-mode-canvas").click();
    const edge = page.getByTestId("inspect-edge");
    await expect(edge).toContainText("Planner → Fix the cart totals");
    await expect(edge.getByTestId("message-bubble")).toHaveCount(3);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `inspect-edge-1280x800-${theme}.png`) });
    await axe(page, `inspect edge ${theme}`);
  });

  test(`the pool's settings — 1280x800 ${theme}`, async ({ page }) => {
    await open(page, theme, 1280, 800);
    await canvas(page);
    await page.getByTestId("inspect-open-settings").click();
    await expect(page.getByTestId("inspect-pool")).toBeVisible();
    await expect(page.getByTestId("pool-rule")).toContainText("Take a task");
    await expect(page.getByTestId("pool-hop-limit")).toHaveAttribute("data-value", "5");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `inspect-pool-1280x800-${theme}.png`) });
    await axe(page, `pool settings ${theme}`);
  });

  test(`a narrow window opens a pool in the List view, until you choose — 1024x700 ${theme}`, async ({ page }) => {
    await open(page, theme, 1024, 700);
    await poolsScreen(page);
    await page.getByTestId("pool-card-tile").filter({ hasText: "Checkout team" }).click();
    await expect(page.getByTestId("pool-card")).toHaveCount(6);
    await expect(page.getByTestId("pool-canvas")).toHaveCount(0);
    await expect(page.getByTestId("pool-mode-list")).toHaveAttribute("aria-checked", "true");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `list-default-1024x700-${theme}.png`) });
    await axe(page, `list default ${theme}`);
    // Chosen once, kept for this pool.
    await page.getByTestId("pool-mode-canvas").click();
    await expect(page.getByTestId("pool-canvas")).toBeVisible();
    await page.reload();
    await poolsScreen(page);
    await page.getByTestId("pool-card-tile").filter({ hasText: "Checkout team" }).click();
    await expect(page.getByTestId("pool-canvas")).toBeVisible();
  });

  test(`View as list — 1280x800 ${theme}`, async ({ page }) => {
    await open(page, theme, 1280, 800);
    await canvas(page);
    await page.getByTestId("pool-mode-list").click();
    await expect(page.getByTestId("pool-card")).toHaveCount(6);
    await expect(page.getByTestId("pool-list-edges").locator("li")).toHaveCount(7);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `list-1280x800-${theme}.png`) });
    await axe(page, `list ${theme}`);
  });

  test(`Tasks and Messages over the canvas — 1280x800 ${theme}`, async ({ page }) => {
    await open(page, theme, 1280, 800);
    await canvas(page);
    await page.getByTestId("pool-panel-messages").click();
    await expect(page.getByTestId("pool-panel-messages-open")).toBeVisible();
    await expect(page.getByTestId("pool-panel-messages")).toHaveAttribute("aria-pressed", "true");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `messages-panel-1280x800-${theme}.png`) });
    await axe(page, `messages panel ${theme}`);
    await page.getByTestId("pool-panel-tasks").click();
    await expect(page.getByTestId("pool-panel-tasks-open")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("pool-panel-tasks-open")).toHaveCount(0);
  });

  test(`a new pool — 1280x800 ${theme}`, async ({ page }) => {
    await open(page, theme, 1280, 800);
    await poolsScreen(page);
    await page.getByTestId("pools-new").click();
    await expect(page.getByTestId("canvas-empty")).toBeVisible();
    await expect(page.getByTestId("pool-name-input")).toBeFocused();
    await expect(page.getByTestId("pool-name-input")).toHaveValue("New pool");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `new-pool-1280x800-${theme}.png`) });
    await axe(page, `new pool ${theme}`);
  });
}

test("drag: a session from the palette in, a node to a new place", async ({ page }) => {
  await open(page, "dark", 1280, 800);
  await canvas(page);
  await palette(page);
  await expect(page.getByTestId("canvas-palette-item")).toHaveCount(2);
  const item = page.getByTestId("canvas-palette-item").filter({ hasText: "Price API tests" });
  const from = (await item.boundingBox())!;
  const area = (await page.getByTestId("pool-canvas").boundingBox())!;
  await page.mouse.move(from.x + 40, from.y + 12);
  await page.mouse.down();
  await page.mouse.move(area.x + area.width - 200, area.y + area.height - 120, { steps: 8 });
  await expect(page.getByTestId("canvas-drag-ghost")).toBeVisible();
  await page.mouse.up();
  await expect(page.getByTestId("canvas-node")).toHaveCount(7);
  expect(await calls(page, "__AIJ_POOL_CALLS__")).toContain("move:s-tests:pool-checkout");
  // The first arrangement keeps every node where it is.
  const placed = (await calls(page, "__AIJ_POOL_CANVAS_CALLS__")).join("\n");
  expect(placed).toContain("s-tests@");
  expect(placed).toContain("s-plan@");

  const node = page.locator('[data-node="s-cart"]');
  const box = (await node.boundingBox())!;
  await page.mouse.move(box.x + 60, box.y + 14);
  await page.mouse.down();
  await page.mouse.move(box.x + 160, box.y + 74, { steps: 6 });
  await page.mouse.up();
  expect((await calls(page, "__AIJ_POOL_CANVAS_CALLS__")).at(-1)).toMatch(/^place:pool-checkout:s-cart@/);
});

test("keyboard: arrows between nodes, Enter, Option+arrows, Delete with Undo, an ask in place", async ({ page }) => {
  await open(page, "dark", 1280, 800);
  await canvas(page);
  const planner = page.locator('[data-node-name="s-plan"]');
  await planner.focus();
  await page.keyboard.press("ArrowDown");
  const focused = await page.evaluate(() => document.activeElement?.getAttribute("data-node-name"));
  expect(focused).not.toBe("s-plan");
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("inspect-node")).toBeVisible();
  await expect(page.locator("#pool-inspector-title")).toBeFocused();

  await page.locator('[data-node-name="s-cart"]').focus();
  await page.keyboard.press("Alt+ArrowRight");
  expect((await calls(page, "__AIJ_POOL_CANVAS_CALLS__")).join("\n")).toMatch(/s-cart@/);

  await page.locator('[data-node-name="s-spike"]').focus();
  await page.keyboard.press("Delete");
  await expect(page.getByTestId("canvas-node")).toHaveCount(5);
  await page.getByTestId("canvas-removed").click();
  await expect(page.getByTestId("canvas-node")).toHaveCount(6);

  // At fit the node is compact: its badge opens the ask, answered there.
  await page.locator('[data-node="s-copy"]').getByTestId("node-badge").focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Enter");
  await expect(page.locator('[data-node="s-copy"]').getByTestId("node-ask")).toHaveCount(0);
  expect(await calls(page, "__AIJ_MESSAGES_CALLS__")).toContain("answer:ask-copy:once");
});

test("semantic zoom: Tidy up and Fit in both forms, places kept across the switch", async ({ page }) => {
  await open(page, "dark", 1280, 800);
  await canvas(page);
  const before = await page.locator('[data-node="s-cart"]').boundingBox();
  await fullCards(page);
  await page.getByTestId("canvas-fit").click();
  await expect(page.locator("[data-form='compact']").first()).toBeVisible();
  // The same box at fit, whichever form it was in before.
  const after = await page.locator('[data-node="s-cart"]').boundingBox();
  expect(Math.round(after!.width)).toBe(Math.round(before!.width));
  await page.getByTestId("canvas-tidy").click();
  await expect(page.getByTestId("canvas-node")).toHaveCount(6);
  await fullCards(page);
  await page.getByTestId("canvas-tidy").click();
  await expect(page.getByTestId("canvas-node")).toHaveCount(6);
});
