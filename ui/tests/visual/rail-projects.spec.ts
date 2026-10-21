/**
 * 0.0.8 dogfood: setups brought in, nothing run in AI Juggler yet — the
 * rail listed every project from their history as a bare row with "0".
 * Now: a short "Projects" heading, rows built like session rows (a folder
 * mark, the name, what was done there), clones of one repository grouped,
 * everything foldable and remembered, no "0".
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { readyHome, type FixtureSet } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/74-rail-projects");
const T0 = 1_728_800_000_000;
const H = 3_600_000;

const project = (id: string, name: string, root: string, extra: Record<string, unknown> = {}) => ({
  id,
  root,
  name,
  last_activity_ms: T0,
  active_instance: null,
  accounts_used: ["Work Max"],
  instance_count: 0,
  history_sessions: 4,
  history_last_used_ms: T0 - 2 * H,
  repo: null,
  missing: false,
  ...extra,
});

const historyOnly: FixtureSet = {
  ...readyHome,
  projects: [
    project("p1", "ai-juggler", "/Users/dev/Dev/ai-juggler", { history_sessions: 41, accounts_used: ["Work Max", "Personal", "Side"], history_last_used_ms: T0 - 20 * 60_000, repo: "github.com/you/ai-juggler" }),
    project("p2", "booknyx3", "/Users/dev/Dev/booknyx/booknyx3", { history_sessions: 12, history_last_used_ms: T0 - 26 * H, repo: "github.com/you/booknyx" }),
    project("p3", "booknyx4", "/Users/dev/Dev/booknyx/booknyx4", { history_sessions: 3, history_last_used_ms: T0 - 50 * H, repo: "github.com/you/booknyx" }),
    project("p4", "jive-code-graph", "/Users/dev/Dev/jive-code-graph", { history_sessions: 9, history_last_used_ms: T0 - 5 * H }),
    project("p5", "t3-connection-check", "/Users/dev/Documents/scratch/t3-connection-check", { history_sessions: 1, history_last_used_ms: T0 - 9 * 24 * H }),
    project("p6", "notes", "/Users/dev/notes", { history_sessions: 2, history_last_used_ms: T0 - 30 * 24 * H }),
    project("p7", "old-site", "/Users/dev/old-site", { history_sessions: 1, history_last_used_ms: T0 - 90 * 24 * H }),
  ],
} as unknown as FixtureSet;

async function seed(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: historyOnly, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

for (const theme of ["dark", "light"] as const) {
  test(`rail — projects from history, grouped, foldable — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    const list = page.getByTestId("rail-history-projects");
    await expect(list).toBeVisible();
    await expect(list).toContainText("Projects");
    // Never a bare "0" and never a long description.
    await expect(list).not.toContainText("from your Claude history");
    const rows = page.getByTestId("rail-history-project");
    await expect(rows.first()).toContainText("ai-juggler");
    await expect(rows.first()).toContainText("20 min ago · 41 conversations · 3 accounts");
    for (const text of await page.getByTestId("rail-project-stats").allInnerTexts()) expect(text).not.toBe("0");
    // Clones of one repository sit under it.
    const repo = page.getByTestId("rail-repo-group");
    await expect(repo).toHaveCount(1);
    await expect(repo).toContainText("booknyx");
    await expect(repo.getByTestId("rail-history-project")).toHaveCount(2);
    // Five groups, then Show all.
    await expect(page.getByTestId("rail-history-all")).toHaveText("Show all 6");
    await expect(list).toContainText("Projects6");
    await page.screenshot({ path: resolve(OUT, `history-1280x800-${theme}.png`) });
    const results = await new AxeBuilder({ page }).include('[data-testid="rail-history-projects"]').analyze();
    expect(results.violations.filter((v) => v.impact === "critical" || v.impact === "serious").map((v) => v.id)).toEqual([]);

    // Fold the list; it stays folded after a reload.
    await page.getByRole("button", { name: "Fold Projects" }).click();
    await expect(rows.first()).toBeHidden();
    await page.reload();
    await expect(page.getByRole("button", { name: "Unfold Projects" })).toHaveAttribute("aria-expanded", "false");
    await page.getByRole("button", { name: "Unfold Projects" }).click();
    await expect(rows.first()).toBeVisible();
  });
}
