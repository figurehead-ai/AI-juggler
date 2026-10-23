/**
 * AJ-E1520 gallery: a session that started subagents. The status row's
 * count (two still running) and, on the session's page, the Subagents
 * list opened: what each was asked, its kind, and where it stands. Both
 * themes, axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, expectRail } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/88-subagents");
const T0 = 1_728_800_000_000;
const SESSION = "sess-2";

const FIXTURE = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) => (session.id === SESSION ? { ...session, subtask_count: 2 } : session)),
  subtasks: {
    [SESSION]: {
      live: true,
      items: [
        { id: "t1", label: "Map every transcript parser", kind: "Explore", state: "done" },
        { id: "t2", label: "Review how parse errors are reported", kind: "code-reviewer", state: "failed" },
        { id: "t3", label: "Run the full test suite and report failures", kind: "general-purpose", state: "running" },
        { id: "t4", label: "Write tests for the tail reader", kind: "general-purpose", state: "running" },
      ],
    },
  },
};

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.density", "comfortable");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: FIXTURE, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expectRail(page);
}

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-1280x800-${theme}.png`) });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`the session's row says two subagents run, and its page lists them — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await expect(page.getByTestId("rail-subagents")).toHaveText("· 2 subagents");
    await shoot(page, "row-count", theme);
    await page.locator(`[data-instance-id="${SESSION}"]`).first().click();
    const section = page.getByTestId("subagents-section");
    await expect(section).toBeVisible();
    await expect(section).toContainText("2 running · 4 in all");
    await section.locator("summary").click();
    await expect(page.getByTestId("subagent-row")).toHaveCount(4);
    await expect(page.getByTestId("subagent-state")).toHaveText(["done", "failed", "running", "running"]);
    await shoot(page, "session-subagents", theme);
  });
}
