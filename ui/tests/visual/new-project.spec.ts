import { expect, test } from "@playwright/test";
import { allStatuses, type FixtureSet } from "./fixtures";

/**
 * AJ-E305 quality gate — flow (b) as a TESTED CONTRACT: New project
 * in ≤ 3 clicks (tile → Start session →), opened with ⌘N from
 * anywhere, the recommended account preselected (last-used on the
 * dir), and the funnel seam complete (open → pick-dir → launch).
 * The no-Account state detours inline into the add-account flow and
 * keeps the pending directory (Req.7).
 */

const noProfiles: FixtureSet = {
  ...allStatuses,
  accounts: [],
  profiles: [],
  projects: [
    {
      id: "proj-1",
      root: "/Users/dev/ai-juggler",
      name: "ai-juggler",
      last_activity_ms: 1_728_800_000_000,
      active_instance: null,
      accounts_used: [],
      instance_count: 0,
      history_sessions: 0,
    },
  ],
};

for (const theme of ["dark", "light"] as const) {
  test(`flow (b): ⌘N → tile → Start session in ≤3 clicks, funnel complete — ${theme}`, async ({
    page,
  }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        const log: string[] = [];
        w.__funnelLog = log;
        window.addEventListener("aij:funnel", (event) => {
          const detail = (event as CustomEvent<{ flow: string; step: string }>).detail;
          log.push(`${detail.flow}:${detail.step}`);
        });
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();

    // ⌘N is a keyboard summon — not a click.
    await page.keyboard.press("Meta+n");
    await expect(page.getByRole("heading", { name: "New project" })).toBeVisible();

    let clicks = 0;
    // Click 1: the recent-dir tile (ai-juggler, last used with Work Max).
    await page
      .getByRole("button", { name: /Work in ai-juggler — last used with Work Max/ })
      .click();
    clicks += 1;
    await expect(page.getByTestId("project-account")).toBeVisible();
    // The preselect rule picked the account last used on this dir.
    await expect(page.getByTestId("project-account")).toContainText("Work Max");
    await expect(page.getByTestId("project-account")).toContainText("last used here");

    // Click 2: Start session → (the recommended recipe + the dir).
    await page.getByTestId("start-session").click();
    clicks += 1;
    // Preview mode has no backend — the honest error, after the
    // launch was attempted (the click contract is what's pinned).
    await expect(page.getByText("Preview mode — no backend connected.")).toBeVisible();

    expect(clicks).toBeLessThanOrEqual(3);
    expect(clicks).toBe(2);

    const funnel = (await page.evaluate(() =>
      (window as unknown as { __funnelLog: string[] }).__funnelLog,
    )) as string[];
    for (const step of ["b-new-project:open", "b-new-project:pick-dir", "b-new-project:launch"]) {
      expect(funnel).toContain(step);
    }
    const indexes = ["b-new-project:open", "b-new-project:pick-dir", "b-new-project:launch"].map(
      (step) => funnel.indexOf(step),
    );
    expect(indexes).toEqual([...indexes].sort((a, b) => a - b));
  });

  test(`flow (b): no Account detours inline and keeps the pending project — ${theme}`, async ({
    page,
  }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_RECIPES__ = [];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: noProfiles, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.keyboard.press("Meta+n");
    await expect(page.getByRole("heading", { name: "New project" })).toBeVisible();
    await page.getByRole("button", { name: /Work in ai-juggler/ }).click();
    await expect(page.getByTestId("no-account-detour")).toBeVisible();
    await page.getByRole("button", { name: "Add an account" }).click();
    // The add-account flow (d) opens INLINE — over the sheet, which
    // keeps the pending directory (Req.7).
    await expect(page.getByTestId("onboarding-wizard")).toBeVisible();
    await expect(page.getByText("Starting in ai-juggler")).toBeVisible();
  });
}
