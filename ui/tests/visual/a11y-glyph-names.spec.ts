/**
 * D-117 (AJ-E3602/E3603), the a11y fix of 2026-11-15 — part of the a11y
 * gate (scripts/a11y_gate.sh). Happo's accessibility snapshots of
 * 83f700ed6 read the rail's head as "Needs you ✕ 5": the L3 glyph, drawn
 * as the count's ::before, was part of its accessible name (VoiceOver:
 * "multiplication x"). A level's glyphs are decoration; the words carry
 * the level ("Needs you 3, 1 stopped with an error, 2 wait on you"; the
 * visible words first, WCAG 2.5.3).
 *
 * Read from Chromium's own accessibility tree (CSS generated content
 * included, as Happo and a screen reader read it): on Home with the rail,
 * on a fixture with L1 (to review), L2 (asks, probably stuck) and L3
 * (stopped with an error) things, and with a folded project in the
 * Project lens, no node's name holds a status glyph; and axe stays clean.
 */
import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { expectRail } from "./fixtures";
import { placesConversations } from "../../src/preview/places";

/** The status glyphs no accessible name may hold. */
const GLYPHS = /[✕⚠○●◐✓]/u;

const T0 = new Date(2026, 9, 24, 15, 0, 0).getTime();
const ACCOUNTS = [{ id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false }];
const PROJECTS = [{ id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false }];

function own(id: string, over: Record<string, unknown> = {}) {
  return {
    id, launch_recipe_id: "p-1", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: "opus", launch_model: "opus",
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: T0 - 600_000,
    last_activity_ms: T0 - 60_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  };
}

/** L3, L2 (a guess and a reply), calm; the preview's places add asks and a review (L1). */
const FIXTURE = {
  accounts: ACCOUNTS,
  projects: PROJECTS,
  sessions: [own("broke", { status: "failed", needs_attention: "failed" }), own("stuck", { status: "stuck", needs_attention: "stuck" }), own("wait", { status: "waiting_user", needs_attention: "waiting_user" }), own("calm")],
  conversations: placesConversations(T0),
};

async function open(page: Page, theme: "dark" | "light", group: "app" | "project", width = 1440) {
  await page.addInitScript(
    ({ set, theme: t, t0, lens }) => {
      (window as unknown as Record<string, unknown>).__AIJ_FIXTURE__ = set;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.rail.view.v1", JSON.stringify({ group: lens }));
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { set: FIXTURE, theme, t0: T0, lens: group },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/");
  await expectRail(page);
  await expect(page.getByTestId("home-heading")).not.toHaveText(/Checking/);
  // The backend's needs are in (the L3 one drawn somewhere: a row, a count).
  await expect(page.locator("[data-level='3']").first()).toBeAttached();
}

/** Every node's name in Chromium's accessibility tree that holds a glyph
 * (the page's own computation: generated content, aria-hidden, labels). */
async function glyphNames(page: Page): Promise<string[]> {
  const cdp = await page.context().newCDPSession(page);
  const { nodes } = (await cdp.send("Accessibility.getFullAXTree")) as { nodes: { ignored?: boolean; role?: { value?: string }; name?: { value?: string } }[] };
  await cdp.detach();
  return nodes
    .filter((node) => !node.ignored && GLYPHS.test(String(node.name?.value ?? "")))
    .map((node) => `${node.role?.value ?? "?"}: ${node.name?.value}`);
}

async function axeSerious(page: Page) {
  const scan = async () => (await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"]).analyze()).violations.filter((v) => ["serious", "critical"].includes(v.impact ?? ""));
  let serious = await scan();
  // AJ-E806: one confirmatory re-scan after a settle beat.
  if (serious.length > 0) {
    await page.waitForTimeout(400);
    serious = await scan();
  }
  return serious.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

for (const theme of ["dark", "light"] as const) {
  test(`status glyphs — no accessible name holds one; axe 0 — Home and the rail by App — ${theme}`, async ({ page }) => {
    await open(page, theme, "app");
    // The head says its levels in words, as the tree reads it.
    const head = page.getByTestId("rail-needs-you");
    await expect(head.locator(".level-glyph[data-level='3']")).toBeVisible();
    await expect(page.getByRole("heading", { name: /^Needs you \d+, 1 stopped with an error, \d+ wait on you$/ })).toBeVisible();
    expect(await glyphNames(page)).toEqual([]);
    expect(await axeSerious(page)).toEqual([]);
  });

  test(`status glyphs — no accessible name holds one; axe 0 — a folded project — ${theme}`, async ({ page }) => {
    await open(page, theme, "project");
    // D-117: the Project lens's head is the App lens's, said in words.
    await expect(page.getByRole("heading", { name: /^Needs you \d+, 1 stopped with an error, \d+ wait on you$/ })).toBeVisible();
    await page.getByRole("button", { name: "Fold ai-juggler" }).click();
    const mark = page.getByTestId("rail-folded-needs");
    await expect(mark).toHaveAttribute("data-level", "3");
    // A mark inside the project's button (never a button in a button): the button says it.
    await expect(page.getByTestId("rail-project")).toHaveAccessibleName(/^ai-juggler 3 need you, 1 stopped with an error, /);
    expect(await glyphNames(page)).toEqual([]);
    expect(await axeSerious(page)).toEqual([]);
  });

  test(`status glyphs — no accessible name holds one; axe 0 — the folded icon rail — ${theme}`, async ({ page }) => {
    await open(page, theme, "app", 1000);
    const rail = page.getByTestId("rail-folded");
    await expect(rail).toBeVisible();
    // Its corners (! at L2, ✕ at L3) are decoration; its names say the word.
    await expect(rail.locator("[data-testid='rail-folded-needs'][data-level='2']").first()).toBeVisible();
    await expect(rail.getByRole("button", { name: /^Probably stuck, / })).toBeVisible();
    expect(await glyphNames(page)).toEqual([]);
    expect(await axeSerious(page)).toEqual([]);
  });
}
