/**
 * AJ-E3011, AJ-E3012 gallery (0.4.6): the pool setup a newcomer
 * understands, and the canvas's selection.
 *
 * - The setup, every step, empty (the first pool: the intro full, nothing
 *   chosen, the defaults filled in) and filled (a name, two Apps, an App's
 *   next session leading, Reviewers joining, a pair blocked, waking on,
 *   the seal), both themes.
 * - The canvas with each kind selected — the pool, a session, an App, an
 *   Account, a Tool, everyone in scope, a line, the pool's own setup, and
 *   the lead's place waiting for an App's next session — both themes:
 *   the inspector titled by the selection, the one selection ring.
 *
 * Invented names and fixture data only. 1280×800; axe 0 critical/serious
 * on each capture.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { goTo } from "./fixtures";
import { canvasSeed } from "./fixtures/pool-canvas";

const SETUP = resolve(process.cwd(), "../docs/qa/evidence/_gallery/98-pool-setup");
const SELECTION = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-pool-selection");
test.beforeAll(() => {
  mkdirSync(SETUP, { recursive: true });
  mkdirSync(SELECTION, { recursive: true });
});

const STEPS = ["template", "where", "who", "lead", "everyone", "talk", "own", "review"] as const;
const POOL = "pool-checkout";
const CREW = "pool-crew";

/** The canvas pool's member rules (an App, an Account, a Tool, everyone)
 * and a second pool that waits for an App's next session to lead. */
function selectionSeed(t0: number) {
  const seed = canvasSeed(t0);
  const rule = (member: string, kind: string, role: string, ids: string[] = []) => ({ member, kind, role, level: null, joined_at: t0 - 3_600_000, sessions: ids.length, session_ids: ids });
  return {
    ...seed,
    __AIJ_POOLS__: {
      pools: [...seed.__AIJ_POOLS__.pools, { id: CREW, name: "Review crew", rules: [] }],
      members: [...seed.__AIJ_POOLS__.members, { session_id: "s-tests", pool_id: CREW, role: "member" }],
    },
    __AIJ_POOL_RULES__: {
      [POOL]: {
        members: [
          rule("app:cli:claude-personal", "app", "reviewer"),
          rule("account:acct:preview:k-claude-work", "account", "member", ["s-cart"]),
          rule("tool:codex", "tool", "observer", ["s-copy"]),
          rule("all", "everyone", "member"),
        ],
        talk: [{ from: "role:member", to: "role:member", rule: "ask" }],
      },
      [CREW]: { members: [rule("app:cli:claude-work", "app", "member", ["s-tests"])], lead_next: "cli:claude-work" },
    },
  };
}

async function open(page: Page, theme: "dark" | "light", seed: Record<string, unknown> | null) {
  await page.clock.setFixedTime(GATE_NOW);
  await page.addInitScript(
    ({ theme, seed }) => {
      if (seed) Object.assign(window, seed);
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, seed },
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

async function shoot(page: Page, dir: string, name: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(dir, name) });
  await axe(page, name);
}

async function newPool(page: Page) {
  await goTo(page, "Pools");
  await expect(page.getByTestId("pools-screen")).toBeVisible();
  await page.getByTestId("pools-new").click();
  await expect(page.getByTestId("new-pool")).toBeVisible();
}

/** Fills the step on screen as a newcomer would, from the walks' goal:
 * two assistants, one in charge, one checking. */
async function fill(page: Page, step: (typeof STEPS)[number]) {
  if (step === "where") await page.getByTestId("new-pool-name").fill("Storefront team");
  if (step === "who") {
    for (const row of await page.getByTestId("new-pool-candidate").all()) await row.getByRole("checkbox").click();
  }
  if (step === "lead") {
    await page.getByTestId("new-pool-lead").getByText(/'s next session$/).first().click();
    await expect(page.locator('[data-testid=new-pool-lead-option][data-kind="next"]').first()).toBeChecked();
  }
  if (step === "everyone") {
    await page.getByTestId("new-pool-member-role").last().click();
    await page.getByRole("menuitemradio", { name: /^Reviewer/ }).click();
  }
  if (step === "talk") await page.locator('[data-testid=new-pool-talk-cell][data-pair="member>reviewer"]').click();
  if (step === "own") {
    await page.getByTestId("new-pool-level-choice-full").click();
    await page.getByTestId("new-pool-seal").getByRole("checkbox").check();
  }
}

for (const theme of ["dark", "light"] as const) {
  test(`the setup, every step, empty: the first pool — ${theme}`, gate(`98-pool-setup-empty-${theme}`), async ({ page }) => {
    await open(page, theme, null);
    await newPool(page);
    // The first pool: the intro is full.
    await expect(page.getByTestId("new-pool-intro")).toHaveAttribute("data-open", "true");
    for (const step of STEPS) {
      await expect(page.getByTestId(`new-pool-step-${step}`)).toBeVisible();
      await shoot(page, SETUP, `setup-${step}-empty-1280x800-${theme}.png`);
      if (step !== "review") await page.getByTestId("new-pool-next").click();
    }
    await expect(page.getByTestId("new-pool-review")).toContainText("New sessions in");
  });

  test(`the setup, every step, filled — ${theme}`, gate(`98-pool-setup-filled-${theme}`), async ({ page }) => {
    await open(page, theme, canvasSeed(GATE_NOW));
    await newPool(page);
    // A pool exists: the intro is one line.
    await expect(page.getByTestId("new-pool-intro").locator("summary")).toContainText("What a pool is");
    for (const step of STEPS) {
      await expect(page.getByTestId(`new-pool-step-${step}`)).toBeVisible();
      await fill(page, step);
      await shoot(page, SETUP, `setup-${step}-filled-1280x800-${theme}.png`);
      if (step !== "review") await page.getByTestId("new-pool-next").click();
    }
    await expect(page.getByTestId("new-pool-review")).toContainText("next session here leads");
    await expect(page.getByTestId("new-pool-review")).toContainText("Sealed");
  });

  test(`the canvas, each kind selected — ${theme}`, gate(`99-pool-selection-${theme}`), async ({ page }) => {
    await open(page, theme, selectionSeed(GATE_NOW));
    await goTo(page, "Pools");
    await page.getByTestId("pool-card-tile").filter({ hasText: "Checkout team" }).click();
    await expect(page.getByTestId("pool-canvas")).toBeVisible();
    await expect(page.getByTestId("canvas-node")).toHaveCount(6);
    const title = page.getByTestId("inspector-title");
    await expect(title).toHaveText("Checkout team");
    await shoot(page, SELECTION, `selection-pool-1280x800-${theme}.png`);

    await page.locator('[data-node-name="s-plan"]').click();
    await expect(title).toHaveText("Planner");
    await expect(page.locator('[data-node="s-plan"]')).toHaveAttribute("aria-current", "true");
    await shoot(page, SELECTION, `selection-session-1280x800-${theme}.png`);

    for (const [ref, name, file] of [
      ["app:cli:claude-personal", "Claude Code CLI · Personal", "app"],
      ["account:acct:preview:k-claude-work", null, "account"],
      ["tool:codex", "Codex", "tool"],
      ["all", "Everyone, in every project", "everyone"],
    ] as const) {
      await page.locator(`[data-rule-ref="${ref}"]`).scrollIntoViewIfNeeded();
      await page.locator(`[data-rule-ref="${ref}"]`).click();
      if (name) await expect(title).toHaveText(name);
      await expect(page.locator(`[data-rule-ref="${ref}"]`)).toHaveAttribute("aria-current", "true");
      await shoot(page, SELECTION, `selection-${file}-1280x800-${theme}.png`);
    }

    // A line, from the pool's live stream.
    await page.getByTestId("inspector-back").click();
    await page.getByTestId("inspect-stream").getByRole("button", { name: /Planner wrote to Fix the cart totals/ }).click();
    await expect(title).toHaveText("Planner → Fix the cart totals");
    await shoot(page, SELECTION, `selection-line-1280x800-${theme}.png`);

    // The pool's own setup, from the checklist: who leads.
    await page.getByTestId("inspector-back").click();
    await page.locator('[data-testid=pool-checklist-step][data-step="talk"]').click();
    await expect(page.getByTestId("pool-setup-talk")).toBeVisible();
    await shoot(page, SELECTION, `selection-pool-setup-1280x800-${theme}.png`);

    // A pool waiting for an App's next session to lead: its place on the canvas.
    await goTo(page, "Pools");
    await page.getByTestId("pool-card-tile").filter({ hasText: "Review crew" }).click();
    await page.getByTestId("canvas-lead-card").click();
    await expect(title).toHaveText("Claude Code CLI · Work Max's next session");
    await shoot(page, SELECTION, `selection-lead-next-1280x800-${theme}.png`);
  });
}
