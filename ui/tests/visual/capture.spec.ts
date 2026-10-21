/**
 * AJ-E802 — visual states harness.
 *
 * Captures the seed-set screens (WorkflowRail with all 9 session
 * statuses, session/inspector panel, onboarding detect) across the
 * required viewport × theme matrix against the mocked bridge with
 * deterministic fixtures. States that do not exist yet are recorded as
 * NOT CAPTURED with the owning task — never silently omitted.
 *
 * Run: pnpm --dir ui visual:capture   (writes docs/qa/evidence/_gallery/)
 */
import { test, expect } from "./axe-gallery";
import { mkdirSync, writeFileSync, existsSync, appendFileSync } from "node:fs";
import { resolve } from "node:path";

// Playwright runs with cwd = config dir (ui/).
const GALLERY_ROOT = resolve(process.cwd(), "../docs/qa/evidence/_gallery");
import { allStatuses, cleanupFixture, honestEmpty, readyHome, goTo, openFilters, openSwitch, expectRail, openRailList } from "./fixtures";

const GALLERY = GALLERY_ROOT;
const NOT_CAPTURETED = resolve(GALLERY, "not-captured.md");

const VIEWPORTS = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x680", width: 1024, height: 680 }, // Tauri minimum
  { name: "390x844", width: 390, height: 844 }, // collapsed-rail check
] as const;

const THEMES = ["dark", "light"] as const;

const T0 = 1_728_800_000_000;

function noteNotCaptured(screen: string, state: string, reason: string) {
  appendFileSync(
    NOT_CAPTURETED,
    `| ${screen} | ${state} | NOT CAPTURED: ${reason} |\n`,
  );
}

test.beforeAll(() => {
  mkdirSync(GALLERY, { recursive: true });
  for (const screen of ["01-rail", "01-rail-density", "01-rail-sort", "05-session-panel", "06-inspector", "07-inspector-content", "08-inspector-aggregate", "09-quality", "10-processes", "11-renewal", "12-model", "13-desktop", "14-focus", "15-preview", "17-ledger", "18-ports", "19-cleanup", "20-kill", "21-tiers", "22-recovery", "23-alerts", "24-fleet", "25-costs", "26-quota", "27-ranking", "28-switch", "29-door", "30-switchback", "31-redaction", "32-graph", "33-beta", "37-flows", "38-flows-cde", "39-import", "40-login", "41-policy", "42-fleet", "43-pairing", "44-settings", "45-remote", "46-fleet-board", "47-remote-focus", "48-remote-cleanup", "49-fleet-sync", "50-cross-switch", "51-identity", "52-worktree", "53-parked", "54-summary", "55-triage", "56-history", "57-backup", "58-updates", "59-vault", "60-cloud-backup", "61-account-macs", "71-add-account", "72-tool-view", "75-feedback", "76-keys"]) {
    mkdirSync(resolve(GALLERY, screen), { recursive: true });
  }
  writeFileSync(NOT_CAPTURETED, "# Not captured (explicit omissions)\n\n| Screen | State | Reason |\n|---|---|---|\n");
  // Loading/error states do not exist yet — recorded, not hidden.
  // AJ-E1503: the rail's loading placeholders are captured in 75-feedback.
  noteNotCaptured("05-session-panel", "loading (skeleton)", "pending AJ-E405+ (state emission not yet implemented)");
  for (const screen of ["01-rail", "05-session-panel"]) {
    noteNotCaptured(screen, "error (banner + retained data)", "pending AJ-E405+ (error states not yet implemented)");
  }
});

for (const theme of THEMES) {
  for (const vp of VIEWPORTS) {
    test(`rail — all statuses — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme }) => {
          (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", theme);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          localStorage.setItem("aij.ready.skipped", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          // Pin the clock for deterministic "ago" labels.
          Date.now = () => t0;
        },
        { fixture: allStatuses, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      if (vp.width < 900) {
        // D-25 (AJ-E1020): narrow windows fold the rail to its icon column.
        await expect(page.getByTestId("rail")).toHaveAttribute("data-density", "minimal");
        await expect(page.getByTestId("rail").getByRole("button", { name: /Work Max/ }).first()).toBeVisible();
      } else {
        await expectRail(page); // D-25: the list, or its icon column when narrow
        await expect(page.getByText("Handed over").first()).toBeVisible();
      }
      await page.screenshot({
        path: resolve(GALLERY, `01-rail/all-statuses-${vp.name}-${theme}.png`),
        fullPage: false,
      });
    });

    test(`session panel — populated — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme }) => {
          (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", theme);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      // D-25 (AJ-E1020): below 900 px the rail is its icon column.
      await expect(page.getByTestId("rail")).toHaveAttribute("data-density", vp.width < 900 ? "minimal" : "comfortable");
      // The row carries its account in the aria-label (initials disc on
      // screen) — click by accessible name (AJ-E402).
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await expect(page.getByTestId("handover-section")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `05-session-panel/populated-${vp.name}-${theme}.png`),
      });
    });

    test(`rail — honest empty — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, theme }) => {
          (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", theme);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: honestEmpty, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      // AJ-E406: the empty work area is the fleet-at-a-glance summary
      // (RightInspector, Req.4) — never a blank panel.
      // D-25: with no accounts, Home is one plain next step.
      await expect(page.getByTestId("home-empty-accounts")).toBeVisible();
      await expect(page.getByRole("button", { name: "Find my accounts" })).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `01-rail/honest-empty-${vp.name}-${theme}.png`),
      });
    });

    test(`ready home — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, theme }) => {
          (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", theme);
          localStorage.removeItem("aij.onboarding.dismissed");
        },
        { fixture: readyHome, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      // D-25: an account that needs its sign-in is a "Needs you" item on Home.
      await expect(page.getByTestId("home-signin").first()).toContainText("needs its sign-in");
      await page.screenshot({
        path: resolve(GALLERY, `01-rail/ready-home-${vp.name}-${theme}.png`),
      });
    });

  }
}

// AJ-E403 density captures: 3 densities × (1440x900, 1024x768), the
// Minimal hover tooltip, and one stale row (freshness forced old via the
// __AIJ_SAMPLED_AT__ fixture hook).
const DENSITY_VIEWPORTS = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1024x768", width: 1024, height: 768 },
] as const;
const DENSITIES = ["comfortable", "compact", "minimal"] as const;

for (const density of DENSITIES) {
  for (const vp of DENSITY_VIEWPORTS) {
    test(`rail density ${density} — ${vp.name}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, mode }) => {
          (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", "dark");
          localStorage.setItem("aij.density", mode);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t0: T0, mode: density },
      );
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      if (density !== "minimal") await expectRail(page); // D-25: the list, or its icon column when narrow
      if (density === "minimal") {
        await expect(page.getByTitle(/Work Max/).first()).toBeVisible();
      } else {
        await expect(page.getByText("Handed over").first()).toBeVisible();
      }
      await page.screenshot({
        path: resolve(GALLERY, `01-rail-density/${density}-${vp.name}.png`),
      });
    });
  }
}

test("rail minimal — hover tooltip open", async ({ page }) => {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.density", "minimal");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  const first = page.locator("aside button[title]").first();
  await first.hover();
  await page.waitForTimeout(150);
  await page.screenshot({ path: resolve(GALLERY, "01-rail-density/minimal-tooltip-1440x900.png") });
});

test("rail — stale row renders stale - Nm", async ({ page }) => {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
      // The running instance's samples are 3 minutes old — past 2× the
      // 5 s resource and 30 s quota intervals → `stale - 3m`.
      (window as never as { __AIJ_SAMPLED_AT__: unknown }).__AIJ_SAMPLED_AT__ = {
        "sess-2": t0 - 3 * 60_000,
      };
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await expect(page.getByText("stale - 3m").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "01-rail-density/stale-row-1440x900.png") });
});

// AJ-E404 captures: the sorted/filtered rail states (1440x900).
test("rail — needs-me sort default", async ({ page }) => {
  await seedRail(page, {});
  await expect(page.getByText("Handed over").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "01-rail-sort/needs-me-1440x900.png") });
});

test("rail — quota risk with no data group", async ({ page }) => {
  await seedRail(page, {});
  await openFilters(page);
  // The "no data" group is the quota sort's own grouping (no project groups).
  await page.selectOption('select[aria-label="Group by"]', "none");
  await page.selectOption('select[aria-label="Sort by"]', "quota");
  await expect(page.getByText("Not measured yet").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "01-rail-sort/quota-risk-1440x900.png") });
});

test("rail — group by project with rollups", async ({ page }) => {
  await seedRail(page, {});
  await openFilters(page);
  await page.selectOption('select[aria-label="Group by"]', "project");
  await expect(page.getByText(/running/).first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "01-rail-sort/group-by-project-1440x900.png") });
});

test("rail — empty search state", async ({ page }) => {
  await seedRail(page, {});
  await page.locator('input[type="search"]').fill("does-not-exist");
  await expect(page.getByText("No sessions match")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "01-rail-sort/empty-search-1440x900.png") });
});

test("rail — snoozed badge (snoozed 12m)", async ({ page }) => {
  const t0 = T0;
  await seedRail(page, {
    snooze: { "sess-3": { until: t0 + 12 * 60_000, muted: false } },
  });
  await expect(page.getByText("snoozed 12m").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "01-rail-sort/snoozed-1440x900.png") });
});

/** Shared rail seeding for the sort/filter captures. */
async function seedRail(
  page: import("@playwright/test").Page,
  options: { snooze?: Record<string, { until: number | null; muted: boolean }> },
) {
  await page.addInitScript(
    ({ fixture, t0, snooze }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
      if (snooze) localStorage.setItem("aij.snooze", JSON.stringify(snooze));
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0, snooze: options.snooze },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await expectRail(page); // D-25: the list, or its icon column when narrow
}

// AJ-E406 inspector captures: the RightInspector shell across selection
// types (empty summary, Instance/Overview, Account/Usage honest empty,
// Project/Processes header) at 1440x900 + 1280x800.
const INSPECTOR_VIEWPORTS = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1280x800", width: 1280, height: 800 },
] as const;

async function seedInspectorPage(page: import("@playwright/test").Page, vp: { width: number; height: number }) {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
}

for (const vp of INSPECTOR_VIEWPORTS) {
  test(`inspector — fleet-at-a-glance (empty) — ${vp.name}`, async ({ page }) => {
    await seedInspectorPage(page, vp);
    await expect(page.getByTestId("home")).toBeVisible(); // D-25: Home replaces "fleet at a glance"
    await page.screenshot({ path: resolve(GALLERY, `06-inspector/empty-${vp.name}.png`) });
  });

  test(`inspector — Instance / Overview — ${vp.name}`, async ({ page }) => {
    await seedInspectorPage(page, vp);
    await expectRail(page); // D-25: the list, or its icon column when narrow
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await expect(page.getByRole("tab", { name: "Quality / Performance" })).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `06-inspector/instance-overview-${vp.name}.png`) });
  });

  test(`inspector — Account / Usage honest empty — ${vp.name}`, async ({ page }) => {
    await seedInspectorPage(page, vp);
    await goTo(page, "Accounts");
    await page.getByRole("button", { name: "Inspect Claude Code" }).first().click();
    await expect(page.getByRole("tab", { name: "Usage" })).toBeVisible();
    await page.getByRole("tab", { name: "Usage" }).click();
    await expect(page.getByText("No usage measured for this yet.", { exact: false })).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `06-inspector/account-usage-${vp.name}.png`) });
  });

  test(`inspector — Project / Processes — ${vp.name}`, async ({ page }) => {
    await seedInspectorPage(page, vp);
    await openRailList(page); // D-25: a project row needs the list
    await page.getByRole("button", { name: /^ai-juggler/ }).first().click();
    await expect(page.getByRole("tab", { name: "Processes" })).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `06-inspector/project-processes-${vp.name}.png`) });
  });
}

// AJ-E407 inspector-content captures (Instance + Session tabs).
test("inspector — Instance Overview fields (populated)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await expect(page.getByTestId("instance-overview-fields")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "07-inspector-content/instance-overview-populated.png") });
});

test("inspector — Instance Usage honest chart empties", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Usage" }).click();
  await expect(page.locator('section:not([hidden])').getByTestId("instance-usage").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "07-inspector-content/instance-usage.png") });
});

test("inspector — Instance Actions disabled-with-reason", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Actions" }).click();
  // AJ-E1025: an action that doesn't apply (Stop, once ended) is disabled
  // with its reason; unbuilt actions aren't shown.
  const panel = page.getByRole("tabpanel", { name: "Actions" });
  await expect(panel.getByRole("button", { name: /^Stop/ })).toBeDisabled();
  await expect(panel.getByText("not running")).toBeVisible();
  await expect(page.getByTestId("action-export-pack")).toHaveCount(0);
  await page.screenshot({ path: resolve(GALLERY, "07-inspector-content/instance-actions.png") });
});

test("inspector — Session Overview ended stamp", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  // An ended session (sess-7 stopped): its overview carries the stamp.
  await page.locator('[data-instance-id="sess-7"]').click();
  await expect(page.getByTestId("session-ended-stamp")).toContainText("Ended");
  await page.screenshot({ path: resolve(GALLERY, "07-inspector-content/session-overview.png") });
});

// AJ-E408 aggregate-content captures.
test("inspector — Account Overview rings + forecast", async ({ page }) => {
  // AJ-E610: the rings are engine-driven now — seed the display seam
  // (badge-adjacent % + reset, the honest typed states).
  await page.addInitScript(
    ({ t0 }) => {
      (window as never as { __AIJ_QUOTA_DISPLAY__: unknown }).__AIJ_QUOTA_DISPLAY__ = [
        {
          account: "claude-work",
          five_hour: { state: "tracking", percentage: 68, badge: "measured", resets_at_ms: t0 + 2 * 3_600_000, burned_tokens: 340_000 },
          weekly: { state: "tracking", percentage: 31, badge: "measured", resets_at_ms: t0 + 3 * 86_400_000, burned_tokens: 340_000 },
          renewal: { state: "set", label: "Renews 2026-10-23", entered_by_user: true },
          forecast: { state: "exhausts", tte_min: 96, range_min: 70, range_max: 130, badge: "measured" },
        },
      ];
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { t0: T0 },
  );
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await goTo(page, "Accounts");
  await page.getByRole("button", { name: "Inspect Claude Code" }).first().click();
  await expect(page.getByTestId("account-rings")).toBeVisible();
  await expect(page.getByText(/5-hour window · Measured/)).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "08-inspector-aggregate/account-overview-1440x900.png") });
});

test("inspector — Account logged-out collapse", async ({ page }) => {
  // Override: the Personal account is signed out in this capture.
  await page.addInitScript(
    ({ fixture, t0 }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = {
        ...fixture,
        accounts: fixture.accounts.map((a: { id: string }) =>
          a.id === "claude-personal" ? { ...a, login_state: "logged_out" } : a,
        ),
      };
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await goTo(page, "Accounts");
  await page.getByRole("button", { name: "Inspect Claude Code" }).nth(1).click();
  await expect(page.getByTestId("account-signin-card")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "08-inspector-aggregate/account-logged-out-1280x800.png") });
});

test("inspector — Project Overview On-it-now card", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await openRailList(page); // D-25: a project row needs the list
  await page.getByRole("button", { name: /^ai-juggler/ }).first().click();
  await expect(page.getByTestId("project-on-it-now")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "08-inspector-aggregate/project-overview-1440x900.png") });
});

// AJ-E409 quality-tab captures.
test("inspector — Quality tab populated", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Quality / Performance" }).click();
  await expect(page.getByTestId("quality-tokens")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "09-quality/populated-1440x900.png") });
});

test("inspector — Quality tab View as table", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Quality / Performance" }).click();
  await page.getByRole("button", { name: "View as table" }).click();
  await expect(page.getByTestId("quality-tokens").getByRole("table")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "09-quality/as-table-1440x900.png") });
});

// AJ-E410 processes-tab captures: live tree (3 levels), orphan section
// with the PID-named confirm, the `gone` verification state, and the
// good empty state.
test("inspector — Processes tab live tree + orphans", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Processes" }).click();
  await expect(page.getByTestId("processes-spark")).toBeVisible();
  await expect(page.getByTestId("orphan-row-39871")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "10-processes/live-tree-1440x900.png") });
});

test("inspector — Processes tab Stop confirm popover (PID named)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Processes" }).click();
  await expect(page.getByTestId("orphan-row-39871")).toBeVisible();
  await page.getByRole("button", { name: "Stop process 39871" }).click();
  await expect(page.getByTestId("orphan-confirm-39871")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "10-processes/stop-confirm-1440x900.png") });
});

test("inspector — Processes tab gone verification state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Processes" }).click();
  await expect(page.getByTestId("orphan-row-39871")).toBeVisible();
  await page.getByRole("button", { name: "Stop process 39871" }).click();
  await page.getByRole("button", { name: "Stop 39871" }).click();
  await expect(page.getByTestId("orphan-state-39871")).toHaveText("gone ✓");
  await page.screenshot({ path: resolve(GALLERY, "10-processes/gone-state-1440x900.png") });
});

test("inspector — Processes tab good empty state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Personal/ }).first().click();
  await page.getByRole("tab", { name: "Processes" }).click();
  await expect(page.getByTestId("processes-empty")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "10-processes/empty-1440x900.png") });
});

// AJ-E411 renewal captures: every threshold state on the Account
// Overview, `Expiry: unknown`, the editor + helper text, and the row
// chip on the Accounts screen. Dates are pinned relative to T0 so the
// tiers are deterministic.
function renewalDate(daysFromT0: number): string {
  const target = new Date(T0 + daysFromT0 * 86_400_000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${target.getFullYear()}-${pad(target.getMonth() + 1)}-${pad(target.getDate())}`;
}

async function seedRenewalPage(
  page: import("@playwright/test").Page,
  vp: { width: number; height: number },
  days: number | null,
) {
  await page.addInitScript(
    ({ fixture, t0, days: d }) => {
      const accounts = (fixture.accounts ?? []).map((entry: { id: string }, index: number) => ({
        ...entry,
        renewal_date: index === 0 ? (d === null ? null : new Date(t0 + d * 86_400_000).toISOString().slice(0, 10)) : null,
      }));
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = { ...fixture, accounts };
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0, days },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
}

async function openAccountOverview(page: import("@playwright/test").Page) {
  await goTo(page, "Accounts");
  await page.getByRole("button", { name: "Inspect Claude Code" }).first().click();
  await expect(page.getByTestId("account-overview")).toBeVisible();
}

for (const [name, days] of [
  ["date-only", 20],
  ["countdown", 10],
  ["warning", 6],
  ["danger", 0],
  ["unknown", null],
] as const) {
  test(`renewal — Account Overview ${name} state`, async ({ page }) => {
    await seedRenewalPage(page, { width: 1440, height: 900 }, days);
    await openAccountOverview(page);
    await expect(page.getByTestId("account-renewal")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `11-renewal/account-${name}-1440x900.png`) });
  });
}

test("renewal — editor with helper text (1440x900 + 1280x800)", async ({ page }) => {
  await seedRenewalPage(page, { width: 1440, height: 900 }, 6);
  await openAccountOverview(page);
  await expect(page.getByTestId("renewal-editor")).toBeVisible();
  await expect(page.getByText("Entered by you - AI Juggler does not read billing")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "11-renewal/editor-1440x900.png") });
});

test("renewal — editor at 1280x800", async ({ page }) => {
  await seedRenewalPage(page, { width: 1280, height: 800 }, 6);
  await openAccountOverview(page);
  await expect(page.getByTestId("renewal-editor")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "11-renewal/editor-1280x800.png") });
});

test("renewal — Accounts row chip at <= 7 days (and none for unknown)", async ({ page }) => {
  await seedRenewalPage(page, { width: 1440, height: 900 }, 6);
  await goTo(page, "Accounts");
  await expect(page.getByText("Renews in 6 days").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "11-renewal/accounts-row-chip-1440x900.png") });
});

// AJ-E412 running-model captures: I-03 plain vs drift (sess-8 carries
// the drift fixture: launched opus, observed sonnet-5), the Overview
// dual-model line, and the not-reported inspector state.
test("model — I-03 drift marker on the rail row", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  const marker = page.getByTestId("model-drift").first();
  await expect(marker).toBeVisible();
  await expect(marker).toHaveAttribute("aria-label", "Launched with opus - now running sonnet-5");
  await marker.hover();
  await page.screenshot({ path: resolve(GALLERY, "12-model/rail-drift-1440x900.png") });
});

test("model — I-03 plain (launch == running rows show no marker)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  // The plain rows carry launch-only models; the marker count is exactly
  // the drift rows (one), asserted before the screenshot.
  expect(await page.getByTestId("model-drift").count()).toBe(1);
  await page.screenshot({ path: resolve(GALLERY, "12-model/rail-plain-1440x900.png") });
});

test("model — Instance Overview dual-model line", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await expect(page.getByTestId("instance-model-line")).toBeVisible();
  await expect(page.getByTestId("instance-model-line")).toContainText(
    "Launched with opus - now running sonnet-5",
  );
  await page.screenshot({ path: resolve(GALLERY, "12-model/overview-drift-1440x900.png") });
});

test("model — not reported by Provider inspector state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Personal/ }).first().click();
  await expect(page.getByTestId("instance-model-line")).toContainText(
    "Model: not reported by Claude Code",
  );
  await page.screenshot({ path: resolve(GALLERY, "12-model/not-reported-1440x900.png") });
});

// AJ-E413 desktop captures: the panel surface (no terminal card), the
// rail `app` chip, and the state matrix (running, exited, crashed,
// single-account warning) at the two required viewports.
test("desktop — Instance panel (running app, isolation label, no terminal)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1280, height: 800 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /running/i })
    .click();
  await expect(page.getByTestId("desktop-surface")).toBeVisible();
  await expect(page.getByTestId("desktop-surface")).toContainText("Running");
  await expect(page.getByTestId("desktop-focus")).toBeVisible();
  await expect(page.getByTestId("desktop-isolation")).toContainText("Single-account app");
  await page.screenshot({ path: resolve(GALLERY, "13-desktop/panel-running-1280x800.png") });
});

test("desktop — panel at 1024x680 light", async ({ page }) => {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", "light");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "light" });
  await page.setViewportSize({ width: 1024, height: 680 });
  await page.goto("/");
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /running/i })
    .click();
  await expect(page.getByTestId("desktop-surface")).toBeVisible();
  await expect(page.getByTestId("desktop-isolation")).toContainText("Single-account app");
  await page.screenshot({ path: resolve(GALLERY, "13-desktop/panel-running-1024x680-light.png") });
});

test("desktop — exited app state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1280, height: 800 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /Done/ })
    .click();
  await expect(page.getByTestId("desktop-surface")).toBeVisible();
  await expect(page.getByTestId("desktop-surface")).toContainText("app exited");
  await page.screenshot({ path: resolve(GALLERY, "13-desktop/panel-exited-1280x800.png") });
});

test("desktop — crashed app state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1280, height: 800 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /failed|crashed/i })
    .click();
  await expect(page.getByTestId("desktop-surface")).toBeVisible();
  await expect(page.getByTestId("desktop-surface")).toContainText("app crashed");
  await page.screenshot({ path: resolve(GALLERY, "13-desktop/panel-crashed-1280x800.png") });
});

test("desktop — single-account warning state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1280, height: 800 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /running/i })
    .click();
  const warning = page.getByTestId("desktop-isolation");
  await expect(warning).toBeVisible();
  await expect(warning).toContainText(
    "Single-account app — Juggler tracks, cannot isolate",
  );
  await page.screenshot({ path: resolve(GALLERY, "13-desktop/panel-single-account-warning-1280x800.png") });
});

test("desktop — rail app chip", async ({ page }) => {
  await seedInspectorPage(page, { width: 1280, height: 800 });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await expect(page.getByTestId("rail-app-chip").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "13-desktop/rail-app-chip-1280x800.png") });
});

// AJ-E414 focus captures: the outcome text after a focus attempt, the
// permission explainer (degraded tier), and the "Go to instance…"
// palette at both themes. The harness seam `__AIJ_FOCUS_OUTCOME__`
// pins the mocked tier result each capture renders.
async function seedFocusPage(
  page: import("@playwright/test").Page,
  outcome: { outcome: string; permission_missing: boolean; window_title: string | null },
) {
  await page.addInitScript(
    ({ fixture, t0, pinned }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
      (window as never as { __AIJ_FOCUS_OUTCOME__: unknown }).__AIJ_FOCUS_OUTCOME__ = pinned;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    {
      fixture: allStatuses,
      t0: T0,
      pinned: outcome,
    },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

test("focus — panel outcome after focus (Focused window, dark)", async ({ page }) => {
  await seedFocusPage(page, {
    outcome: "Focused window",
    permission_missing: false,
    window_title: "aij-e414-a.txt",
  });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /running/i })
    .click();
  await page.getByTestId("desktop-focus").click();
  await expect(page.getByTestId("desktop-focus-outcome")).toContainText("Focused window");
  await expect(page.getByTestId("desktop-focus-outcome")).toContainText("aij-e414-a.txt");
  await expect(page.getByTestId("desktop-a11y-explainer")).toHaveCount(0);
  await page.screenshot({ path: resolve(GALLERY, "14-focus/panel-outcome-1280x800.png") });
});

test("focus — permission explainer on the degraded tier", async ({ page }) => {
  await seedFocusPage(page, {
    outcome: "App opened — window not found",
    permission_missing: true,
    window_title: null,
  });
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /running/i })
    .click();
  await page.getByTestId("desktop-focus").click();
  const explainer = page.getByTestId("desktop-a11y-explainer");
  await expect(explainer).toBeVisible();
  await expect(explainer).toContainText(
    "Juggler can open the app; exact-window focus needs Accessibility permission",
  );
  await page.screenshot({ path: resolve(GALLERY, "14-focus/permission-explainer-1280x800.png") });
});

test("focus — Go to instance… palette (dark 1280x800)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1280, height: 800 });
  // The ⌘K handler mounts with the shell — wait for it (AJ-E806).
  await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
  await page.keyboard.press("Meta+k");
  const palette = page.getByTestId("goto-palette");
  await expect(palette).toBeVisible();
  // Live instances only, desktop first; the footer names the binding.
  await expect(palette.locator('[data-testid="goto-row-sess-desktop-1"]')).toBeVisible();
  await expect(palette.locator('[data-testid="goto-row-sess-2"]')).toBeVisible();
  await expect(palette.locator('[data-testid="goto-row-sess-desktop-3"]')).toHaveCount(0);
  await expect(palette.locator('[data-testid="goto-row-sess-6"]')).toHaveCount(0);
  await expect(palette).toContainText("Alt+Space");
  await page.screenshot({ path: resolve(GALLERY, "14-focus/palette-1280x800.png") });
});

test("focus — Go to instance… palette (light 1024x680)", async ({ page }) => {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", "light");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "light" });
  await page.setViewportSize({ width: 1024, height: 680 });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
  await page.keyboard.press("Meta+k");
  const palette = page.getByTestId("goto-palette");
  await expect(palette).toBeVisible();
  await expect(palette.locator('[data-testid="goto-row-sess-desktop-1"]')).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "14-focus/palette-1024x680-light.png") });
});

// AJ-E415 preview captures: the live mirror (indicator + frame age +
// click-to-focus affordance), the paused (minimised) state, and the
// permission-denied state — both themes. The harness seam
// `__AIJ_PREVIEW_PHASE__` pins the phase each capture renders.
async function seedPreviewPage(
  page: import("@playwright/test").Page,
  vp: { width: number; height: number },
  theme: "dark" | "light",
  phase: { phase: string; text: string } | null,
) {
  await page.addInitScript(
    ({ fixture, t0, themeName, seededPhase }) => {
      (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
      if (seededPhase !== null) {
        (window as never as { __AIJ_PREVIEW_PHASE__: unknown }).__AIJ_PREVIEW_PHASE__ = {
          session_id: "sess-desktop-1",
          ...seededPhase,
        };
      }
      localStorage.setItem("aij.theme", themeName);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0, themeName: theme, seededPhase: phase },
  );
  await page.emulateMedia({
    reducedMotion: "reduce",
    colorScheme: theme === "light" ? "light" : "dark",
  });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
}

async function openRunningDesktopPanel(page: import("@playwright/test").Page) {
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /running/i })
    .click();
}

test("preview — live mirror with indicator, frame age, click-to-focus (dark)", async ({ page }) => {
  await seedPreviewPage(page, { width: 1280, height: 800 }, "dark", null);
  await openRunningDesktopPanel(page);
  const preview = page.getByTestId("desktop-preview");
  await expect(preview).toBeVisible();
  await expect(preview.locator('[data-testid="preview-live-indicator"]')).toContainText("Live preview");
  await expect(preview.locator('[data-testid="preview-frame-age"]')).toContainText("frame age");
  const frame = preview.locator('[data-testid="preview-frame"]');
  await expect(frame).toBeVisible();
  await expect(preview).toContainText("click to focus the real window");
  await page.screenshot({ path: resolve(GALLERY, "15-preview/panel-live-1280x800.png") });
});

test("preview — live mirror at 1024x680 light", async ({ page }) => {
  await seedPreviewPage(page, { width: 1024, height: 680 }, "light", null);
  await openRunningDesktopPanel(page);
  const preview = page.getByTestId("desktop-preview");
  await expect(preview.locator('[data-testid="preview-frame"]')).toBeVisible();
  await expect(preview.locator('[data-testid="preview-live-indicator"]')).toContainText("Live preview");
  await page.screenshot({ path: resolve(GALLERY, "15-preview/panel-live-1024x680-light.png") });
});

test("preview — paused (window minimised) keeps the labelled last frame", async ({ page }) => {
  await seedPreviewPage(page, { width: 1280, height: 800 }, "dark", {
    phase: "paused",
    text: "Preview paused — window not visible",
  });
  await openRunningDesktopPanel(page);
  const preview = page.getByTestId("desktop-preview");
  await expect(preview.locator('[data-testid="preview-paused"]')).toHaveText(
    "Preview paused — window not visible",
  );
  await expect(preview.locator('[data-testid="preview-frame"]')).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "15-preview/panel-paused-1280x800.png") });
});

test("preview — permission denied renders guidance, no frame", async ({ page }) => {
  await seedPreviewPage(page, { width: 1280, height: 800 }, "dark", {
    phase: "denied",
    text: "Screen Recording permission needed — grant it in System Settings, then re-enable preview",
  });
  await openRunningDesktopPanel(page);
  const preview = page.getByTestId("desktop-preview");
  await expect(preview.locator('[data-testid="preview-denied"]')).toContainText(
    "Screen Recording permission needed",
  );
  await expect(preview.locator('[data-testid="preview-frame"]')).toHaveCount(0);
  await page.screenshot({ path: resolve(GALLERY, "15-preview/panel-denied-1280x800.png") });
});

test("preview — exited instance renders no preview (status only)", async ({ page }) => {
  await seedPreviewPage(page, { width: 1280, height: 800 }, "dark", null);
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /Done/ })
    .click();
  await expect(page.getByTestId("desktop-surface")).toContainText("app exited");
  await expect(page.getByTestId("desktop-preview")).toHaveCount(0);
  await page.screenshot({ path: resolve(GALLERY, "15-preview/panel-exited-1280x800.png") });
});

// AJ-E501: the ledger-degraded banner — copy exact, both themes, on Home
// (it moved there from Overview, AJ-E1021).
for (const theme of ["dark", "light"] as const) {
  test(`ledger — degraded banner (${theme} 1440x900)`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, themeName }) => {
        const seeded = { ...fixture };
        seeded.stats = {
          accounts: 3,
          profiles: 3,
          active_sessions: 3,
          ledger_degraded: true,
        };
        (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = seeded;
        localStorage.setItem("aij.theme", themeName);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, themeName: theme },
    );
    await page.emulateMedia({
      reducedMotion: "reduce",
      colorScheme: theme === "light" ? "light" : "dark",
    });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    const banner = page.getByTestId("ledger-degraded-banner");
    await expect(banner).toBeVisible();
    await expect(banner).toContainText(
      "Process tracking degraded — cleanup is read-only until restart.",
    );
    await page.screenshot({
      path: resolve(GALLERY, `17-ledger/degraded-banner-1440x900-${theme}.png`),
    });
  });
}

// Determinism self-test (Req.4): same fixture twice → byte-identical PNG.
test("determinism — two captures are byte-identical", async ({ page }) => {  const capture = async (suffix: string) => {
    await page.addInitScript(
      ({ fixture, t0 }) => {
        (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", "dark");
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0 },
    );
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expectRail(page); // D-25: the list, or its icon column when narrow
    const path = resolve(GALLERY, `01-rail/determinism-${suffix}.png`);
    await page.screenshot({ path });
    return path;
  };
  const a = await capture("a");
  const b = await capture("b");
  const { readFileSync, statSync, unlinkSync } = await import("node:fs");
  const sizeA = statSync(a).size;
  const sizeB = statSync(b).size;
  // Byte comparison (pinned clock + reduced motion + fixed viewport).
  const identical = readFileSync(a).equals(readFileSync(b));
  if (identical) {
    unlinkSync(b); // keep one artifact
  } else {
    // PNG streams may differ in chunk ordering; report honestly rather
    // than fake a pass — the task requires 0 px diff, so fail loudly.
    throw new Error(
      `determinism check failed: captures differ (sizes ${sizeA} vs ${sizeB}); see determinism-a/b.png`,
    );
  }
  if (existsSync(resolve(GALLERY, "determinism-b.png"))) {
    // already unlinked on success; no-op
  }
});


// AJ-E505 port-map captures: the inventory with all four
// classifications (unmanaged muted, icon+text — never colour alone)
// and the pre-launch collision sheet (copy-exact), both themes, two
// sizes.
const PORTS_FIXTURE = {
  rows: [
    {
      port: 3000, pid: 4400, class: "managed-running", session: "s-2", provider: "claude-code",
      account: "Team Max", project: "api", listener_age_ms: 65_000, name: "node", hint: "common dev server port", killable: true,
    },
    {
      port: 5173, pid: 8123, class: "managed-orphan", session: "s-1", provider: "claude-code",
      account: "Work Max", project: "web", listener_age_ms: 9_000_000, name: "python3", hint: "vite dev server port", killable: true,
    },
    {
      port: 8080, pid: 5500, class: "re-adopted", session: "s-3", provider: "claude-code",
      account: null, project: null, listener_age_ms: 250_000, name: "python3", hint: "common dev server port", killable: true,
    },
    {
      port: 8000, pid: 6600, class: "not-managed", session: null, provider: null,
      account: null, project: null, listener_age_ms: null, name: null, hint: "common dev server port", killable: false,
    },
  ],
  note: null,
  hogs: [],
};

for (const theme of THEMES) {
  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x700", width: 1024, height: 700 },
  ]) {
    test(`ports — inventory, four classifications — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, ports, theme }) => {
          (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
          (window as never as { __AIJ_PORTS__: unknown }).__AIJ_PORTS__ = ports;
          localStorage.setItem("aij.theme", theme);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t0: T0, ports: PORTS_FIXTURE, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await page.getByRole("tab", { name: "Processes" }).click();
      await expect(page.getByTestId("port-inventory")).toBeVisible();
      await expect(page.getByTestId("port-row-5173")).toContainText("Managed orphan");
      await expect(page.getByTestId("port-row-8000")).toContainText("Not managed by Juggler");
      await page.screenshot({ path: resolve(GALLERY, `18-ports/inventory-${vp.name}-${theme}.png`) });
    });
  }
}

for (const theme of THEMES) {
  test(`ports — pre-launch collision sheet — 1440x900 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, warning, theme }) => {
        (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
        (window as never as { __AIJ_PORT_WARNING__: unknown }).__AIJ_PORT_WARNING__ = warning;
        localStorage.setItem("aij.theme", theme);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        warning: {
          port: 5173,
          pid: 8123,
          instance: "claude-code (s-1a2b3c4d)",
          age_ms: 9_000_000,
          message: ":5173 is already held by an orphan from claude-code (s-1a2b3c4d) (2h).",
        },
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    // Trigger a launch through the Instance Actions' Resume row —
    // the pre-check returns the fixture warning and the sheet renders
    // instead of spawning (launch is never blocked; the sheet offers
    // Launch anyway).
    await expectRail(page); // D-25: the list, or its icon column when narrow
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await page.getByRole("tab", { name: "Actions" }).click();
    await page.getByRole("button", { name: /Resume|Relaunch/ }).first().click();
    await expect(page.getByTestId("port-warning-sheet")).toBeVisible();
    await expect(page.getByTestId("port-warning-copy")).toContainText(
      ":5173 is already held by an orphan",
    );
    await page.screenshot({ path: resolve(GALLERY, `18-ports/warning-sheet-1440x900-${theme}.png`) });
  });
}


// AJ-E506 Cleanup Centre captures: six states (needs-attention with
// orphan+hog+collision, all-managed, re-adopted, unverifiable,
// empty-success, ledger-degraded) × themes × sizes.

for (const state of ["needs-attention", "all-managed", "re-adopted", "unverifiable", "empty", "degraded"]) {
  for (const theme of THEMES) {
    for (const vp of [
      { name: "1440x900", width: 1440, height: 900 },
      { name: "1024x700", width: 1024, height: 700 },
    ]) {
      test(`cleanup — ${state} — ${vp.name} — ${theme}`, async ({ page }) => {
        await page.addInitScript(
          ({ fixture, t0, cleanup, theme }) => {
            (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
            (window as never as { __AIJ_CLEANUP__: unknown }).__AIJ_CLEANUP__ = cleanup;
            localStorage.setItem("aij.theme", theme);
            localStorage.setItem("aij.onboarding.dismissed", "1");
            sessionStorage.setItem("aij.ready.skipped", "1");
            Date.now = () => t0;
          },
          { fixture: allStatuses, t0: T0, cleanup: cleanupFixture(state), theme },
        );
        await page.emulateMedia({ reducedMotion: "reduce" });
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.goto("/");
        // Reached from the TOP-RIGHT global menus (ADR-0007) — never
        // a rail peer.
        await goTo(page, "Cleanup");
        await expect(page.getByTestId("cleanup-centre")).toBeVisible();
        if (state === "needs-attention") {
          await expect(page.getByTestId("cleanup-row-8123")).toContainText(
            "Still running after its session closed",
          );
          await expect(page.getByTestId("cleanup-row-8124")).toContainText("hog");
        }
        if (state === "empty") {
          await expect(page.getByTestId("cleanup-empty")).toContainText(
            "Nothing left behind — 3 sessions running clean.",
          );
        }
        if (state === "degraded") {
          await expect(page.getByTestId("cleanup-degraded-banner")).toBeVisible();
        }
        if (state === "unverifiable") {
          await expect(page.getByTestId("cleanup-state-8124")).toContainText(
            "Unverifiable — will not be touched",
          );
        }
        await page.screenshot({ path: resolve(GALLERY, `19-cleanup/${state}-${vp.name}-${theme}.png`) });
      });
    }
  }
}


// AJ-E509 history captures: audit rows (manual + automatic actors,
// outcome vocabulary incl. the crash-pending unknown) and the
// empty-history copy — via the Cleanup Centre's History filter.
for (const theme of THEMES) {
  test(`cleanup — history populated — 1440x900 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, cleanup, theme }) => {
        (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
        (window as never as { __AIJ_CLEANUP__: unknown }).__AIJ_CLEANUP__ = cleanup;
        localStorage.setItem("aij.theme", theme);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        cleanup: {
          groups: [],
          clean_instances: 0,
          ledger_degraded: false,
          history: {
            rows: [
              {
                id: 1, at_ms: T0 - 90_000, actor: "user:orphan_stop", automatic: false,
                pid: 8123, cmdline_trunc: "python3 -m http.server 5173", session_id: "s-1",
                provider: "claude-code", account_id: "Work Max", machine_id: "machine-local",
                outcome: "exited on TERM", ports_before: "5173", ports_after: "",
              },
              {
                id: 2, at_ms: T0 - 200_000, actor: "auto:tier-1", automatic: true,
                pid: 4400, cmdline_trunc: "node vite", session_id: "s-2",
                provider: "claude-code", account_id: null, machine_id: "machine-local",
                outcome: "TERM'd, outcome unknown", ports_before: null, ports_after: null,
              },
            ],
            empty_note: null,
          },
        },
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await goTo(page, "Cleanup");
    await page.getByRole("radio", { name: "History (7 days)" }).click();
    await expect(page.getByTestId("cleanup-history")).toBeVisible();
    await expect(page.getByTestId("cleanup-history-actor-2")).toContainText("automatic");
    await page.screenshot({ path: resolve(GALLERY, `19-cleanup/history-populated-1440x900-${theme}.png`) });
  });

  test(`cleanup — history empty — 1440x900 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, cleanup, theme }) => {
        (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
        (window as never as { __AIJ_CLEANUP__: unknown }).__AIJ_CLEANUP__ = cleanup;
        localStorage.setItem("aij.theme", theme);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        cleanup: {
          groups: [],
          clean_instances: 0,
          ledger_degraded: false,
          history: {
            rows: [],
            empty_note: "No cleanups yet — automatic cleanup is off by default.",
          },
        },
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await goTo(page, "Cleanup");
    await page.getByRole("radio", { name: "History (7 days)" }).click();
    await expect(page.getByTestId("cleanup-history-empty")).toContainText(
      "No cleanups yet — automatic cleanup is off by default.",
    );
    await page.screenshot({ path: resolve(GALLERY, `19-cleanup/history-empty-1440x900-${theme}.png`) });
  });
}


// ---------------------------------------------------------------------------
// AJ-E507 — the verify-before-kill sheet: the three steps and every
// state. All states ride the SAME needs-attention fixture (verified
// orphan row holding :5173); the sheet is opened the real way — the
// row's Terminate… affordance — never mounted out of context.
// ---------------------------------------------------------------------------

const KILL_VERIFY_OK = {
  pid: 8123,
  session: "s-1",
  verified: true,
  refusal: null,
  exe_path: "/usr/local/bin/node",
  cmdline: "node /srv/web/node_modules/.bin/vite dev",
  cwd: "/srv/web",
  ports: [5173],
  pgid: 8100,
  escaped_group: false,
  members: [
    {
      pid: 8100,
      pgid: 8100,
      exe: "sh",
      cmdline: "sh -c vite dev",
      role_guess: "TerminalShell",
      unsaved_state_warning: "Includes a TerminalShell that may hold unsaved state.",
      from_ledger: true,
    },
    {
      pid: 8123,
      pgid: 8100,
      exe: "node",
      cmdline: "node vite dev",
      role_guess: "DevServer",
      unsaved_state_warning: null,
      from_ledger: true,
    },
  ],
  tree_pids: [8100, 8123],
  platform: "TERM → grace → KILL",
  grace_secs: 5,
  provider: "claude-code",
  account_id: "Work Max",
  project_id: "web",
};

const KILL_VERIFY_FAILED = {
  ...KILL_VERIFY_OK,
  verified: false,
  refusal: "This PID now belongs to a different program — Juggler will not touch it.",
  members: [],
};

const KILL_EXECUTE_REPORT = {
  refusal: null,
  members: [
    { pid: 8100, outcome: "already gone" },
    { pid: 8123, outcome: "needed KILL" },
  ],
  cancelled: false,
  all_succeeded: true,
  still_holding: [{ port: 5173, pid: 9100, exe: "node" }],
  reoffer_pid: 9100,
};

const KILL_EXECUTE_FREED = {
  refusal: null,
  members: [
    { pid: 8123, outcome: "exited on TERM" },
  ],
  cancelled: false,
  all_succeeded: true,
  still_holding: [],
  reoffer_pid: null,
};

const killSeams = (state: string) => ({
  verify:
    state === "verification-failed"
      ? KILL_VERIFY_FAILED
      : state === "windows-wording"
        ? { ...KILL_VERIFY_OK, platform: "TerminateProcess only", grace_secs: 0 }
        : KILL_VERIFY_OK,
  pending: state === "grace-countdown",
  execute: state === "outcome-report" ? KILL_EXECUTE_REPORT : KILL_EXECUTE_FREED,
});

for (const state of [
  "verify-pid",
  "verify-pgid",
  "verify-tree",
  "verification-failed",
  "grace-countdown",
  "outcome-report",
  "outcome-freed",
  "windows-wording",
] as const) {
  for (const theme of THEMES) {
    for (const vp of [
      { name: "1440x900", width: 1440, height: 900 },
      { name: "1024x700", width: 1024, height: 700 },
    ]) {
      test(`kill-sheet — ${state} — ${vp.name} — ${theme}`, async ({ page }) => {
        const seams = killSeams(state);
        await page.addInitScript(
          ({ fixture, t0, cleanup, theme, seams }) => {
            (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
            (window as never as { __AIJ_CLEANUP__: unknown }).__AIJ_CLEANUP__ = cleanup;
            const w = window as never as Record<string, unknown>;
            w.__AIJ_KILL_VERIFY__ = () => seams.verify;
            // The countdown state leaves execute UNSET so the pending
            // seam applies (a ladder that never resolves); every
            // other state resolves with the report fixture.
            if (!seams.pending) {
              w.__AIJ_KILL_EXECUTE__ = () => seams.execute;
            }
            w.__AIJ_KILL_EXECUTE_PENDING__ = seams.pending;
            // During the countdown the members are still alive (the
            // per-process ticks show "running" → "exited" honestly).
            w.__AIJ_KILL_LIVENESS__ = (pids: number[]) => pids.map(() => true);
            localStorage.setItem("aij.theme", theme);
            localStorage.setItem("aij.onboarding.dismissed", "1");
            sessionStorage.setItem("aij.ready.skipped", "1");
            Date.now = () => t0;
          },
          { fixture: allStatuses, t0: T0, cleanup: cleanupFixture("needs-attention"), theme, seams },
        );
        await page.emulateMedia({ reducedMotion: "reduce" });
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.goto("/");
        await goTo(page, "Cleanup");
        await expect(page.getByTestId("cleanup-centre")).toBeVisible();
        // The REAL entry: the verified orphan row's affordance.
        await page.getByTestId("cleanup-kill-8123").click();
        await expect(page.getByTestId("kill-sheet-8123")).toBeVisible();

        if (state === "verification-failed") {
          await expect(page.getByTestId("kill-refused-8123")).toContainText(
            "This PID now belongs to a different program",
          );
          // No kill button in the failed-state DOM (Req.2 gate).
          const confirm = await page.getByTestId("kill-confirm-8123").count();
          if (confirm !== 0) throw new Error("kill button present in failed-state DOM");
        } else if (state === "verify-pgid" || state === "verify-tree") {
          await expect(page.getByTestId("kill-ready-8123")).toBeVisible();
          await page
            .getByTestId(`kill-scope-${state === "verify-pgid" ? "pgid" : "tree"}-8123`)
            .check();
          await expect(page.getByTestId("kill-member-8123")).toBeVisible();
        } else if (state === "grace-countdown") {
          await expect(page.getByTestId("kill-ready-8123")).toBeVisible();
          await page.getByTestId("kill-confirm-8123").click();
          // The spec's "grace countdown at 3 s" moment.
          await expect(page.getByTestId("kill-countdown-8123")).toContainText("3s", {
            timeout: 8_000,
          });
          await expect(page.getByTestId("kill-tick-8123")).toContainText("running");
        } else if (state === "outcome-report" || state === "outcome-freed") {
          await expect(page.getByTestId("kill-ready-8123")).toBeVisible();
          await page.getByTestId("kill-confirm-8123").click();
          await expect(page.getByTestId("kill-report-8123")).toBeVisible();
          if (state === "outcome-report") {
            await expect(page.getByTestId("kill-still-holding-8123")).toContainText(":5173");
          } else {
            await expect(page.getByTestId("kill-success-8123")).toContainText(":5173 is free.");
          }
        } else if (state === "windows-wording") {
          await expect(page.getByTestId("kill-platform-8123")).toContainText(
            "Windows: TerminateProcess only",
          );
        } else {
          await expect(page.getByTestId("kill-ready-8123")).toBeVisible();
          await expect(page.getByTestId("kill-confirm-8123")).toContainText(
            "Terminate vite (PID 8123) and free :5173",
          );
        }
        await page.screenshot({
          path: resolve(GALLERY, `20-kill/${state}-${vp.name}-${theme}.png`),
        });
      });
    }
  }
}


// ---------------------------------------------------------------------------
// AJ-E508 — the tier surfaces: the per-Account toggles (off/on) in the
// Account launch-settings area, the Cleanup Centre's paused banner,
// the suspended-after-crash banner, and the Tier-2 notification with
// its Undo window (at appearance and at 5 s remaining).
// ---------------------------------------------------------------------------

async function seedTiersPage(
  page: import("@playwright/test").Page,
  vp: { width: number; height: number },
  theme: string,
  seams: Record<string, unknown>,
) {
  await page.addInitScript(
    ({ fixture, t0, theme: t, seams: s }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      for (const [key, value] of Object.entries(s)) {
        w[key] = value;
      }
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0, theme, seams },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
}

for (const theme of THEMES) {
  for (const vp of [{ name: "1440x900", width: 1440, height: 900 }]) {
    test(`tiers — account toggles off — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedTiersPage(page, vp, theme, {
        __AIJ_CLEANUP_TIERS__: { tier1_opt_in: false, tier2_opt_in: false },
      });
      await openAccountOverview(page);
      await expect(page.getByTestId("account-tier-toggles")).toBeVisible();
      await expect(page.getByTestId("account-cleanup-toggle")).toContainText(
        "Automatically tidy left-behind dev servers",
      );
      await expect(page.getByTestId("account-tier2-toggle")).toContainText(
        "Also tidy after a crash (with notification)",
      );
      await page.screenshot({
        path: resolve(GALLERY, `21-tiers/toggles-off-${vp.name}-${theme}.png`),
      });
    });

    test(`tiers — account toggles on — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedTiersPage(page, vp, theme, {
        __AIJ_CLEANUP_TIERS__: { tier1_opt_in: true, tier2_opt_in: true },
      });
      await openAccountOverview(page);
      await expect(page.getByTestId("account-tier-toggles")).toBeVisible();
      await expect(
        page.getByTestId("account-cleanup-toggle").locator("input"),
      ).toBeChecked();
      await expect(
        page.getByTestId("account-tier2-toggle").locator("input"),
      ).toBeChecked();
      await page.screenshot({
        path: resolve(GALLERY, `21-tiers/toggles-on-${vp.name}-${theme}.png`),
      });
    });

    test(`tiers — centre paused banner — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedTiersPage(page, vp, theme, {
        __AIJ_CLEANUP_STATUS__: {
          paused: true,
          suspended: false,
          suspended_until_ms: null,
          tier2_pending: [],
        },
      });
      await goTo(page, "Cleanup");
      await expect(page.getByTestId("cleanup-paused-banner")).toContainText(
        "Automatic cleanup is paused.",
      );
      await expect(page.getByTestId("cleanup-pause-toggle")).toContainText("Resume");
      await page.screenshot({
        path: resolve(GALLERY, `21-tiers/paused-${vp.name}-${theme}.png`),
      });
    });

    test(`tiers — suspended after crash — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedTiersPage(page, vp, theme, {
        __AIJ_CLEANUP_STATUS__: {
          paused: false,
          suspended: true,
          suspended_until_ms: T0 + 24 * 3_600_000,
          tier2_pending: [],
        },
      });
      await goTo(page, "Cleanup");
      await expect(page.getByTestId("cleanup-suspended-banner")).toContainText(
        "suspended for 24 h",
      );
      await page.screenshot({
        path: resolve(GALLERY, `21-tiers/suspended-${vp.name}-${theme}.png`),
      });
    });

    test(`tiers — tier-2 notification at appearance — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedTiersPage(page, vp, theme, {
        __AIJ_CLEANUP_STATUS__: {
          paused: false,
          suspended: false,
          suspended_until_ms: null,
          tier2_pending: [
            {
              pid: 8123,
              session: "s-1",
              cmdline_trunc: "node vite dev",
              deadline_ms: T0 + 30_000,
            },
          ],
        },
      });
      await goTo(page, "Cleanup");
      await expect(page.getByTestId("cleanup-tier2-notice-8123")).toContainText(
        "survived a crash",
      );
      await expect(page.getByTestId("cleanup-tier2-notice-8123")).toContainText("in 30s");
      await page.screenshot({
        path: resolve(GALLERY, `21-tiers/tier2-appearance-${vp.name}-${theme}.png`),
      });
    });

    test(`tiers — tier-2 notification at 5 s remaining — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedTiersPage(page, vp, theme, {
        __AIJ_CLEANUP_STATUS__: {
          paused: false,
          suspended: false,
          suspended_until_ms: null,
          tier2_pending: [
            {
              pid: 8123,
              session: "s-1",
              cmdline_trunc: "node vite dev",
              deadline_ms: T0 + 5_000,
            },
          ],
        },
      });
      await goTo(page, "Cleanup");
      await expect(page.getByTestId("cleanup-tier2-notice-8123")).toContainText("in 5s");
      await expect(page.getByTestId("cleanup-tier2-undo-8123")).toContainText("Undo");
      await page.screenshot({
        path: resolve(GALLERY, `21-tiers/tier2-5s-${vp.name}-${theme}.png`),
      });
    });
  }
}


// ---------------------------------------------------------------------------
// AJ-E510 — crash-recovery captures: the ONE-time restart summary (the
// exact 2/1/1 drill text rides the __AIJ_RECOVERY__ seam), the
// re-adopted Instance panel (the ↻ chip + the honest detached-terminal
// notice on the Processes tab), and the Cleanup Centre's Re-adopted
// filter with the distinct left-behind label.
// ---------------------------------------------------------------------------

const RECOVERY_SUMMARY = {
  summary:
    "2 sessions re-adopted, 1 helpers left behind, 1 processes could not be verified and were left alone.",
  adopted: 2,
  left_behind: 1,
  unverifiable: 1,
};

const RECOVERY_CLEANUP = {
  groups: [
    {
      session: "s-crash",
      provider: "claude-code",
      account: "Work Max",
      project: "web",
      machine: "machine-local",
      healthy: false,
      rows: [
        {
          pid: 8402, pgid: 8400, role: "root", state: "adopted",
          state_label: "Running — re-adopted", down_period: false,
          role_guess: "sh (guess)", exe_path: "/bin/sh", cmdline: "sh -c claude",
          age_ms: 45 * 60_000, rss_bytes: 12_000_000, cpu_pct: 0.2, ports: null,
          hog: false, hog_rules: [],
          evidence: {
            root_pid: 8402, parent_pid: null, pgid: 8400, escaped_group: false,
            verification: { verdict: "verified", detail: "start-time matches the live process" },
            first_seen: T0 - 45 * 60_000, last_seen: T0,
          },
        },
        {
          pid: 8301, pgid: 8300, role: "helper", state: "orphaned",
          state_label: "left behind while Juggler was closed", down_period: true,
          role_guess: "python3 (guess)", exe_path: "/usr/bin/python3",
          cmdline: "python3 -m http.server 5173",
          age_ms: 9_000_000, rss_bytes: 48 * 1024 * 1024, cpu_pct: 1.4, ports: "5173",
          hog: false, hog_rules: [],
          evidence: {
            root_pid: 8300, parent_pid: 8300, pgid: 8300, escaped_group: false,
            verification: { verdict: "verified", detail: "start-time and exe match the live process" },
            first_seen: T0 - 9_000_000, last_seen: T0,
          },
        },
      ],
    },
  ],
  clean_instances: 1,
  ledger_degraded: false,
};

async function seedRecoveryPage(
  page: import("@playwright/test").Page,
  theme: string,
  seams: Record<string, unknown>,
) {
  await page.addInitScript(
    ({ fixture, t0, theme: t, seams: s }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      for (const [key, value] of Object.entries(s)) {
        w[key] = value;
      }
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0, theme, seams },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
}

for (const theme of THEMES) {
  test(`recovery — one-time summary dialog — 1440x900 — ${theme}`, async ({ page }) => {
    await seedRecoveryPage(page, theme, { __AIJ_RECOVERY__: RECOVERY_SUMMARY });
    const dialog = page.getByTestId("recovery-summary");
    await expect(dialog).toBeVisible();
    // The exact drill text (one source of truth with the Rust test).
    await expect(page.getByTestId("recovery-summary-text")).toHaveText(RECOVERY_SUMMARY.summary);
    await expect(dialog).toContainText("stays suspended for 24 hours");
    await page.screenshot({
      path: resolve(GALLERY, `22-recovery/summary-dialog-1440x900-${theme}.png`),
    });
  });

  test(`recovery — adopted panel: chip + detached notice — 1440x900 — ${theme}`, async ({ page }) => {
    // sess-2 (the running Work Max terminal) survived the crash and was
    // re-adopted at restart.
    await page.addInitScript(
      ({ fixture, t0, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = {
          ...fixture,
          sessions: fixture.sessions.map((session: { id: string }) =>
            session.id === "sess-2" ? { ...session, adopted: true } : session,
          ),
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await expectRail(page); // D-25: the list, or its icon column when narrow
    await page
      .getByRole("button", { name: /Work Max/ })
      .filter({ hasText: /running/i })
      .first()
      .click();
    await expect(page.getByTestId("instance-adopted-chip")).toContainText("↻ re-adopted");
    await page.getByRole("tab", { name: "Processes" }).click();
    const notice = page.getByTestId("instance-detached-notice");
    await expect(notice).toBeVisible();
    await expect(notice).toContainText("Running — re-adopted after restart");
    await expect(notice).toContainText("terminal is detached");
    await page.screenshot({
      path: resolve(GALLERY, `22-recovery/adopted-panel-1440x900-${theme}.png`),
    });
  });

  test(`recovery — cleanup Re-adopted filter + left-behind label — 1440x900 — ${theme}`, async ({ page }) => {
    await seedRecoveryPage(page, theme, { __AIJ_CLEANUP__: RECOVERY_CLEANUP });
    await goTo(page, "Cleanup");
    await expect(page.getByTestId("cleanup-centre")).toBeVisible();
    await page.getByRole("radio", { name: "Re-adopted" }).click();
    await expect(page.getByTestId("cleanup-state-8402")).toContainText("Running — re-adopted");
    await expect(page.getByTestId("cleanup-state-8301")).toContainText(
      "left behind while Juggler was closed",
    );
    await page.screenshot({
      path: resolve(GALLERY, `22-recovery/cleanup-readopted-1440x900-${theme}.png`),
    });
  });
}


// ---------------------------------------------------------------------------
// AJ-E512 — the passive alert surfaces: the hog alert (exact copy +
// staged sheet pre-fill) and the idle-orphan nudge, plus the recorded
// keyboard-only pass (open Centre → review hog → stage sheet → Esc).
// ---------------------------------------------------------------------------

const HOG_ALERT = {
  kind: "hog",
  text: "node (vite) under Work Max holds 3.1 GiB — review it in Cleanup.",
  pid: 8123,
  session: "s-1",
};
const IDLE_ALERT = {
  kind: "idle",
  text: "python3 has been idle for 2h — review it in Cleanup.",
  pid: 8124,
  session: "s-1",
};

for (const theme of THEMES) {
  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x700", width: 1024, height: 700 },
  ]) {
    test(`alerts — hog alert + staged sheet — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedRecoveryPage(page, theme, {
        __AIJ_CLEANUP__: cleanupFixture("needs-attention"),
        __AIJ_CLEANUP_ALERTS__: { alerts: [HOG_ALERT] },
        __AIJ_KILL_VERIFY__: () => ({
          pid: 8123,
          session: "s-1",
          verified: true,
          refusal: null,
          exe_path: "/usr/local/bin/node",
          cmdline: "node vite dev",
          cwd: "/srv/web",
          ports: [5173],
          pgid: 8100,
          escaped_group: false,
          members: [
            {
              pid: 8123, pgid: 8100, exe: "node", cmdline: "node vite dev",
              role_guess: "DevServer", unsaved_state_warning: null, from_ledger: true,
            },
          ],
          tree_pids: [8100, 8123],
          platform: "TERM → grace → KILL",
          grace_secs: 5,
          provider: "claude-code",
          account_id: "Work Max",
          project_id: "web",
        }),
        __AIJ_KILL_EXECUTE__: () => ({
          refusal: null,
          members: [{ pid: 8123, outcome: "exited on TERM" }],
          cancelled: false,
          all_succeeded: true,
          still_holding: [],
          reoffer_pid: null,
        }),
      });
      await goTo(page, "Cleanup");
      await expect(page.getByTestId("cleanup-alerts")).toBeVisible();
      await expect(page.getByTestId("hog-alert-8123")).toContainText(
        "node (vite) under Work Max holds 3.1 GiB — review it in Cleanup.",
      );
      // A status message, not a dialog.
      await expect(
        page.getByRole("status", { name: "Passive cleanup alerts" }),
      ).toHaveAttribute("aria-live", "polite");
      await page.getByTestId("hog-stage-8123").click();
      await expect(page.getByTestId("kill-sheet-8123")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `23-alerts/hog-staged-${vp.name}-${theme}.png`) });
    });

    test(`alerts — idle nudge (passive, no action) — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedRecoveryPage(page, theme, {
        __AIJ_CLEANUP__: cleanupFixture("needs-attention"),
        __AIJ_CLEANUP_ALERTS__: { alerts: [IDLE_ALERT] },
      });
      await goTo(page, "Cleanup");
      await expect(page.getByTestId("idle-nudge-8124")).toBeVisible();
      await expect(page.getByTestId("idle-nudge-8124")).toContainText(
        "has been idle for 2h",
      );
      await expect(page.getByTestId("idle-nudge-8124").locator("button")).toHaveCount(0);
      await page.screenshot({ path: resolve(GALLERY, `23-alerts/idle-nudge-${vp.name}-${theme}.png`) });
    });

    test(`alerts — mixed gate-day state — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedRecoveryPage(page, theme, {
        __AIJ_CLEANUP__: cleanupFixture("needs-attention"),
        __AIJ_CLEANUP_ALERTS__: { alerts: [HOG_ALERT, IDLE_ALERT] },
      });
      await goTo(page, "Cleanup");
      await expect(page.getByTestId("hog-alert-8123")).toBeVisible();
      await expect(page.getByTestId("idle-nudge-8124")).toBeVisible();
      await expect(page.getByTestId("cleanup-row-8124")).toContainText("hog");
      await page.screenshot({ path: resolve(GALLERY, `23-alerts/mixed-${vp.name}-${theme}.png`) });
    });
  }
}

// The recorded keyboard-only pass (AJ-E512 visual testing): open the
// Centre → Tab to the hog alert's stage button → Enter opens the
// verified sheet → Esc closes it → focus RETURNS to the stage button
// (2.4.3). No pointer input anywhere in the pass.
test("alerts — keyboard-only pass: review hog, stage sheet, Esc returns focus", async ({ page }) => {
  await seedRecoveryPage(page, "dark", {
    __AIJ_CLEANUP__: cleanupFixture("needs-attention"),
    __AIJ_CLEANUP_ALERTS__: { alerts: [HOG_ALERT] },
    __AIJ_KILL_VERIFY__: () => ({
      pid: 8123, session: "s-1", verified: true, refusal: null,
      exe_path: "/usr/local/bin/node", cmdline: "node vite dev", cwd: "/srv/web",
      ports: [5173], pgid: 8100, escaped_group: false,
      members: [
        { pid: 8123, pgid: 8100, exe: "node", cmdline: "node vite dev",
          role_guess: "DevServer", unsaved_state_warning: null, from_ledger: true },
      ],
      tree_pids: [8100, 8123], platform: "TERM → grace → KILL", grace_secs: 5,
      provider: "claude-code", account_id: "Work Max", project_id: "web",
    }),
  });
  await goTo(page, "Cleanup");
  await expect(page.getByTestId("hog-alert-8123")).toBeVisible();
  const stage = page.getByTestId("hog-stage-8123");
  // Keyboard-only traversal: Tab until the stage button owns focus.
  for (let i = 0; i < 30; i += 1) {
    const focused = await page.evaluate(
      () => document.activeElement?.getAttribute("data-testid") ?? "",
    );
    if (focused === "hog-stage-8123") break;
    await page.keyboard.press("Tab");
  }
  await expect(stage).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("kill-sheet-8123")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("kill-sheet-8123")).toHaveCount(0);
  await expect(stage).toBeFocused();
  await page.screenshot({ path: resolve(GALLERY, "23-alerts/keyboard-pass-1440x900-dark.png") });
});


// ---------------------------------------------------------------------------
// AJ-E511 — the fleet machine dimension: the quiet Machine column
// (This Mac locally, the machine id on foreign rows), the labelled
// machine filter (default This machine), and the display-only
// foreign-row state (no kill control in the DOM).
// ---------------------------------------------------------------------------

const FLEET_CLEANUP = {
  groups: [
    {
      session: "s-local",
      provider: "claude-code",
      account: "Work Max",
      project: "web",
      machine: "machine-local",
      healthy: false,
      rows: [
        {
          machine_id: "machine-local",
          pid: 8123, pgid: 8100, role: "helper", state: "orphaned",
          state_label: "Still running after its session closed",
          role_guess: "python3 (guess)", exe_path: "/usr/bin/python3",
          cmdline: "python3 -m http.server 5173", age_ms: 9_000_000,
          rss_bytes: 48 * 1024 * 1024, cpu_pct: 1.4, ports: "5173",
          hog: false, hog_rules: [],
          evidence: {
            root_pid: 8100, parent_pid: 8100, pgid: 8100, escaped_group: false,
            verification: { verdict: "verified", detail: "match" },
            first_seen: T0, last_seen: T0 + 90_000,
          },
        },
      ],
    },
    {
      session: "s-local",
      provider: "claude-code",
      account: "Work Max",
      project: "web",
      machine: "machine-studio",
      healthy: false,
      rows: [
        {
          machine_id: "machine-studio",
          pid: 9101, pgid: 9100, role: "helper", state: "orphaned",
          state_label: "Still running after its session closed",
          role_guess: "node (guess)", exe_path: "/usr/local/bin/node",
          cmdline: "node vite dev", age_ms: 4_000_000,
          rss_bytes: 220 * 1024 * 1024, cpu_pct: 0.4, ports: "5174",
          hog: false, hog_rules: [],
          evidence: {
            root_pid: 9100, parent_pid: 9100, pgid: 9100, escaped_group: false,
            verification: { verdict: "verified", detail: "match (remote record)" },
            first_seen: T0, last_seen: T0 + 90_000,
          },
        },
      ],
    },
  ],
  local_machine: "machine-local",
  clean_instances: 2,
  ledger_degraded: false,
};

const STUDIO_PEER = {
  machine_id: "machine-studio", label: "Mac Studio", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
  permission: "view", permission_label: "View only", ask_each_time: false, paired_via: "code",
  paired_at_ms: T0, last_seen_ms: T0, connection: "connected", protocol: "v1.0", connection_detail: null,
};

for (const theme of THEMES) {
  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x700", width: 1024, height: 700 },
  ]) {
    test(`fleet — machine column + filter + foreign display-only — ${vp.name} — ${theme}`, async ({ page }) => {
      // D-25: the machine filter shows once another Mac is paired.
      await seedRecoveryPage(page, theme, { __AIJ_CLEANUP__: FLEET_CLEANUP, __AIJ_FLEET_PEERS__: [STUDIO_PEER] });
      await goTo(page, "Cleanup");
      await expect(page.getByTestId("cleanup-centre")).toBeVisible();
      // The default is This machine — only the local group shows.
      await expect(page.getByTestId("cleanup-machine-8123")).toHaveText("This Mac");
      await expect(page.getByTestId("cleanup-row-9101")).toHaveCount(0);
      // Widen: the foreign row appears, display-only (no kill control).
      await page.getByRole("combobox", { name: "Machine filter" }).selectOption("all");
      await expect(page.getByTestId("cleanup-machine-9101")).toHaveText("machine-studio");
      await expect(page.getByTestId("cleanup-kill-9101")).toHaveCount(0);
      await expect(page.getByTestId("cleanup-kill-8123")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `24-fleet/machine-filter-${vp.name}-${theme}.png`) });
    });
  }
}

/**
 * AJ-E609 — the Usage cost matrix states: priced / unpriced-model /
 * stale-pricing across the viewport × theme matrix. The component
 * reads the __AIJ_COST_ROLLUPS__ seam (no backend), so the states are
 * deterministic fixtures, exactly as the gallery law requires.
 */
const costStates: Record<string, Record<string, unknown>> = {
  priced: {
    rows: [
      {
        key: "sess-alpha", model: "claude-opus-4", day: "2026-10-15",
        tokens: { input: 1_204_318, output: 88_412, cache_read: 902_114, cache_write: 12_004, reasoning: 0 },
        usd: { usd: 24.31, label: "Estimate" }, confidence: "Measured",
      },
      {
        key: "sess-beta", model: "claude-sonnet-4", day: "2026-10-15",
        tokens: { input: 402_118, output: 30_221, cache_read: 200_331, cache_write: 0, reasoning: 0 },
        usd: { usd: 3.92, label: "Estimate" }, confidence: "Measured",
      },
    ],
    subscription: { api_equivalent_usd: 28.23, flat_fee_usd: 200, value_ratio: 0.14 },
    pricing_stale: false, total_usd: 28.23, scope: "ALL",
  },
  "unpriced-model": {
    rows: [
      {
        key: "sess-z", model: "GLM-6-X", day: "2026-10-15",
        tokens: { input: 12_004, output: 1_204, cache_read: 0, cache_write: 0, reasoning: 0 },
        usd: { label: "Estimate" }, credits: 1.25, confidence: "Computed",
      },
    ],
    subscription: {}, pricing_stale: false, scope: "ALL",
  },
  "stale-pricing": {
    rows: [
      {
        key: "sess-alpha", model: "claude-opus-4", day: "2026-09-01",
        tokens: { input: 999_999, output: 50_000, cache_read: 0, cache_write: 0, reasoning: 0 },
        usd: { usd: 15.0, label: "Estimate" }, confidence: "Measured",
      },
    ],
    subscription: {}, pricing_stale: true, total_usd: 15.0,
    pricing_last_synced_ms: T0 - 40 * 24 * 3_600_000, scope: "ALL",
  },
};

for (const theme of THEMES) {
  for (const vp of VIEWPORTS) {
    for (const [state, fixture] of Object.entries(costStates)) {
      test(`cost matrix — ${state} — ${vp.name} — ${theme}`, async ({ page }) => {
        await page.addInitScript(
          ({ rollups, t0, theme: themeName }) => {
            (window as never as { __AIJ_COST_ROLLUPS__: unknown }).__AIJ_COST_ROLLUPS__ = rollups;
            (window as never as { __AIJ_COST_CSV__: unknown }).__AIJ_COST_CSV__ =
              "/tmp/ai-juggler-costs.csv";
            (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = {
              accounts: [], recipes: [], sessions: [], projects: [],
            };
            localStorage.setItem("aij.theme", themeName);
            localStorage.setItem("aij.onboarding.dismissed", "1");
            sessionStorage.setItem("aij.ready.skipped", "1");
            Date.now = () => t0;
          },
          { rollups: fixture, t0: T0, theme },
        );
        await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.goto("/");
        // The matrix lives on the Usage screen.
        await goTo(page, "Usage");
        await expect(
          page.getByText("USD is Estimated — list pricing, not your subscription fee."),
        ).toBeVisible();
        await page.screenshot({
          path: resolve(GALLERY, `25-costs/${state}-${vp.name}-${theme}.png`),
        });
      });
    }
  }
}

/**
 * AJ-E610 — the two-clock surfaces: the Inspector Usage tab (measured
 * fresh / stale / no-data / blocked), the account card's renewal
 * states, and the NewSession row chip — across the required
 * 1440×900 + 1280×800 × light+dark matrix. Fixtures drive the
 * __AIJ_QUOTA_DISPLAY__ seam; states are copy-conformance-checked
 * before capture (badge adjacent, typed empties, never 0%).
 */
const quotaWindow = (over: Record<string, unknown> = {}): Record<string, unknown> => ({
  state: "tracking",
  percentage: 82,
  badge: "measured",
  stale_note_ms: undefined,
  resets_at_ms: T0 + 3 * 3_600_000,
  burned_tokens: 1_234_567,
  ...over,
});

const quotaDisplays: Record<string, Record<string, unknown>[]> = {
  measured: [{ account: "claude-work", five_hour: quotaWindow(), weekly: quotaWindow({ percentage: 41 }), renewal: { state: "set", label: "Renews 2026-10-23", entered_by_user: true }, forecast: { state: "exhausts", tte_min: 55, range_min: 40, range_max: 70, badge: "measured" } }],
  stale: [{ account: "claude-work", five_hour: quotaWindow({ stale_note_ms: 22 * 60 * 1000 }), weekly: quotaWindow({ percentage: 41, stale_note_ms: 22 * 60 * 1000 }), renewal: { state: "approaching", label: "Renews in 6 days — check your plan", entered_by_user: true }, forecast: { state: "wont-exhaust", badge: "measured" } }],
  "no-data": [{ account: "claude-work", five_hour: { state: "unknown", burned_tokens: 0 }, weekly: { state: "learning", burned_tokens: 0 }, renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false }, forecast: { state: "learning", badge: "estimated" } }],
  blocked: [{ account: "claude-work", five_hour: quotaWindow({ state: "exhausted", percentage: 100 }), weekly: quotaWindow({ percentage: 41 }), renewal: { state: "overdue", label: "Renewal date passed", entered_by_user: true }, forecast: { state: "exhausts", tte_min: 0, badge: "measured" } }],
};

const RAIL_QUOTA_VP = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1280x800", width: 1280, height: 800 },
] as const;

for (const theme of THEMES) {
  for (const vp of RAIL_QUOTA_VP) {
    for (const [state, displays] of Object.entries(quotaDisplays)) {
      test(`two-clocks — ${state} — ${vp.name} — ${theme}`, async ({ page }) => {
        await page.addInitScript(
          ({ fixture, t0, theme: themeName, display }) => {
            (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
            (window as never as { __AIJ_QUOTA_DISPLAY__: unknown }).__AIJ_QUOTA_DISPLAY__ = display;
            localStorage.setItem("aij.theme", themeName);
            localStorage.setItem("aij.onboarding.dismissed", "1");
            sessionStorage.setItem("aij.ready.skipped", "1");
            Date.now = () => t0;
          },
          { fixture: allStatuses, t0: T0, theme, display: displays },
        );
        await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.goto("/");
        await expectRail(page); // D-25: the list, or its icon column when narrow
        await page.getByRole("button", { name: /Work Max/ }).first().click();
        await page.getByRole("tab", { name: "Usage" }).click();
        await expect(page.getByTestId("two-clock-card")).toBeVisible();
        // Copy conformance before capture: the badge sits adjacent.
        if (state === "measured") {
          await expect(page.getByText(/5-hour window · Measured/)).toBeVisible();
        }
        if (state === "no-data") {
          await expect(page.getByText("Start a session to measure")).toBeVisible();
        }
        await page.screenshot({
          path: resolve(GALLERY, `26-quota/usage-${state}-${vp.name}-${theme}.png`),
        });
      });
    }
  }
}

/**
 * AJ-E611 — the ranking + notification surfaces: the NewSession
 * ranked list (Suggested badge + top-2 factors + TTE, greyed filtered
 * row with its reason, "Why this order?" expander) and the
 * notification region (actionable 90% line, the hourly digest's
 * renewal item visually distinct, reset-queue queued/ready rows with
 * an explicit Resume click) across the required 1440×900 + 1280×800 ×
 * light+dark matrix. Copy conformance is asserted before every
 * capture (≥2 factors without hover, badge adjacent, no silent
 * drops).
 */
const rankedRows: Record<string, unknown>[] = [
  {
    account: "claude-work",
    score: 0.87,
    top_factors: ["82% headroom (Measured)", "usual account for this repo"],
    factors: [["headroom 82%", 0.37], ["affinity", 0.2], ["fidelity", 0.075], ["cost fit", 0.1], ["calm", 0.1]],
    headroom_unknown: false,
    confidence: "Measured",
    tte_min: 210,
    suggested: true,
  },
  {
    account: "claude-personal",
    score: 0.54,
    top_factors: ["headroom unknown — ranked without quota", "low session load"],
    factors: [["headroom unknown", 0], ["affinity", 0.04], ["fidelity", 0.075], ["cost fit", 0.05], ["calm", 0.1]],
    headroom_unknown: true,
    confidence: "Estimate",
    suggested: false,
  },
  {
    account: "claude-lab",
    filtered_reason: "logged out — sign in from the Ready home",
    top_factors: [],
    factors: [],
    headroom_unknown: true,
    confidence: "Estimate",
    suggested: false,
  },
];

const notificationEvents: Record<string, unknown>[] = [
  { kind: "actionable", account: "claude-work", text: "claude-work at 92% used (Measured), five-hour window — hand over or queue at reset", at_ms: T0 - 60_000 },
  { kind: "digest", account: "claude-personal", text: "Renews in 6 days — check your plan", at_ms: T0 - 3_600_000 },
];

const queueRows: Record<string, unknown>[] = [
  { id: "rq-1", session: "sess-1", account: "claude-work", resume_at_ms: T0 + 45 * 60_000, state: "queued" },
  { id: "rq-2", session: "sess-2", account: "claude-lab", resume_at_ms: T0 - 60_000, state: "ready" },
];

for (const theme of THEMES) {
  for (const vp of RAIL_QUOTA_VP) {
    test(`ranking — NewSession ranked list — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme: themeName, ranked }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_RANKING__ = ranked;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t0: T0, theme, ranked: rankedRows },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
      await expect(page.getByTestId("ranked-rows")).toBeVisible();
      // Copy conformance before capture: Suggested badge, ≥2 factors
      // without hover, TTE, and the greyed filtered row keeps its seat.
      await expect(page.getByText("Suggested")).toBeVisible();
      await expect(page.getByText(/82% headroom \(Measured\)/)).toBeVisible();
      await expect(page.getByText(/exhausts ≈210 min/)).toBeVisible();
      await expect(page.getByText("logged out — sign in from the Ready home")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `27-ranking/ranked-${vp.name}-${theme}.png`),
      });
    });

    test(`ranking — why-this-order expanded — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme: themeName, ranked }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_RANKING__ = ranked;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t0: T0, theme, ranked: rankedRows },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
      await page.getByRole("button", { name: "Why this order?" }).click();
      // All five inputs are listed once expanded (both scored rows
      // expand — assert on the first).
      await expect(page.getByText(/calm/).first()).toBeVisible();
      await expect(page.getByText(/fidelity/).first()).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `27-ranking/why-${vp.name}-${theme}.png`),
      });
    });

    test(`notification — actionable + digest — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme: themeName, events }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_ALERTS__ = events;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t0: T0, theme, events: notificationEvents },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expect(page.getByTestId("alert-actionable")).toContainText("92%");
      // The digest's renewal item is visually distinct (dashed, quiet)
      // and never reads as a window alert.
      await expect(page.getByTestId("alert-digest")).toContainText("Digest · Renews in 6 days");
      await page.screenshot({
        path: resolve(GALLERY, `27-ranking/notification-${vp.name}-${theme}.png`),
      });
    });

    test(`queue — queued + ready with explicit Resume — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme: themeName, rows }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_RESET_QUEUE__ = rows;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t0: T0, theme, rows: queueRows },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expect(page.getByTestId("queue-queued")).toContainText("claude-work");
      await expect(page.getByTestId("queue-ready")).toContainText("ready to resume");
      // The Resume action is keyboard-focusable (the click is the only
      // launch path) — capture it focused.
      await page.getByRole("button", { name: "Resume" }).focus();
      await page.screenshot({
        path: resolve(GALLERY, `27-ranking/queue-${vp.name}-${theme}.png`),
      });
    });
  }
}
/**
 * AJ-E205 — the switch execution surface: the two-choice freeze
 * dialog (default finish-turn; stop-now with the truncation note),
 * preflight warnings rendered before anything moves, the honest
 * labelled stage panel while executing (no fake determinism), and
 * the §7.1 source-safe failure banner — across 1280×800 + 800×600 ×
 * light+dark. Copy conformance asserted before each capture.
 */
const SWITCH_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "800x600", width: 800, height: 600 },
] as const;

const preflightSeams: Record<string, unknown> = {
  "live-default": undefined, // the bridge default: live source, no warnings
};
void preflightSeams;

/** AJ-E206: the door's wizard fixture — an unproven pair (F2,
 * compacted) opens the wizard; the freeze choice + §7.1 warnings
 * render on the Confirm step. */
const SWITCH_WIZARD_FIXTURE = {
  mechanism: "transplant",
  fidelity: "F2",
  score: 0.78,
  warnings: [
    "This conversation was compacted. Early detail carries as text, not as live memory — it may ask about early decisions.",
  ],
  zombies: ["pid 1234 (node vite)"],
  diff_snapshotted: true,
  source_live: true,
  experimental_copy:
    "Experimental: only the main transcript moves. Subagent files and rewind history stay on the source. The source stays resumable.",
  door: "wizard" as const,
  wizard_steps: ["Target", "What's carried", "Confirm"],
  wizard_reasons: [
    "This isn't the usual account for this project.",
    "The carry-list drops: todo list.",
  ],
  usual_account: false,
  carry: {
    carried: ["chat transcript", "rules & guidelines", "uncommitted git diff"],
    regenerated: [],
    dropped: ["todo list", "subagent state"],
  },
};

for (const theme of THEMES) {
  for (const vp of SWITCH_VP) {
    const openDialog = async (page: import("@playwright/test").Page) => {
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await openSwitch(page);
      await page.getByTestId("handover-section")
        // AJ-E209: the ranked, badged row is the switch entry point.
        .locator("button[data-row]:not(:disabled)")
        .first()
        .click();
      await expect(page.getByTestId("switch-flow")).toBeVisible();
    };
    // The wizard's Confirm step (Target → What's carried → seen en
    // route — no execution from an unseen preview).
    const reachConfirm = async (page: import("@playwright/test").Page) => {
      await openDialog(page);
      await expect(page.getByTestId("switch-wizard")).toBeVisible();
      await page.getByTestId("switch-continue").click();
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-step-confirm")).toBeVisible();
    };

    test(`switch — freeze on the wizard Confirm step (finish-turn default) — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_PREFLIGHT__ = fixture.doorFixture;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: { ...allStatuses, doorFixture: SWITCH_WIZARD_FIXTURE }, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await reachConfirm(page);
      await expect(page.getByText("Finish this turn, then switch")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `28-switch/dialog-finish-${vp.name}-${theme}.png`),
      });
    });

    test(`switch — freeze stop-now + warnings on Confirm — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_PREFLIGHT__ = fixture.doorFixture;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: { ...allStatuses, doorFixture: SWITCH_WIZARD_FIXTURE }, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await reachConfirm(page);
      await page.getByText("Stop now and switch").click();
      await expect(page.getByTestId("switch-warnings")).toContainText("compacted");
      await page.screenshot({
        path: resolve(GALLERY, `28-switch/dialog-stop-${vp.name}-${theme}.png`),
      });
    });

    test(`switch — executing (labelled stages) — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_PREFLIGHT__ = fixture.doorFixture;
          w.__AIJ_SWITCH_EXECUTE_HANG__ = true;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: { ...allStatuses, doorFixture: SWITCH_WIZARD_FIXTURE }, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await reachConfirm(page);
      await page.getByTestId("switch-confirm").click();
      await expect(page.getByTestId("switch-executing")).toContainText("Launch");
      await page.screenshot({
        path: resolve(GALLERY, `28-switch/executing-${vp.name}-${theme}.png`),
      });
    });

    test(`switch — failure banner (source safe) — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_PREFLIGHT__ = fixture.doorFixture;
          w.__AIJ_SWITCH_EXECUTE_FAIL__ =
            "S-F5: target launch failed — staged for retry (terminal spawn failed).";
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: { ...allStatuses, doorFixture: SWITCH_WIZARD_FIXTURE }, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await reachConfirm(page);
      await page.getByTestId("switch-confirm").click();
      await expect(page.getByTestId("switch-failed")).toContainText("resume it exactly where you left it");
      await page.screenshot({
        path: resolve(GALLERY, `28-switch/failed-${vp.name}-${theme}.png`),
      });
    });
  }
}

/**
 * AJ-E206 gallery 29 — the door itself (pass2/02 §8): the 1-click
 * popover for a proven pair (carry one-liner + cost + the §5.2
 * freeze choice), the 10-second undo toast after a 1-click switch,
 * and the wizard at 3-step and 5-step renders — only non-trivial
 * steps ever shown, and the Drop list is visible whenever the
 * state is lossy (the P0 visual rule: a lossy state cannot be
 * captured without its Drop list on screen).
 */
const SWITCH_ONECLICK_FIXTURE = {
  mechanism: "transplant",
  fidelity: "F4",
  score: 1.0,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: true,
  experimental_copy:
    "Experimental: only the main transcript moves. Subagent files and rewind history stay on the source. The source stays resumable.",
  door: "one-click" as const,
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: {
    carried: [
      "chat transcript",
      "rules & guidelines",
      "uncommitted git diff",
      "MCP config",
      "skills & commands",
      "model settings",
    ],
    regenerated: [],
    dropped: [],
  },
};

const SWITCH_WIZARD_5STEP_FIXTURE = {
  ...SWITCH_WIZARD_FIXTURE,
  wizard_steps: ["Target", "What's carried", "Conflicts", "Rules & diff", "Confirm"],
  wizard_reasons: [
    "This isn't the usual account for this project.",
    "The carry-list drops: todo list.",
  ],
};

for (const theme of THEMES) {
  for (const vp of SWITCH_VP) {
    const openDoor = async (
      page: import("@playwright/test").Page,
      preflight: unknown,
    ) => {
      await page.addInitScript(
        ({ fixture, door, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_PREFLIGHT__ = door;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, door: preflight, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await openSwitch(page);
      await page.getByTestId("handover-section")
        // AJ-E209: the ranked, badged row is the switch entry point.
        .locator("button[data-row]:not(:disabled)")
        .first()
        .click();
      await expect(page.getByTestId("switch-flow")).toBeVisible();
    };

    test(`door — 1-click popover (proven pair) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openDoor(page, SWITCH_ONECLICK_FIXTURE);
      await expect(page.getByTestId("switch-oneclick")).toBeVisible();
      await expect(page.getByTestId("switch-oneclick-carry")).toContainText("nothing dropped");
      await expect(page.getByTestId("switch-oneclick")).toContainText("usual account");
      await page.screenshot({
        path: resolve(GALLERY, `29-door/oneclick-popover-${vp.name}-${theme}.png`),
      });
    });

    test(`door — undo toast (10 s window) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openDoor(page, SWITCH_ONECLICK_FIXTURE);
      await page.getByTestId("switch-confirm").click();
      await expect(page.getByTestId("switch-undo-toast")).toBeVisible();
      await expect(page.getByTestId("switch-undo")).toContainText("Undo (10s)");
      await page.screenshot({
        path: resolve(GALLERY, `29-door/undo-toast-${vp.name}-${theme}.png`),
      });
    });

    test(`door — wizard Target step (why the wizard opened) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openDoor(page, SWITCH_WIZARD_FIXTURE);
      await expect(page.getByTestId("switch-wizard")).toBeVisible();
      await expect(page.getByTestId("switch-steps")).toContainText("Step 1 of 3: Target");
      await expect(page.getByTestId("switch-wizard-reasons")).toContainText("isn't the usual account");
      await page.screenshot({
        path: resolve(GALLERY, `29-door/wizard-target-${vp.name}-${theme}.png`),
      });
    });

    test(`door — wizard 3-step at What's carried (Drop list visible) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openDoor(page, SWITCH_WIZARD_FIXTURE);
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-steps")).toContainText("Step 2 of 3: What's carried");
      await expect(page.getByTestId("switch-carry-dropped")).toContainText("todo list");
      await page.screenshot({
        path: resolve(GALLERY, `29-door/wizard-3step-carried-${vp.name}-${theme}.png`),
      });
    });

    test(`door — wizard 5-step render (conflicts + rules steps) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openDoor(page, SWITCH_WIZARD_5STEP_FIXTURE);
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-steps")).toContainText("Step 2 of 5: What's carried");
      await expect(page.getByTestId("switch-carry-dropped")).toContainText("todo list");
      await page.screenshot({
        path: resolve(GALLERY, `29-door/wizard-5step-carried-${vp.name}-${theme}.png`),
      });
    });
  }
}

/**
 * AJ-E207 gallery 30 — switch-back (pass2/02 §6.1-6.2): the
 * reset-trigger suggestion card (verbatim copy, suggest-only), the
 * review (delta turn preview + per-file conflict radio trio +
 * freeze-or-fork), the both-live warning, the S-F7 dead-original
 * state, and the S-F8 forked failure — across 1280×800 + 800×600 ×
 * light+dark. Copy conformance asserted before each capture; the
 * copy never says "we switched you".
 */
const BACK_CARD = {
  session_id: "sess-1",
  original_account: "Work Max",
  away_account: "Personal",
  copy: "Work Max has reset. Resume the original conversation there? Work from Personal arrives as one 'while you were away' update.",
};
/** One card per Work Max session — whichever row the table sorts
 * first, its card shows (the filter is by session id). */
const BACK_CARDS = ["sess-1", "sess-2", "sess-4", "sess-6", "sess-8"].map(
  (session_id) => ({ ...BACK_CARD, session_id }),
);

for (const theme of THEMES) {
  for (const vp of SWITCH_VP) {
    const openInspector = async (
      page: import("@playwright/test").Page,
      seams: Record<string, unknown>,
    ) => {
      await page.addInitScript(
        ({ fixture, cards, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_BACK_SUGGESTIONS__ = cards;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
          void w;
        },
        { fixture: allStatuses, cards: BACK_CARDS, t0: T0, theme },
      );
      // Extra seams the caller may carry (plan/execute fixtures).
      for (const [key, value] of Object.entries(seams)) {
        await page.addInitScript(
          ({ key: k, value: v }) => {
            (window as unknown as Record<string, unknown>)[k] = v;
          },
          { key, value },
        );
      }
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("button", { name: /Work Max/ }).first().click();
    };

    test(`switch-back — suggestion card (verbatim, suggest-only) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openInspector(page, {});
      await expect(page.getByTestId("switch-back-card")).toBeVisible();
      await expect(page.getByTestId("switch-back-card-copy")).toContainText(
        "Work Max has reset. Resume the original conversation there?",
      );
      await page.screenshot({
        path: resolve(GALLERY, `30-switchback/card-${vp.name}-${theme}.png`),
      });
    });

    test(`switch-back — review (delta + conflict + freeze-or-fork) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openInspector(page, {});
      await page.getByTestId("switch-back-card").click();
      await expect(page.getByTestId("switch-back-review")).toBeVisible();
      await expect(page.getByTestId("switch-back-delta")).toContainText("While you were away");
      await expect(page.getByTestId("switch-back-conflicts")).toContainText("todos.json");
      await expect(page.getByTestId("switch-back-freeze-or-fork")).toBeVisible();
      const confirm = page.getByTestId("switch-back-confirm");
      await expect(confirm).toBeDisabled();
      await page.screenshot({
        path: resolve(GALLERY, `30-switchback/review-${vp.name}-${theme}.png`),
      });
    });

    test(`switch-back — both-live warning (keep away running) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openInspector(page, {});
      await page.getByTestId("switch-back-card").click();
      await expect(page.getByTestId("switch-back-review")).toBeVisible();
      await page.getByText("Keep it running").click();
      await expect(page.getByTestId("switch-back-both-live")).toContainText(
        "Both accounts are now live on this work item",
      );
      await page.screenshot({
        path: resolve(GALLERY, `30-switchback/both-live-${vp.name}-${theme}.png`),
      });
    });

    test(`switch-back — S-F7 dead original (fresh-f0 offer) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openInspector(page, {
        __AIJ_SWITCH_BACK_PLAN__: {
          suggestion: BACK_CARD.copy,
          original_account: "Work Max",
          away_account: "Personal",
          original_native_id: "",
          original_alive: false,
          sf7_fallback: "fresh-f0",
          sf7_copy:
            "S-F7: the original session is gone — nearest epoch or pack-primed fresh start",
          away_live: false,
          both_live_copy: "",
          divergence: "Both sides moved since the switch. 2 new turns and 0 file conflicts need your choice — nothing is merged silently.",
          delta_preview:
            "While you were away on Personal — 2 new turns (4 shared before the switch).\n\nFull detail: the attached switch pack (not replayed).",
          away_only_turns: 2,
          original_only_turns: 0,
          base_turns: 4,
          conflicts: [],
        },
      });
      await page.getByTestId("switch-back-card").click();
      await expect(page.getByTestId("switch-back-sf7")).toContainText("S-F7");
      await expect(page.getByTestId("switch-back-sf7")).toContainText("fresh session");
      await page.screenshot({
        path: resolve(GALLERY, `30-switchback/sf7-${vp.name}-${theme}.png`),
      });
    });

    test(`switch-back — S-F8 forked (refused merge) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openInspector(page, {
        __AIJ_SWITCH_BACK_EXECUTE_FAIL__:
          "S-F8: merge conflict unresolved — work stays forked, both sides resumable",
      });
      await page.getByTestId("switch-back-card").click();
      await expect(page.getByTestId("switch-back-review")).toBeVisible();
      await page.getByText("Keep original").click();
      await page.getByTestId("switch-back-confirm").click();
      await expect(page.getByTestId("switch-back-failed")).toContainText(
        "both sides stay resumable",
      );
      await page.screenshot({
        path: resolve(GALLERY, `30-switchback/sf8-forked-${vp.name}-${theme}.png`),
      });
    });
  }
}

/**
 * AJ-E207 gallery 31 — the redaction policy surfaces (D-14): the
 * first-run chooser (required, NO pre-selected default — no dark
 * pattern), the verbatim policy line (global choice vs override),
 * the per-item un-redact list on the wizard's preview step, and the
 * per-switch Ask answer on Confirm — across 1280×800 + 800×600 ×
 * light+dark. The destination policy is readable without opening
 * Settings; no secret VALUE ever appears (masked previews only).
 */
const REDACTION_LINE_GLOBAL =
  "Redaction for claude-code: Strip — replace keys with ••• (your global choice)";

for (const theme of THEMES) {
  for (const vp of SWITCH_VP) {
    const openRedactionDoor = async (
      page: import("@playwright/test").Page,
      doorFixture: Record<string, unknown>,
    ) => {
      await page.addInitScript(
        ({ fixture, door, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_PREFLIGHT__ = door;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, door: doorFixture, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await openSwitch(page);
      await page.getByTestId("handover-section")
        // AJ-E209: the ranked, badged row is the switch entry point.
        .locator("button[data-row]:not(:disabled)")
        .first()
        .click();
      await expect(page.getByTestId("switch-flow")).toBeVisible();
    };

    test(`redaction — first-run chooser (no default) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRedactionDoor(page, {
        ...SWITCH_ONECLICK_FIXTURE,
        redaction: { global: null, override: null, effective: null, source: "unset", line: "" },
        redaction_items: [],
      });
      await expect(page.getByTestId("switch-redaction-chooser")).toBeVisible();
      await expect(page.getByTestId("switch-redaction-save")).toBeDisabled();
      // Every option UNCHECKED — the choice is the user's, not ours.
      for (const radio of await page.getByRole("radio", { name: /Strip|Keep|Ask/ }).all()) {
        await expect(radio).not.toBeChecked();
      }
      await expect(page.getByTestId("switch-confirm")).toBeDisabled();
      await page.screenshot({
        path: resolve(GALLERY, `31-redaction/chooser-${vp.name}-${theme}.png`),
      });
    });

    test(`redaction — policy line (global choice) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRedactionDoor(page, {
        ...SWITCH_ONECLICK_FIXTURE,
        redaction: {
          global: "strip",
          override: null,
          effective: "strip",
          source: "global",
          line: REDACTION_LINE_GLOBAL,
        },
        redaction_items: [],
      });
      await expect(page.getByTestId("switch-redaction-line")).toContainText(
        "Strip — replace keys with ••• (your global choice)",
      );
      await expect(page.getByTestId("switch-redaction-none")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `31-redaction/line-global-${vp.name}-${theme}.png`),
      });
    });

    test(`redaction — override + per-item un-redact list — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRedactionDoor(page, {
        ...SWITCH_WIZARD_FIXTURE,
        redaction: {
          global: "keep",
          override: "strip",
          effective: "strip",
          source: "override",
          line: "Redaction for claude-code: Strip — replace keys with ••• (override)",
        },
        redaction_items: [
          { kind: "sk-key", preview: "s•••5678", segment: 0 },
          { kind: "bearer", preview: "c•••aaaa", segment: 1 },
        ],
      });
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-redaction-items")).toBeVisible();
      await expect(page.getByTestId("switch-redaction-line")).toContainText("(override)");
      // Masked previews only — never a value.
      await expect(page.getByText("sk-key")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `31-redaction/items-override-${vp.name}-${theme}.png`),
      });
    });

    test(`redaction — Ask per switch on Confirm — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRedactionDoor(page, {
        ...SWITCH_WIZARD_FIXTURE,
        redaction: {
          global: "ask",
          override: null,
          effective: "ask",
          source: "global",
          line: "Redaction for claude-code: Ask each time (your global choice)",
        },
        redaction_items: [],
      });
      await page.getByTestId("switch-continue").click();
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-step-confirm")).toBeVisible();
      await expect(page.getByTestId("switch-confirm")).toBeDisabled();
      await page.getByText("Strip secrets this time").click();
      await expect(page.getByTestId("switch-confirm")).toBeEnabled();
      await page.screenshot({
        path: resolve(GALLERY, `31-redaction/ask-${vp.name}-${theme}.png`),
      });
    });
  }
}

/**
 * AJ-E209 gallery 32 — the ranked + badged switch entry and the
 * Switch Graph: ranked destinations with explanations + the Why
 * expander, the badge set (unvalidated hatched vs validated solid),
 * the graph's linear A→B→A travel, fork/merge shapes, the failed
 * edge with its §9 reason, the collapsed "+n switches" pill, and the
 * node menu (Resume / Switch from here / Export pack) — across
 * 1280×800 + 800×600 × light+dark. Suggest-only everywhere: every
 * action shot is pre-click, never mid-execution.
 */
const GRAPH_LINEAR = [
  {
    title: "Login refactor",
    epochs: [
      { id: "ep-a", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-1" },
      { id: "ep-b", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-2" },
      { id: "ep-a2", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-3" },
    ],
    edges: [
      { from: "ep-a", to: "ep-b", label: "transplant F1 · 12:04", switch_back: false },
      { from: "ep-b", to: "ep-a2", label: "switch-back ↺ · F1", switch_back: true },
    ],
    collapsed: 2,
    hop_note: "3 switches so far ≈41k tokens",
  },
];

const GRAPH_FORK_MERGE = [
  {
    title: "Payments fix",
    epochs: [
      { id: "ep-s", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-s" },
      { id: "ep-l", provider: "codex", account_label: "Personal", kind: "terminal", resumable: false, session_id: null },
      { id: "ep-r", provider: "zcode", account_label: "Lab", kind: "terminal", resumable: true, session_id: "sess-r" },
      { id: "ep-m", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-m" },
    ],
    edges: [
      { from: "ep-s", to: "ep-l", label: "replay F1 · ≈41k tok", switch_back: false },
      { from: "ep-s", to: "ep-r", label: "transplant F1 · 15:12", switch_back: false },
      { from: "ep-l", to: "ep-m", label: "switch-back ↺ · F1", switch_back: true },
      { from: "ep-r", to: "ep-m", label: "pack F1 · 16:40", switch_back: false },
    ],
    collapsed: 0,
    hop_note: null,
  },
];

const GRAPH_FAILED = [
  {
    title: "Migration script",
    epochs: [
      { id: "ep-src", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-src" },
      { id: "ep-dst", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-dst" },
    ],
    edges: [
      {
        from: "ep-src",
        to: null,
        label: "transplant F1 · target launch failed",
        failed_reason: "S-F6: target window closed mid-launch",
        switch_back: false,
      },
      { from: "ep-src", to: "ep-dst", label: "transplant F1 · 18:02 (retry)", switch_back: false },
    ],
    collapsed: 0,
    hop_note: null,
  },
];

const BADGE_SET_TARGETS = [
  {
    recipe_id: "r-validated",
    account_id: "a-1",
    account_label: "Work Max",
    badge: "F4 · Validated",
    validated: true,
    cross_provider: false,
    drops: [],
    score: 0.71,
    top_factors: ["80% headroom", "usual account for this repo"],
    factors: [["80% headroom", 0.36], ["usual account for this repo", 0.2]],
    confidence: "Measured",
    tte_min: 262,
    suggested: true,
  },
  {
    recipe_id: "r-beta",
    account_id: "a-2",
    account_label: "Personal",
    badge: "F1 · Beta",
    validated: false,
    cross_provider: true,
    drops: ["todo list", "subagent state"],
    score: 0.44,
    top_factors: ["35% headroom", "quiet"],
    factors: [["35% headroom", 0.16], ["quiet", 0.1]],
    confidence: "Computed",
    tte_min: 95,
    suggested: false,
  },
  {
    recipe_id: "r-filtered",
    account_id: "a-3",
    account_label: "Lab",
    badge: "F0 · Beta",
    validated: false,
    cross_provider: true,
    drops: ["rewind history"],
    filtered_reason: "logged out",
    top_factors: [],
    factors: [],
    confidence: "Estimate",
    suggested: false,
  },
];

const openGraphProject = async (
  page: import("@playwright/test").Page,
  graph: unknown[],
  theme: string,
) => {
  await page.addInitScript(
    ({ fixture, workGraph, t0, theme: themeName }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_SWITCH_GRAPH__ = workGraph;
      localStorage.setItem("aij.theme", themeName);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, workGraph: graph, t0: T0, theme },
  );
};

for (const theme of THEMES) {
  for (const vp of SWITCH_VP) {
    test(`graph — ranked targets with explanations — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, targets, t0 }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_TARGETS__ = targets;
          localStorage.setItem("aij.theme", theme);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, targets: BADGE_SET_TARGETS, t0: T0 },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await openSwitch(page);
      await expect(page.getByTestId("switch-target-rows")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `32-graph/targets-${vp.name}-${theme}.png`),
      });
      // The Why expander: every input incl. switch cost.
      await page.getByRole("button", { name: "Why this order?" }).click();
      await page.screenshot({
        path: resolve(GALLERY, `32-graph/targets-why-${vp.name}-${theme}.png`),
      });
    });

    test(`graph — linear A→B→A + hop note — ${vp.name} — ${theme}`, async ({ page }) => {
      await openGraphProject(page, GRAPH_LINEAR, theme);
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await openRailList(page); // D-25: a project row needs the list
      await page.getByRole("button", { name: /^ai-juggler/ }).first().click();
      await expect(page.getByTestId("switch-graph")).toBeVisible();
      await expect(page.getByTestId("switch-graph-hop-note")).toContainText("3 switches");
      await expect(page.getByTestId("switch-graph-collapse")).toContainText("+2 switches");
      await page.screenshot({
        path: resolve(GALLERY, `32-graph/linear-${vp.name}-${theme}.png`),
      });
    });

    test(`graph — fork + merge shapes — ${vp.name} — ${theme}`, async ({ page }) => {
      await openGraphProject(page, GRAPH_FORK_MERGE, theme);
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await openRailList(page); // D-25: a project row needs the list
      await page.getByRole("button", { name: /^ai-juggler/ }).first().click();
      await expect(page.getByTestId("switch-graph")).toBeVisible();
      await expect(page.getByTestId("node-fork-chip").first()).toBeVisible();
      await expect(page.getByTestId("node-merge-chip").first()).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `32-graph/fork-merge-${vp.name}-${theme}.png`),
      });
    });

    test(`graph — failed edge reason + recovery — ${vp.name} — ${theme}`, async ({ page }) => {
      await openGraphProject(page, GRAPH_FAILED, theme);
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await openRailList(page); // D-25: a project row needs the list
      await page.getByRole("button", { name: /^ai-juggler/ }).first().click();
      await expect(page.getByTestId("switch-graph-failed-edge")).toBeVisible();
      await page.getByTestId("switch-graph-failed-edge").click();
      await expect(page.getByTestId("switch-graph-failed-detail")).toContainText("S-F6");
      await page.screenshot({
        path: resolve(GALLERY, `32-graph/failed-${vp.name}-${theme}.png`),
      });
    });

    test(`graph — node menu (Resume / Export pack) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openGraphProject(page, GRAPH_LINEAR, theme);
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await openRailList(page); // D-25: a project row needs the list
      await page.getByRole("button", { name: /^ai-juggler/ }).first().click();
      await expect(page.getByTestId("switch-graph")).toBeVisible();
      await page.getByTestId("switch-graph-node").nth(1).click();
      await expect(page.getByTestId("switch-graph-node-menu")).toBeVisible();
      await expect(page.getByTestId("epoch-resume-button")).toBeEnabled();
      await page.getByTestId("epoch-export-button").click();
      await expect(page.getByTestId("pack-export-toast")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `32-graph/node-menu-${vp.name}-${theme}.png`),
      });
    });

    test(`graph — badge set (validated vs beta) — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, targets, t0 }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_TARGETS__ = targets;
          localStorage.setItem("aij.theme", theme);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, targets: BADGE_SET_TARGETS, t0: T0 },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      const solid = page.locator("[data-testid='fidelity-badge'][data-validated='true']");
      const hatched = page.locator("[data-testid='fidelity-badge'][data-validated='false']");
      await expect(solid).toContainText("F4 · Validated");
      await expect(hatched.first()).toContainText("Not yet validated");
      await page.screenshot({
        path: resolve(GALLERY, `32-graph/badges-${vp.name}-${theme}.png`),
      });
    });
  }
}

/**
 * AJ-E210 gallery 33 — the Beta validation matrix (Settings → Beta,
 * disclosure only): the researched table (all pairs open beta debt),
 * a failed live cell's lowered-ceiling disclosure, a validated pair
 * graduating its badge, and one unvalidated badge in situ on the
 * wizard — across 1280×800 + 800×600 × light+dark. The table is the
 * same one the switch badges derive from (one source, no drift).
 */
const MATRIX_FAILED: import("../src/lib/bridge").ValidationCellData[] = [
  {
    from: "claude-code", to: "claude-code", mechanism: "transplant",
    researched_ceiling: "F1", advertised: "F0", badge: "F0 · Beta",
    validated: false, mechanism_green: true, ci_green: true,
    dogfood: {
      at: "2026-10-16", project_shape: "monorepo", verdict: "fail",
      source_sha256: "aa", target_sha256: "bb", note: "S-F4: rules ring lost",
    },
    status: "Live cell FAILED 2026-10-16 — S-F4: rules ring lost; ceiling lowered one level",
    notes: [],
  },
];

const MATRIX_MIXED: import("../src/lib/bridge").ValidationCellData[] = [
  {
    from: "claude-code", to: "claude-code", mechanism: "transplant",
    researched_ceiling: "F1", advertised: "F1", badge: "F1 · Validated",
    validated: true, mechanism_green: true, ci_green: true,
    dogfood: {
      at: "2026-10-17", project_shape: "tiny", verdict: "pass",
      source_sha256: "aa", target_sha256: "bb",
      note: "target answered the pre-switch question",
    },
    status: "Validated — live cell passed 2026-10-17 (tiny shape)",
    notes: [],
  },
  {
    from: "claude-code", to: "codex", mechanism: "replay",
    researched_ceiling: "F1", advertised: "F1", badge: "F1 · Beta",
    validated: false, mechanism_green: true, ci_green: false, dogfood: null,
    status: "Real-adapter fixtures not landed · live dogfood pending — open beta debt",
    notes: ["codex target adapter not landed"],
  },
];

for (const theme of THEMES) {
  for (const vp of SWITCH_VP) {
    const openProfiles = async (
      page: import("@playwright/test").Page,
      matrix?: unknown[],
    ) => {
      await page.addInitScript(
        ({ fixture, matrix: table, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          if (table !== undefined) w.__AIJ_VALIDATION_MATRIX__ = table;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, matrix, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await goTo(page, "Accounts");
      // D-25: the matrix sits behind its disclosure on Accounts.
      await page.getByTestId("accounts-beta-matrix").locator("summary").click();
      await expect(page.getByTestId("beta-matrix")).toBeVisible();
    };

    test(`beta — matrix, all pairs open debt — ${vp.name} — ${theme}`, async ({ page }) => {
      await openProfiles(page);
      await expect(page.getByTestId("beta-matrix-summary")).toContainText(
        "All 11 tool pairs ship in beta",
      );
      await expect(page.getByTestId("beta-matrix-row").first()).toContainText(
        "Beta — not yet validated",
      );
      await page.screenshot({
        path: resolve(GALLERY, `33-beta/matrix-default-${vp.name}-${theme}.png`),
      });
    });

    test(`beta — failed cell lowers the ceiling — ${vp.name} — ${theme}`, async ({ page }) => {
      await openProfiles(page, MATRIX_FAILED);
      await expect(page.getByTestId("beta-matrix-row").first()).toContainText(
        "lowered from F1 — a live cell failed",
      );
      await page.screenshot({
        path: resolve(GALLERY, `33-beta/matrix-failed-${vp.name}-${theme}.png`),
      });
    });

    test(`beta — validated pair graduates — ${vp.name} — ${theme}`, async ({ page }) => {
      await openProfiles(page, MATRIX_MIXED);
      await expect(page.getByTestId("beta-matrix-summary")).toContainText(
        "1 of 2 pairs validated",
      );
      await expect(page.getByTestId("beta-matrix-row").first()).toContainText("Validated ✓");
      await page.screenshot({
        path: resolve(GALLERY, `33-beta/matrix-validated-${vp.name}-${theme}.png`),
      });
    });

    test(`beta — unvalidated badge in situ on the wizard — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, door, t0, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SWITCH_PREFLIGHT__ = door;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, door: SWITCH_WIZARD_FIXTURE, t0: T0, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await openSwitch(page);
      await page.getByTestId("handover-section")
        .locator("button[data-row]:not(:disabled)")
        .first()
        .click();
      await expect(page.getByTestId("switch-wizard")).toBeVisible();
      // AJ-E808: the register's words for an unvalidated pair, verbatim.
      await expect(page.getByTestId("switch-wizard")).toContainText("· Beta · Not yet validated");
      await page.screenshot({
        path: resolve(GALLERY, `33-beta/badge-in-situ-${vp.name}-${theme}.png`),
      });
    });
  }
}

/**
 * AJ-E305 gallery 37 — flow (b)'s two steps (tiles → preselected
 * account) and the no-Account inline detour (Add account over the sheet,
 * AJ-E1022) — at 1280×800, 1024×768, 390×844 × light+dark. Flow (a) is
 * first run's journey (journey-first-run.spec.ts, gallery 66).
 */
const FLOW_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x768", width: 1024, height: 768 },
  { name: "390x844", width: 390, height: 844 },
] as const;

for (const theme of THEMES) {
  for (const vp of FLOW_VP) {
    test(`flow b — both steps — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: allStatuses, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      // The ⌘N handler mounts with the shell — wait for it (AJ-E806:
      // pressing during the first frames after load is a test race).
      await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
      await page.keyboard.press("Meta+n");
      await expect(page.getByRole("heading", { name: "New project" })).toBeVisible();
      // Step 1: the tiles.
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/b1-tiles-${vp.name}-${theme}.png`),
        fullPage: true,
      });
      await page
        .getByRole("button", { name: /Work in ai-juggler.*last used with Work Max/ })
        .click();
      // Step 2: the preselected account.
      await expect(page.getByTestId("project-account")).toContainText("Work Max");
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/b2-account-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow b — no-Account inline detour — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_RECIPES__ = [];
          localStorage.setItem("aij.theme", themeName);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        {
          fixture: {
            ...allStatuses,
            accounts: [],
            profiles: [],
            sessions: [],
          },
          theme,
        },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      // The ⌘N handler mounts with the shell — wait for it (AJ-E806:
      // pressing during the first frames after load is a test race).
      await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
      await page.keyboard.press("Meta+n");
      await expect(page.getByRole("heading", { name: "New project" })).toBeVisible();
      await page.getByRole("button", { name: /Work in ai-juggler/ }).click();
      await expect(page.getByTestId("no-account-detour")).toBeVisible();
      await page.getByRole("button", { name: "Add an account" }).click();
      // AJ-E1022: Add account opens over the sheet, at what's on this Mac.
      await expect(page.getByTestId("first-run")).toHaveAttribute("data-mode", "add");
      await expect(page.getByTestId("first-run-found")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/b-detour-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });
  }
}

/**
 * AJ-E306 gallery 38 — flows (c)/(d)/(e): the Provider cards
 * (installed / not-installed / unverified / desktop), and the launch
 * sheet's verdict states (OK / Tight / No, logged-out row, unknown
 * headroom) — 1280×800 + 390×844 × light+dark.
 */
const CDE_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "390x844", width: 390, height: 844 },
] as const;

const E306_INSTALLS = [
  {
    provider: "claude-code",
    display_name: "Claude Code",
    install_url: "https://claude.com/download",
    status: "installed",
    version: "2.1.296",
    binary: "/usr/local/bin/claude",
    notes: [],
    verified_against: ["2.0.x", "2.1.x"],
    kind: "terminal",
    single_instance_note: null,
  },
  {
    provider: "fake-desktop",
    display_name: "Fake Desktop",
    install_url: "https://example.com/fake",
    status: "installed",
    version: "0.4.1",
    binary: null,
    notes: [],
    verified_against: [],
    kind: "desktop",
    single_instance_note: "Single-account app — Juggler tracks, cannot isolate",
  },
  {
    provider: "codex",
    display_name: "Codex",
    install_url: "https://developers.openai.com/codex",
    status: "unverified",
    version: "0.11.0",
    binary: "/opt/homebrew/bin/codex",
    notes: [],
    verified_against: [],
    kind: "terminal",
    single_instance_note: null,
  },
  {
    provider: "gemini",
    display_name: "Gemini CLI",
    install_url: "https://gemini.google.com/cli",
    status: "not_installed",
    version: null,
    binary: null,
    notes: [],
    verified_against: [],
    kind: "terminal",
    single_instance_note: null,
  },
];

for (const theme of THEMES) {
  for (const vp of CDE_VP) {
    test(`flow c — provider cards, 4 states — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, installs, theme: t }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_PROVIDER_INSTALLS__ = installs;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: honestEmpty, installs: E306_INSTALLS, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await goTo(page, "Accounts");
      // AJ-E1023: several tools installed → Add account asks which first.
      await page.getByTestId("add-account").click();
      const cards = page.getByTestId("provider-cards");
      await expect(cards).toBeVisible();
      await expect(cards).toContainText("Verified against 2.0.x, 2.1.x");
      await expect(cards).toContainText("Unverified — features may drift");
      await expect(cards).toContainText("one session at a time");
      await expect(cards).toContainText("Install Gemini CLI →");
      await page.screenshot({
        path: resolve(GALLERY, `38-flows-cde/c-provider-cards-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow e — launch sheet: verdicts + logged-out + unknown — ${vp.name} — ${theme}`, async ({
      page,
    }) => {
      // Personal is signed OUT here — the logged-out row state.
      const withLoggedOut = {
        ...allStatuses,
        accounts: allStatuses.accounts.map((account) =>
          account.label === "Personal"
            ? { ...account, login_state: "logged_out" }
            : account,
        ),
      };
      await page.addInitScript(
        ({ fixture, ranking, theme: t }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_RANKING__ = ranking;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        {
          fixture: withLoggedOut,
          theme,
          ranking: [
            {
              account: "claude-work",
              score: 0.9,
              top_factors: [],
              factors: [],
              headroom_unknown: false,
              confidence: "Measured",
              suggested: true,
              admission: "ok",
            },
            {
              account: "claude-personal",
              score: 0.5,
              top_factors: [],
              factors: [],
              headroom_unknown: true,
              confidence: "Estimate",
              suggested: false,
            },
          ],
        },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
      await expect(page.getByTestId("new-session-sheet")).toBeVisible();
      await page
        .getByRole("button", { name: /ai-juggler.*prior sessions/s })
        .click();
      const pre = page.getByTestId("launch-preselected");
      await expect(pre).toContainText("OK — inside headroom");
      await page.screenshot({
        path: resolve(GALLERY, `38-flows-cde/e-launch-ok-${vp.name}-${theme}.png`),
        fullPage: true,
      });
      await page
        .getByRole("button", { name: "Customise ▸" })
        .click();
      await expect(page.getByTestId("sign-in-first")).toBeVisible();
      await expect(page.getByTestId("launch-sheet-accounts")).toContainText(
        "headroom unknown",
      );
      await page.screenshot({
        path: resolve(GALLERY, `38-flows-cde/e-launch-expanded-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow e — Tight and No verdicts — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, ranking, theme: t }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_RANKING__ = ranking;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        {
          fixture: allStatuses,
          theme,
          ranking: [
            {
              account: "claude-work",
              score: 0.9,
              top_factors: [],
              factors: [],
              headroom_unknown: false,
              confidence: "Measured",
              suggested: true,
              admission: "tight",
            },
          ],
        },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
      await page
        .getByRole("button", { name: /ai-juggler.*prior sessions/s })
        .click();
      await expect(page.getByTestId("launch-preselected")).toContainText(
        "Tight — projected burn nears",
      );
      // Tight NEVER blocks: Launch stays enabled.
      await expect(page.getByTestId("launch-now")).toBeEnabled();
      await page.screenshot({
        path: resolve(GALLERY, `38-flows-cde/e-launch-tight-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });
  }
}

/**
 * AJ-E306's successor gallery 39 — AJ-E307 flow (f), first run's folder
 * step since AJ-E1022: the check (normal / oversize / non-config), the
 * duplicate block, the moved-folder state and the sign-in after —
 * 1280×800 + 390×844 × light+dark.
 */
const NORMAL_INSPECT = {
  total_bytes: 46_000_000,
  file_count: 128,
  folders: [
    { name: "projects/", bytes: 40_000_000 },
    { name: "settings.json", bytes: 60 },
    { name: ".credentials.json", bytes: 40 },
  ],
  truncated: false,
  ceiling_bytes: 2_147_483_648,
  is_config: true,
  already_managed: false,
  login: "logged_in",
  keychain_bound: true,
};

// AJ-E1022: the folder import is first run's own step now (the old
// account wizard is gone): the picker, then the check, then the copy.
const openConfirmSource = async (
  page: import("@playwright/test").Page,
  theme: string,
  vp: { width: number; height: number },
  inspect: unknown,
  extra: Record<string, unknown> = {},
) => {
  await page.addInitScript(
    ({ fixture, inspect: probe, theme: t, extra: more }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_SCAN_REPORT__ = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false };
      w.__AIJ_SOURCE_INSPECT__ = probe;
      w.__AIJ_PICK_DIRECTORY__ = "/Users/you/.claude-manual";
      for (const [key, value] of Object.entries(more)) w[key] = value;
      localStorage.setItem("aij.theme", t);
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: honestEmpty, inspect, theme, extra },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  await expect(page.getByTestId("first-run-found")).toBeVisible();
  await page.getByRole("button", { name: "Import from another folder…" }).click();
  await expect(page.getByTestId("first-run-folder-check")).toBeVisible();
};

for (const theme of THEMES) {
  for (const vp of CDE_VP) {
    test(`flow f — confirm-source normal — ${vp.name} — ${theme}`, async ({ page }) => {
      await openConfirmSource(page, theme, vp, NORMAL_INSPECT);
      const panel = page.getByTestId("first-run-folder-check");
      await expect(panel).toContainText("128 files");
      await expect(page.getByTestId("first-run-folder-keychain")).toContainText("macOS Keychain");
      await panel.getByText("What's in it").click();
      await expect(page.getByTestId("source-breakdown")).toContainText("projects/");
      await page.screenshot({
        path: resolve(GALLERY, `39-import/f-normal-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow f — oversize offers the degraded pair — ${vp.name} — ${theme}`, async ({ page }) => {
      await openConfirmSource(page, theme, vp, {
        ...NORMAL_INSPECT,
        total_bytes: 3_300_000_000,
        folders: [{ name: "projects/", bytes: 3_299_000_000 }],
      });
      await expect(page.getByTestId("first-run-folder-large")).toBeVisible();
      await expect(page.getByTestId("first-run-folder-bring-in")).toHaveText("Bring in settings only");
      await expect(page.getByRole("button", { name: "Add a fresh account instead" })).toBeVisible();
      await expect(page.getByTestId("first-run-folder")).not.toContainText("ceiling");
      await page.screenshot({
        path: resolve(GALLERY, `39-import/f-oversize-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow f — non-config rejection — ${vp.name} — ${theme}`, async ({ page }) => {
      await openConfirmSource(page, theme, vp, {
        ...NORMAL_INSPECT,
        is_config: false,
        folders: [{ name: "random.txt", bytes: 10 }],
      });
      const reject = page.getByTestId("source-not-config");
      await expect(reject).toBeVisible();
      await expect(reject).toContainText("doesn't look like a Claude Code setup");
      await expect(page.getByTestId("first-run-folder-bring-in")).toHaveCount(0);
      await page.screenshot({
        path: resolve(GALLERY, `39-import/f-non-config-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow f — duplicate block — ${vp.name} — ${theme}`, async ({ page }) => {
      await openConfirmSource(page, theme, vp, NORMAL_INSPECT, { __AIJ_IMPORT_DUPLICATE_LABEL__: "Work" });
      await page.getByTestId("first-run-folder-bring-in").click();
      await expect(page.getByTestId("first-run-folder-duplicate")).toContainText("Already imported");
      await page.screenshot({
        path: resolve(GALLERY, `39-import/f-duplicate-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow f — moved since the pick — ${vp.name} — ${theme}`, async ({ page }) => {
      await openConfirmSource(page, theme, vp, NORMAL_INSPECT, {
        __AIJ_IMPORT_ERROR__: { code: "io", retryable: false, user_message: "/Users/you/.claude-manual does not exist" },
      });
      await page.getByTestId("first-run-folder-bring-in").click();
      await expect(page.getByTestId("first-run-folder-moved")).toContainText("That folder moved or is gone.");
      await expect(page.getByTestId("first-run-folder-again")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `39-import/f-moved-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow f — brought in, then its sign-in — ${vp.name} — ${theme}`, async ({ page }) => {
      await openConfirmSource(page, theme, vp, NORMAL_INSPECT);
      await page.getByTestId("first-run-folder-bring-in").click();
      await expect(page.getByTestId("first-run-signin-row")).toContainText("claude-manual");
      await page.screenshot({
        path: resolve(GALLERY, `39-import/f-signin-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });
  }
}

/**
 * AJ-E308 gallery 40 — the shared login card's states (Waiting /
 * Partial / Expired / SSO / ApiKey / SignedIn) via the Ready Home
 * door, plus the Summary's isolation checklist (pass + a forced
 * fail that blocks Launch) — 1280×800 + 390×844 × light+dark.
 */
for (const theme of THEMES) {
  for (const vp of CDE_VP) {
    for (const state of ["logged_out", "partial", "expired", "sso_pending", "api_key", "signed_in"] as const) {
      test(`login card — ${state} — ${vp.name} — ${theme}`, async ({ page }) => {
        await page.addInitScript(
          ({ fixture, state: loginState, theme: t }) => {
            const w = window as unknown as Record<string, unknown>;
            w.__AIJ_FIXTURE__ = fixture;
            w.__AIJ_LOGIN_STATE__ = loginState;
            localStorage.setItem("aij.theme", t);
          },
          { fixture: readyHome, state, theme },
        );
        await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.goto("/");
        // D-25: Accounts › Sign in opens the shared queue card (Home's
        // Sign in goes straight to the login terminal instead).
        await goTo(page, "Accounts");
        const signIn = page.getByRole("button", { name: /^Sign in$/ }).first();
        await expect(signIn).toBeVisible();
        await signIn.click();
        const card = page.getByTestId("login-queue-card");
        await expect(card).toBeVisible();
        await expect(card).toHaveAttribute("data-state", state);
        await page.screenshot({
          path: resolve(GALLERY, `40-login/card-${state}-${vp.name}-${theme}.png`),
          fullPage: true,
        });
      });
    }
  }
}

// AJ-E1022 — 71-add-account: Add account with nothing new on this Mac
// opens at a fresh account (its name filled in), then its sign-in —
// 1280×800 + 390×844 × light+dark.
for (const theme of THEMES) {
  for (const vp of CDE_VP) {
    test(`71 add account — fresh, then its sign-in — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SCAN_REPORT__ = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false };
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: allStatuses, t: theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await goTo(page, "Accounts");
      await page.getByTestId("add-account").click();
      await expect(page.getByTestId("first-run-fresh")).toBeVisible();
      await expect(page.getByTestId("first-run-fresh-name")).toHaveValue(/^Claude Code/);
      await page.screenshot({ path: resolve(GALLERY, `71-add-account/fresh-${vp.name}-${theme}.png`), fullPage: true });
      await page.getByTestId("first-run-fresh-create").click();
      await expect(page.getByTestId("first-run-signin-row")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `71-add-account/fresh-signin-${vp.name}-${theme}.png`), fullPage: true });
    });
  }
}

// AJ-E1025 — 72-tool-view: the tool's own view, opened from a session's
// tool chip in the header ("About Claude Code"). It reads what is
// installed in plain words and shows no task or decision code on any tab
// (D-25) — 1280×800 + 1024×680 (the Tauri minimum) × light+dark.
const TOOL_VIEW_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x680", width: 1024, height: 680 },
] as const;
for (const theme of THEMES) {
  for (const vp of TOOL_VIEW_VP) {
    test(`72 tool view — from a session's tool chip — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t, t0 }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t: theme, t0: T0 },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page);
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      const chip = page.getByRole("button", { name: "About Claude Code" });
      await expect(chip).toBeVisible();
      await chip.click();
      await expect(page.getByRole("heading", { level: 1, name: "Claude Code" })).toBeVisible();
      await expect(chip).toHaveCount(0); // the view doesn't link to itself
      await expect(page.getByTestId("provider-install")).toHaveText(/^Version \S+/);
      await page.screenshot({ path: resolve(GALLERY, `72-tool-view/overview-${vp.name}-${theme}.png`), fullPage: true });
      for (const tab of ["Overview", "Usage", "History", "Quality / Performance", "Processes", "Actions"]) {
        await page.getByRole("tab", { name: tab }).click();
        const text = (await page.getByRole("tabpanel").first().innerText()) ?? "";
        expect(text, `tool view › ${tab}`).not.toMatch(/\bAJ-E\d+|\bE\d+\+|\(E\d+\)|\bD-\d{2}\b|lands with/);
      }
      await page.screenshot({ path: resolve(GALLERY, `72-tool-view/actions-${vp.name}-${theme}.png`), fullPage: true });
    });
  }
}

// AJ-E309 — 41-policy: Accounts → Pack secrets (the first-run question
// went with the old account wizard; D-14: the first switch asks).
const shot41 = (page: import("@playwright/test").Page, name: string) =>
  page.screenshot({ path: resolve(GALLERY, `41-policy/${name}.png`), fullPage: true });

// Accounts is a main-shell screen: captured at the desktop sizes the
// Tauri window allows (its minimum is 1024×680), not phone width.
const ACCOUNTS_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x680", width: 1024, height: 680 },
] as const;

for (const theme of THEMES) {
  for (const vp of ACCOUNTS_VP) {
    test(`41 policy — Accounts → Pack secrets + env overrides — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_RECIPE_ENV__ = { "p-1": { CLAUDE_CODE_MAX_OUTPUT_TOKENS: "8000" } };
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: allStatuses, t: theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await goTo(page, "Accounts");
      await page.getByRole("button", { name: /Edit launch settings/ }).first().click();
      await page.getByTestId("recipe-env-toggle").first().click();
      await expect(page.getByTestId("env-overlay-editor")).toBeVisible();
      await shot41(page, `accounts-pack-secrets-${vp.name}-${theme}`);
      // D-25: the choice itself is in Settings, one link from Accounts.
      await page.getByTestId("accounts-pack-settings").click();
      await expect(page.getByTestId("pack-secrets")).toBeVisible();
      await expect(page.getByTestId("pack-secrets-unset")).toBeVisible();
      await shot41(page, `settings-pack-secrets-${vp.name}-${theme}`);
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E702 — 42-fleet: this Mac's identity (fingerprint sheet), the
// paired list, the Unpair confirmation, and the fail-closed "Trust
// broken" banner — 1280×800 and 1024×680 (the window minimum), both
// themes.
// ---------------------------------------------------------------------

const FLEET_PEER = {
  machine_id: "machine-mini",
  label: "Mac mini",
  fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
  permission: "view",
  permission_label: "View only",
  ask_each_time: false,
  paired_via: "code",
  paired_at_ms: T0,
  last_seen_ms: T0 + 60_000,
  connection: "connected",
  protocol: "v1.0",
  connection_detail: null,
};

const openFleet = async (
  page: import("@playwright/test").Page,
  theme: string,
  vp: { width: number; height: number },
  seams: Record<string, unknown>,
) => {
  await page.addInitScript(
    ({ fixture, t, extra, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      Object.assign(w, extra);
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t: theme, extra: seams, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  await goTo(page, "Fleet");
  await expect(page.getByTestId("fleet-this-mac")).toBeVisible();
};

for (const theme of THEMES) {
  for (const vp of ACCOUNTS_VP) {
    test(`42 fleet — this Mac, no peers — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, { __AIJ_FLEET_PEERS__: [] });
      await expect(page.getByTestId("fleet-fingerprint")).toHaveAttribute("aria-label", "Machine fingerprint");
      await expect(page.getByTestId("fleet-peers-empty")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `42-fleet/this-mac-${vp.name}-${theme}.png`), fullPage: true });
    });

    test(`42 fleet — paired list — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, {
        __AIJ_FLEET_PEERS__: [FLEET_PEER, { ...FLEET_PEER, machine_id: "m2", label: "Office iMac", permission: "control", permission_label: "View + control", paired_via: "account", last_seen_ms: null }],
      });
      await expect(page.getByTestId("fleet-peer")).toHaveCount(2);
      await page.screenshot({ path: resolve(GALLERY, `42-fleet/paired-${vp.name}-${theme}.png`), fullPage: true });
    });

    test(`42 fleet — unpair confirmation — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, { __AIJ_FLEET_PEERS__: [FLEET_PEER] });
      await page.getByTestId("fleet-unpair").click();
      await expect(page.getByTestId("fleet-confirm")).toContainText("Unpair Mac mini?");
      await page.screenshot({ path: resolve(GALLERY, `42-fleet/unpair-confirm-${vp.name}-${theme}.png`), fullPage: true });
    });

    test(`42 fleet — degraded and offline banners — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, {
        __AIJ_FLEET_PEERS__: [
          { ...FLEET_PEER, connection: "degraded", protocol: "v1.1", connection_detail: "Peer v1.1, this Mac v1.0 — telemetry only until both are updated" },
          { ...FLEET_PEER, machine_id: "m3", label: "Office iMac", connection: "offline" },
        ],
      });
      await expect(page.getByTestId("fleet-connection-banner")).toHaveCount(2);
      await page.screenshot({ path: resolve(GALLERY, `42-fleet/connection-banners-${vp.name}-${theme}.png`), fullPage: true });
    });

    test(`42 fleet — trust broken — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, {
        __AIJ_FLEET_PEERS__: [],
        __AIJ_FLEET_SELF__: { trust: "broken", trust_reason: "an older copy of the registry was restored (rollback)" },
      });
      await expect(page.getByTestId("fleet-trust-broken")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `42-fleet/trust-broken-${vp.name}-${theme}.png`), fullPage: true });
    });
  }
}


// ---------------------------------------------------------------------
// AJ-E704 — 43-pairing: the pairing wizard's three steps (Find → Verify
// → Permissions), Done, and the distinct failure states (no machines
// found, code mismatch, peer cancelled, window closed, paused, turned
// off) — 1280×800 and 1024×680 (the window minimum), both themes.
// ---------------------------------------------------------------------

const LINK_ON = { enabled: true, port: 47821, error: null };
const UNNAMED = [{ instance_id: "a1b2c3d4", label: "Unnamed machine (a1b2c3d4)", version: "1.0" }];
const simulate = (page: import("@playwright/test").Page, event: string) =>
  page.evaluate((e) => (window as unknown as { __AIJ_PAIR_SIMULATE__: (x: string) => void }).__AIJ_PAIR_SIMULATE__(e), event);
const shot43 = (page: import("@playwright/test").Page, name: string, vp: { name: string }, theme: string) =>
  page.screenshot({ path: resolve(GALLERY, `43-pairing/${name}-${vp.name}-${theme}.png`) });

for (const theme of THEMES) {
  for (const vp of ACCOUNTS_VP) {
    test(`43 pairing — Link on, host steps — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, { __AIJ_FLEET_PEERS__: [], __AIJ_LINK__: LINK_ON });
      await page.screenshot({ path: resolve(GALLERY, `43-pairing/link-on-${vp.name}-${theme}.png`), fullPage: true });
      await page.getByTestId("fleet-add").click();
      await expect(page.getByTestId("pairing-code")).toHaveText("482 913");
      await shot43(page, "1-host-find", vp, theme);
      await simulate(page, "joiner");
      await expect(page.getByTestId("pairing-verify")).toBeVisible();
      await shot43(page, "2-verify", vp, theme);
      await page.getByTestId("pairing-fingerprints-match").click();
      await expect(page.getByTestId("pairing-permissions")).toBeVisible();
      await shot43(page, "3-permissions", vp, theme);
      await page.getByTestId("pairing-pair").click();
      await expect(page.getByTestId("pairing-done")).toBeVisible();
      await shot43(page, "4-done", vp, theme);
    });

    test(`43 pairing — join find + failures — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, { __AIJ_FLEET_PEERS__: [], __AIJ_LINK__: LINK_ON, __AIJ_DISCOVERED__: UNNAMED });
      await page.getByTestId("fleet-join").click();
      await expect(page.getByTestId("pairing-machine")).toBeVisible();
      await page.getByRole("textbox", { name: "Digit 1 of 6" }).focus();
      await page.keyboard.type("482913");
      await shot43(page, "1-join-find", vp, theme);
      await page.getByRole("textbox", { name: "Digit 1 of 6" }).focus();
      await page.keyboard.type("000000");
      await page.getByTestId("pairing-connect").click();
      await expect(page.getByTestId("pairing-failure")).toHaveAttribute("data-reason", "mismatch");
      await shot43(page, "fail-mismatch", vp, theme);
    });

    test(`43 pairing — no machines found + by address — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, { __AIJ_FLEET_PEERS__: [], __AIJ_LINK__: LINK_ON, __AIJ_DISCOVERED__: [] });
      await page.getByTestId("fleet-join").click();
      await expect(page.getByTestId("pairing-none-found")).toBeVisible();
      await shot43(page, "fail-none-found", vp, theme);
      await page.getByTestId("pairing-toggle-address").click();
      await page.getByTestId("pairing-address").fill("100.64.0.2");
      await shot43(page, "1-join-address", vp, theme);
    });

    test(`43 pairing — window closed, cancelled, wrong-code notice — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, { __AIJ_FLEET_PEERS__: [], __AIJ_LINK__: LINK_ON });
      await page.getByTestId("fleet-add").click();
      await expect(page.getByTestId("pairing-code")).toBeVisible();
      await simulate(page, "mismatch");
      await expect(page.getByTestId("pairing-notice")).toHaveAttribute("data-reason", "mismatch");
      await expect(page.getByTestId("pairing-code")).toHaveText("731 058");
      await shot43(page, "host-new-code", vp, theme);
      await simulate(page, "expire");
      await expect(page.getByTestId("pairing-failure")).toHaveAttribute("data-reason", "expired");
      await shot43(page, "fail-expired", vp, theme);
    });

    test(`43 pairing — peer cancelled (joiner) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, {
        __AIJ_FLEET_PEERS__: [], __AIJ_LINK__: LINK_ON, __AIJ_DISCOVERED__: UNNAMED, __AIJ_PAIR_JOIN_FAIL__: "cancelled",
      });
      await page.getByTestId("fleet-join").click();
      await page.getByRole("textbox", { name: "Digit 1 of 6" }).focus();
      await page.keyboard.type("482913");
      await page.getByTestId("pairing-connect").click();
      await expect(page.getByTestId("pairing-failure")).toHaveAttribute("data-reason", "cancelled");
      await shot43(page, "fail-cancelled", vp, theme);
    });

    test(`43 pairing — paused and turned off — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFleet(page, theme, vp, { __AIJ_FLEET_PEERS__: [], __AIJ_LINK__: LINK_ON, __AIJ_PAIR_LOCK__: "locked_out" });
      await page.getByTestId("fleet-add").click();
      await expect(page.getByTestId("pairing-failure")).toHaveAttribute("data-reason", "locked_out");
      await shot43(page, "fail-locked", vp, theme);
      await page.evaluate(() => {
        (window as unknown as { __AIJ_PAIR_LOCK__: string }).__AIJ_PAIR_LOCK__ = "disabled";
      });
      await page.getByTestId("pairing-retry").click();
      await expect(page.getByTestId("pairing-failure")).toHaveAttribute("data-reason", "disabled");
      await expect(page.getByTestId("pairing-retry")).toHaveText("Turn pairing back on");
      await shot43(page, "fail-disabled", vp, theme);
    });
  }
}
// ---------------------------------------------------------------------
// AJ-E1203 — 44-settings: Settings → Appearance + Account & Cloud in
// every state (signed out, pairing code, linking, consent, connected,
// paid-locked, unlinked notice, offline) — 1280×800 and 1024×680 ×
// light/dark.
// ---------------------------------------------------------------------

const FH_LINKED = {
  state: "linked",
  email: "you@example.com",
  plan: "Free",
  device_name: "Studio Mac",
  features: [
    { key: "cloudBackup", access: "included", tier: "free", on: true },
    { key: "fleetDiscovery", access: "included", tier: "free", on: true },
    { key: "usageSync", access: "included", tier: "free", on: false },
  ],
  link: null,
  refreshed_at_ms: T0 - 3_600_000,
  offline: false,
  notice: null,
  consent_version: 1,
  consent_current: 1,
};

const openSettings = async (
  page: import("@playwright/test").Page,
  theme: string,
  vp: { width: number; height: number },
  seams: Record<string, unknown>,
) => {
  await page.addInitScript(
    ({ fixture, t, extra, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      Object.assign(w, extra);
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t: theme, extra: seams, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  await page.getByRole("button", { name: "Settings" }).first().click();
  await expect(page.getByTestId("account-cloud")).toBeVisible();
};

const shot44 = (page: import("@playwright/test").Page, name: string, vp: { name: string }, theme: string) =>
  page.screenshot({ path: resolve(GALLERY, `44-settings/${name}-${vp.name}-${theme}.png`), fullPage: true });

for (const theme of THEMES) {
  for (const vp of ACCOUNTS_VP) {
    test(`44 settings — signed out, code, linking — ${vp.name} — ${theme}`, async ({ page }) => {
      await openSettings(page, theme, vp, {});
      await expect(page.getByTestId("cloud-signed-out")).toBeVisible();
      await shot44(page, "signed-out", vp, theme);
      await page.getByTestId("cloud-use-code").click();
      await shot44(page, "pairing-code", vp, theme);
      await page.getByTestId("cloud-connect").click();
      await expect(page.getByTestId("cloud-user-code")).toHaveText("QVM3-GABM");
      await shot44(page, "linking", vp, theme);
    });

    test(`44 settings — consent, connected, paid-locked — ${vp.name} — ${theme}`, async ({ page }) => {
      await openSettings(page, theme, vp, {
        __AIJ_FIGUREHEAD__: { ...FH_LINKED, consent_version: 0, features: FH_LINKED.features.map((f) => ({ ...f, on: false })) },
      });
      await expect(page.getByTestId("cloud-consent")).toBeVisible();
      await shot44(page, "consent", vp, theme);
      await page.getByTestId("cloud-turn-on").click();
      await expect(page.getByTestId("cloud-connected")).toBeVisible();
      await shot44(page, "connected", vp, theme);
    });

    test(`44 settings — paid-locked, offline, unlinked — ${vp.name} — ${theme}`, async ({ page }) => {
      await openSettings(page, theme, vp, {
        __AIJ_FIGUREHEAD__: {
          ...FH_LINKED,
          offline: true,
          features: [...FH_LINKED.features, { key: "transcriptVault", access: "upgrade", tier: "paid", on: false }],
        },
      });
      await expect(page.getByTestId("cloud-upgrade")).toBeVisible();
      await expect(page.getByTestId("cloud-offline")).toBeVisible();
      await shot44(page, "paid-locked-offline", vp, theme);
      await page.getByTestId("cloud-unlink").click();
      await expect(page.getByTestId("cloud-unlink-confirm")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `44-settings/unlink-confirm-${vp.name}-${theme}.png`) });
      await page.getByTestId("cloud-unlink-yes").click();
      await expect(page.getByTestId("cloud-signed-out")).toBeVisible();
      await page.evaluate(() => {
        const w = window as unknown as { __AIJ_FIGUREHEAD__: Record<string, unknown> };
        w.__AIJ_FIGUREHEAD__ = { ...w.__AIJ_FIGUREHEAD__, notice: "This Mac was unlinked from Figurehead." };
      });
      await goTo(page, "Fleet");
      await page.getByRole("button", { name: "Settings" }).first().click();
      await expect(page.getByTestId("cloud-notice")).toBeVisible();
      await shot44(page, "unlinked", vp, theme);
    });
  }
}
// ---------------------------------------------------------------------
// AJ-E705 — 45-remote: a paired machine in the SAME six-tab inspector —
// Instance Overview / Processes (live, stale, offline, view-only), the
// machine overview — at 1440×900 and 1024×768, light/dark.
// ---------------------------------------------------------------------

const REMOTE_VP = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1024x768", width: 1024, height: 768 },
] as const;

const remoteMachine = (over: Record<string, unknown> = {}) => ({
  machine_id: "machine-mini",
  label: "Mac mini",
  connection: "connected",
  freshness: "live",
  last_heard_ms: T0 - 1_000,
  permission: "control",
  resources: { cpu_percent: 23.4, memory_used_bytes: 12 * 1024 ** 3, memory_total_bytes: 32 * 1024 ** 3, measured_at_ms: T0 - 1_000 },
  instances: [
    {
      id: "inst-1", provider: "claude-code", account_label: "Work Max", status: "running",
      model: "claude-opus-4", model_source: "transcript", project_name: "web-shop",
      started_at_ms: T0 - 3_600_000, last_activity_ms: T0 - 60_000,
      cpu_percent: 4.2, rss_bytes: 420 * 1024 ** 2, process_count: 3, measured_at_ms: T0 - 1_000,
    },
    {
      id: "inst-2", provider: "codex", account_label: "Personal", status: "waiting_user",
      model: null, model_source: "unknown", project_name: "api",
      started_at_ms: T0 - 7_200_000, last_activity_ms: null,
      cpu_percent: null, rss_bytes: null, process_count: null, measured_at_ms: T0 - 1_000,
    },
  ],
  ...over,
});

const remotePeer = {
  ...FLEET_PEER,
  machine_id: "machine-mini",
};

const openRemote = async (
  page: import("@playwright/test").Page,
  theme: string,
  vp: { width: number; height: number },
  machine: Record<string, unknown>,
) => {
  await openFleet(page, theme, vp, {
    __AIJ_FLEET_PEERS__: [remotePeer],
    __AIJ_LINK__: { enabled: true, port: 47821, error: null },
    __AIJ_FLEET_REMOTE__: [machine],
  });
};

for (const theme of THEMES) {
  for (const vp of REMOTE_VP) {
    test(`45 remote — live instance, all tabs — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRemote(page, theme, vp, remoteMachine());
      await page.getByTestId("fleet-remote-instance").first().click();
      await expect(page.getByTestId("remote-freshness")).toHaveAttribute("data-freshness", "live");
      await page.screenshot({ path: resolve(GALLERY, `45-remote/instance-overview-live-${vp.name}-${theme}.png`) });
      await page.getByRole("tab", { name: "Processes" }).click();
      await expect(page.getByTestId("remote-processes")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `45-remote/instance-processes-${vp.name}-${theme}.png`) });
      await page.getByRole("tab", { name: "Usage" }).click();
      await expect(page.getByTestId("remote-unavailable").first()).toBeVisible();
      await page.getByRole("tab", { name: "Overview" }).click();
    });

    test(`45 remote — stale, offline, view-only — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRemote(page, theme, vp, remoteMachine({ freshness: "stale", last_heard_ms: T0 - 40_000 }));
      await page.getByTestId("fleet-remote-instance").first().click();
      await expect(page.getByTestId("remote-freshness")).toHaveAttribute("data-freshness", "stale");
      await page.screenshot({ path: resolve(GALLERY, `45-remote/instance-overview-stale-${vp.name}-${theme}.png`) });
      await page.evaluate(() => {
        const w = window as unknown as { __AIJ_FLEET_REMOTE__: Array<Record<string, unknown>> };
        w.__AIJ_FLEET_REMOTE__ = w.__AIJ_FLEET_REMOTE__.map((m) => ({ ...m, connection: "offline", freshness: "offline", permission: "view" }));
      });
      await expect(page.getByTestId("remote-freshness")).toHaveAttribute("data-freshness", "offline");
      await page.screenshot({ path: resolve(GALLERY, `45-remote/instance-overview-offline-${vp.name}-${theme}.png`) });
      await page.getByRole("tab", { name: "Actions" }).click();
      await expect(page.getByTestId("remote-stop")).toBeDisabled();
      await page.screenshot({ path: resolve(GALLERY, `45-remote/instance-actions-view-only-${vp.name}-${theme}.png`) });
    });

    test(`45 remote — machine overview — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRemote(page, theme, vp, remoteMachine());
      await page.getByTestId("fleet-inspect").click();
      await expect(page.getByTestId("remote-machine-overview")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `45-remote/machine-overview-${vp.name}-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E709 — 46-fleet-board: the board in All machines and a single-
// machine filter, a Fleet alert, the Access sheet and the "Ask each
// time" prompt — 1440×900 and 1024×768, light/dark.
// ---------------------------------------------------------------------

for (const theme of THEMES) {
  for (const vp of REMOTE_VP) {
    test(`46 fleet board — all machines, one machine, alert — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRemote(page, theme, vp, {
        ...remoteMachine(),
        alerts: [{ kind: "exhausted", account: "Work Max", text: "Work Max hit its 5-hour limit — resets at 14:00.", at_ms: T0 - 30_000 }],
      });
      // Back to the home view (the board + the alert region).
      await page.reload();
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await expect(page.getByTestId("board-machine")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `46-fleet-board/all-machines-${vp.name}-${theme}.png`) });
      await openFilters(page);
      await page.getByLabel("Filter by machine").selectOption("machine-mini");
      await expect(page.getByTestId("board-machine-local")).toHaveCount(0);
      await page.screenshot({ path: resolve(GALLERY, `46-fleet-board/one-machine-${vp.name}-${theme}.png`) });
      await openFilters(page);
      await page.getByLabel("Filter by machine").selectOption("all");
      await expect(page.getByTestId("remote-alert-exhausted")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `46-fleet-board/alert-${vp.name}-${theme}.png`) });
    });

    test(`46 fleet board — access sheet and ask-each-time prompt — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRemote(page, theme, vp, remoteMachine());
      await page.getByTestId("fleet-access").click();
      await expect(page.getByTestId("fleet-access-sheet")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `46-fleet-board/access-sheet-${vp.name}-${theme}.png`) });
      await page.keyboard.press("Escape");
      await page.evaluate((t0) => {
        (window as unknown as Record<string, unknown>).__AIJ_FLEET_PROMPTS__ = [
          { id: 1, peer_label: "Mac mini", action: "stop Work Max (claude-code)", expires_at_ms: t0 + 18_000 },
        ];
      }, T0);
      await expect(page.getByTestId("fleet-prompt")).toBeVisible({ timeout: 8000 });
      await page.screenshot({ path: resolve(GALLERY, `46-fleet-board/ask-each-time-${vp.name}-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E711 — 47-remote-focus: the four honest focus outcomes in the
// remote inspector, the palette with mixed local/remote Instances, and
// the View-only refusal — 1280×800, light/dark.
// ---------------------------------------------------------------------

const desktopMachine = (over: Record<string, unknown> = {}) =>
  remoteMachine({
    instances: [
      {
        id: "inst-desk", provider: "claude-code", account_label: "Desk", status: "running",
        model: null, model_source: "unknown", project_name: "web-shop", started_at_ms: T0 - 600_000,
        last_activity_ms: T0 - 10_000, cpu_percent: null, rss_bytes: null, process_count: null,
        kind: "desktop", measured_at_ms: T0 - 1_000,
      },
    ],
    alerts: [],
    ...over,
  });

for (const theme of THEMES) {
  test(`47 remote focus — the four outcomes — ${theme}`, async ({ page }) => {
    await openRemote(page, theme, { width: 1280, height: 800 }, desktopMachine());
    await page.getByTestId("fleet-remote-instance").first().click();
    await page.getByRole("tab", { name: "Actions" }).click();
    const outcomes: Array<[string, { accepted: boolean; reason_code: string | null; message: string }]> = [
      ["focused", { accepted: true, reason_code: null, message: "Focused window" }],
      ["window-not-found", { accepted: true, reason_code: null, message: "App opened — window not found" }],
      ["not-running", { accepted: true, reason_code: null, message: "App not running — Relaunch?" }],
      ["permission-needed", { accepted: true, reason_code: null, message: "Permission needed on Mac mini" }],
    ];
    for (const [name, outcome] of outcomes) {
      await page.evaluate((o) => {
        (window as unknown as Record<string, unknown>).__AIJ_REMOTE_FOCUS__ = o;
      }, outcome);
      await page.getByTestId("remote-focus").click();
      await expect(page.getByTestId("remote-outcome")).toContainText(outcome.message);
      await page.screenshot({ path: resolve(GALLERY, `47-remote-focus/outcome-${name}-1280x800-${theme}.png`) });
    }
  });

  test(`47 remote focus — palette mixed + view-only — ${theme}`, async ({ page }) => {
    await openRemote(page, theme, { width: 1280, height: 800 }, desktopMachine());
    await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
    await page.keyboard.press("Meta+k");
    await expect(page.getByTestId("goto-remote-machine").first()).toContainText("on Mac mini");
    await page.screenshot({ path: resolve(GALLERY, `47-remote-focus/palette-mixed-1280x800-${theme}.png`) });
    await page.keyboard.press("Escape");
    await page.evaluate(() => {
      const w = window as unknown as { __AIJ_FLEET_REMOTE__: Array<Record<string, unknown>> };
      w.__AIJ_FLEET_REMOTE__ = w.__AIJ_FLEET_REMOTE__.map((m) => ({ ...m, permission: "view" }));
    });
    await page.getByTestId("fleet-remote-instance").first().click();
    await page.getByRole("tab", { name: "Actions" }).click();
    await expect(page.getByTestId("remote-focus")).toBeDisabled();
    await page.screenshot({ path: resolve(GALLERY, `47-remote-focus/view-only-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E707 — 48-remote-cleanup: the Cleanup Centre grouped by machine →
// Instance with verified / not-owned rows, the confirmation and the
// outcome — 1440×900 and 1024×768, light/dark.
// ---------------------------------------------------------------------

const cleanupMachine = (over: Record<string, unknown> = {}) =>
  remoteMachine({
    alerts: [],
    processes: [
      { instance_id: "inst-1", provider: "claude-code", account_label: "Work Max", pid: 4242, role: "helper", state: "orphaned",
        state_label: "Still running after its session closed", verified: true, name: "node", age_ms: 3_600_000,
        rss_bytes: 210 * 1024 ** 2, cpu_percent: 1.2, ports: "3000", measured_at_ms: T0 - 1_000 },
      { instance_id: "inst-1", provider: "claude-code", account_label: "Work Max", pid: 4250, role: "helper", state: "unverifiable",
        state_label: "Unverifiable — will not be touched", verified: false, name: "python3", age_ms: 7_200_000,
        rss_bytes: null, cpu_percent: null, ports: "", measured_at_ms: T0 - 1_000 },
    ],
    ...over,
  });

for (const theme of THEMES) {
  for (const vp of REMOTE_VP) {
    test(`48 remote cleanup — grouped, confirm, outcome — ${vp.name} — ${theme}`, async ({ page }) => {
      await openRemote(page, theme, vp, cleanupMachine());
      await goTo(page, "Cleanup");
      await page.getByLabel("Machine filter").selectOption("all");
      await expect(page.getByTestId("remote-cleanup")).toBeVisible();
      await expect(page.getByTestId("remote-cleanup-clean")).toHaveCount(1);
      await page.screenshot({ path: resolve(GALLERY, `48-remote-cleanup/grouped-${vp.name}-${theme}.png`) });
      await page.getByTestId("remote-cleanup-clean").click();
      await expect(page.getByTestId("remote-cleanup-confirm")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `48-remote-cleanup/confirm-${vp.name}-${theme}.png`) });
      await page.evaluate(() => {
        (window as unknown as Record<string, unknown>).__AIJ_REMOTE_CLEAN__ = {
          accepted: false, reason_code: "refused", message: "Refused — identity changed: pid 4242 is no longer the recorded process",
        };
      });
      await page.getByTestId("remote-cleanup-yes").click();
      await expect(page.getByTestId("remote-cleanup-outcome")).toContainText("Refused");
      await page.screenshot({ path: resolve(GALLERY, `48-remote-cleanup/identity-changed-${vp.name}-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E708 — 49-fleet-sync: the sync chip (Synced / N changes queued),
// an away machine's last-known rows on Fleet and on the board, and the
// clock-skew note — 1280×800, light/dark.
// ---------------------------------------------------------------------

const SYNC_VP = { width: 1280, height: 800 };
const awayMachine = () =>
  remoteMachine({
    connection: "offline",
    freshness: "offline",
    last_heard_ms: T0 - 3_600_000,
    synced_at_ms: T0 - 3_600_000,
    resources: null,
    alerts: [],
    processes: [],
    clock_skew_ms: 0,
    instances: (remoteMachine().instances as Array<Record<string, unknown>>).map((row) => ({
      ...row,
      cpu_percent: null,
      rss_bytes: null,
      process_count: null,
      measured_at_ms: T0 - 3_600_000,
    })),
  });

for (const theme of THEMES) {
  test(`49 fleet sync — synced, queued, last known, skew — ${theme}`, async ({ page }) => {
    await openFleet(page, theme, SYNC_VP, {
      __AIJ_FLEET_PEERS__: [{ ...remotePeer, queued_changes: 0 }],
      __AIJ_LINK__: { enabled: true, port: 47821, error: null },
      __AIJ_FLEET_REMOTE__: [remoteMachine({ clock_skew_ms: 0, synced_at_ms: T0 - 1_000, alerts: [], processes: [] })],
    });
    await expect(page.getByTestId("fleet-sync")).toHaveAttribute("data-state", "synced");
    await page.getByTestId("fleet-remote-instances").scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(GALLERY, `49-fleet-sync/synced-1280x800-${theme}.png`) });

    await page.evaluate((t0) => {
      const w = window as unknown as Record<string, Array<Record<string, unknown>>>;
      w.__AIJ_FLEET_PEERS__ = w.__AIJ_FLEET_PEERS__.map((peer) => ({
        ...peer, connection: "offline", protocol: null, last_seen_ms: t0 - 3_600_000, queued_changes: 12,
      }));
    }, T0);
    await page.evaluate((machine) => {
      (window as unknown as Record<string, unknown>).__AIJ_FLEET_REMOTE__ = [machine];
    }, awayMachine());
    await expect(page.getByTestId("fleet-sync")).toHaveText("⇡ 12 changes queued");
    await expect(page.getByTestId("fleet-last-known")).toBeVisible();
    await expect(page.getByTestId("fleet-remote-instance").first()).toContainText("was running");
    await page.getByTestId("fleet-remote-instances").scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(GALLERY, `49-fleet-sync/offline-queued-1280x800-${theme}.png`) });

    await page.evaluate((machine) => {
      (window as unknown as Record<string, unknown>).__AIJ_FLEET_REMOTE__ = [{ ...machine, clock_skew_ms: 12 * 60_000 }];
    }, awayMachine());
    await expect(page.getByTestId("fleet-clock-skew")).toContainText("12 min ahead");
    await page.getByTestId("fleet-remote-instances").scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(GALLERY, `49-fleet-sync/clock-skew-1280x800-${theme}.png`) });
  });

  test(`49 fleet sync — offline board — ${theme}`, async ({ page }) => {
    await openFleet(page, theme, SYNC_VP, {
      __AIJ_FLEET_PEERS__: [{ ...remotePeer, connection: "offline", protocol: null, queued_changes: 12 }],
      __AIJ_LINK__: { enabled: true, port: 47821, error: null },
      __AIJ_FLEET_REMOTE__: [awayMachine()],
    });
    await page.reload();
    await expectRail(page); // D-25: the list, or its icon column when narrow
    await openFilters(page);
    await page.getByLabel("Filter by machine").selectOption("machine-mini");
    await expect(page.getByTestId("board-machine")).toContainText("offline · last known");
    await expect(page.getByTestId("board-remote-row").first()).toContainText("was running");
    await page.screenshot({ path: resolve(GALLERY, `49-fleet-sync/offline-board-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E706 — 50-cross-switch: paired machines in the switch targets
// (T2 ready / T2 blocked → T3 / needs a project mapping), the wizard's
// target and path-mapping steps, the push progress, the switch-back
// preview and the divergence-on-return conflict sheet (AJ-E708) —
// 1280×800, light/dark.
// ---------------------------------------------------------------------

const crossTarget = (over: Record<string, unknown> = {}) => ({
  machine_id: "machine-studio",
  machine_label: "Studio Mac",
  recipe_id: "rec-desk",
  recipe_name: "Main",
  account_label: "Desktop Max",
  provider: "claude-code",
  mechanism: "transplant",
  readiness: "t2_ready",
  blockers: [],
  fidelity: "F3",
  badge: "F3 · Unvalidated",
  validated: false,
  carried: ["chat transcript", "rules & guidelines", "uncommitted git diff"],
  regenerated: [],
  dropped: ["todo list"],
  same_path_project: "proj-1",
  projects: [{ project_id: "proj-1", name: "ai-juggler", same_path: true, same_name: true }],
  replay_available: true,
  version_here: "2.1.0",
  version_there: "2.1.0",
  ...over,
});

const CROSS_TARGETS = [
  crossTarget(),
  crossTarget({
    recipe_id: "rec-personal",
    account_label: "Personal",
    readiness: "t2_blocked",
    mechanism: "replay",
    blockers: ["claude-code 2.1.0 here, 2.0.9 on Studio Mac — T2 needs the same version."],
    version_there: "2.0.9",
    fidelity: "F1",
    badge: "F1 · Unvalidated",
  }),
  crossTarget({
    machine_id: "machine-mini",
    machine_label: "Mac mini",
    recipe_id: "rec-mini",
    account_label: "Team",
    readiness: "needs_mapping",
    same_path_project: null,
    blockers: ["ai-juggler isn't at the same path on Mac mini — pick its copy there (you confirm the mapping) or use T3."],
    projects: [
      { project_id: "proj-m1", name: "ai-juggler", same_path: false, same_name: true },
      { project_id: "proj-m2", name: "storefront", same_path: false, same_name: false },
    ],
  }),
];

const CROSS_VP = { width: 1280, height: 800 };

const openCross = async (
  page: import("@playwright/test").Page,
  theme: string,
  seams: Record<string, unknown>,
) => {
  await page.addInitScript(
    ({ fixture, t, extra, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      Object.assign(w, extra);
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t: theme, extra: seams, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize(CROSS_VP);
  await page.goto("/");
  await expectRail(page); // D-25: the list, or its icon column when narrow
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await expect(page.getByTestId("handover-section")).toBeVisible();
  await openSwitch(page);
};

for (const theme of THEMES) {
  test(`50 cross switch — targets, wizard, mapping, progress — ${theme}`, async ({ page }) => {
    await openCross(page, theme, { __AIJ_FLEET_SWITCH_TARGETS__: CROSS_TARGETS });
    await expect(page.getByTestId("remote-switch-target")).toHaveCount(3);
    await page.getByTestId("remote-switch-targets").scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/targets-1280x800-${theme}.png`) });

    await page.getByTestId("remote-switch-target").nth(0).click();
    await expect(page.getByTestId("cross-switch-preconditions")).toContainText("2.1.0 here, 2.1.0 on Studio Mac");
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/wizard-t2-ready-1280x800-${theme}.png`) });
    await page.keyboard.press("Escape");

    await page.getByTestId("remote-switch-target").nth(1).click();
    await expect(page.getByTestId("cross-switch-t3")).toBeChecked();
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/wizard-t2-blocked-1280x800-${theme}.png`) });
    await page.keyboard.press("Escape");

    await page.getByTestId("remote-switch-target").nth(2).click();
    await page.getByTestId("cross-switch-next").click();
    await page.getByTestId("cross-switch-project-option").first().check();
    await page.getByTestId("cross-switch-confirm-mapping").check();
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/path-mapping-1280x800-${theme}.png`) });
    await page.getByTestId("cross-switch-next").click();
    await expect(page.getByTestId("cross-switch-drops")).toContainText("todo list");
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/carried-1280x800-${theme}.png`) });
    await page.getByTestId("cross-switch-next").click();
    await expect(page.getByTestId("cross-switch-go")).toHaveText("Switch to Mac mini…");
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/confirm-1280x800-${theme}.png`) });
  });

  test(`50 cross switch — switch-back preview and the conflict sheet — ${theme}`, async ({ page }) => {
    await openCross(page, theme, {
      __AIJ_FLEET_SWITCH_BACK_PLAN__: {
        original_machine_id: "machine-laptop",
        original_machine_label: "Laptop",
        original_account_label: "Work Max",
        away_account_label: "Desktop Max",
        new_turns: 4,
        delta_preview:
          "While you were away on Desktop Max — 4 new turns (12 shared before the switch).\n\n- [user] add the payment form\n- [assistant] payment form added with validation\n- [user] wire it to checkout\n- [assistant] checkout now posts the payment\n\nFull detail: the attached switch pack (not replayed).",
        blockers: [],
      },
      __AIJ_FLEET_SWITCH_BACK_OUTCOMES__: [
        {
          accepted: false,
          reason_code: "diverged",
          message: "The original moved on here since the work left (2 new turns) — choose what to keep.",
          instance_id: null,
          edge_id: "edge-back",
          divergence: { turns_since: 2, last_activity_ms: T0 - 1_800_000, summary: "2 new turns" },
        },
      ],
    });
    await expect(page.getByTestId("cross-back-card")).toBeVisible();
    await page.getByTestId("cross-back-card").click();
    await expect(page.getByTestId("cross-back-delta")).toContainText("While you were away");
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/switch-back-preview-1280x800-${theme}.png`) });
    await page.getByTestId("cross-back-go").click();
    await expect(page.getByTestId("conflict-choices")).toBeVisible();
    await expect(page.getByTestId("conflict-later")).toBeFocused();
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/conflict-sheet-1280x800-${theme}.png`) });
    await page.getByTestId("conflict-keep_both").check();
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/conflict-keep-both-1280x800-${theme}.png`) });
    await page.getByTestId("conflict-apply").click();
    await expect(page.getByTestId("cross-back-done")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `50-cross-switch/resolved-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E1001 — 51-identity: the identity marker on Instance rows and the
// inspector chip, and the wrong-account pause (expected vs detected,
// switch / check again / cancel) — 1280×800 and 1024×680, light/dark.
// ---------------------------------------------------------------------

const IDENTITY_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x680", width: 1024, height: 680 },
] as const;

const identityFixture = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session, index) => ({
    ...session,
    identity_state: ["verified", "first_seen", "unverified", "signed_out"][index % 4],
    identity_label: session.account_label === "Personal" ? "o…@home.example" : "o…@work.example · Acme",
  })),
};

for (const theme of THEMES) {
  for (const vp of IDENTITY_VP) {
    test(`51 identity — rail marker, inspector chip, wrong-account pause — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t, t0 }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_IDENTITY_CHECK__ = {
            "p-1": {
              state: "mismatch", provider: "claude-code", account_id: "claude-work", account_label: "Work Max",
              model: "opus", expected: "o…@work.example · Acme", detected: "o…@home.example",
              reason: null, alternatives: [{ recipe_id: "p-2", recipe_name: "Main", account_label: "Personal" }],
            },
          };
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: identityFixture, t: theme, t0: T0 },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await expect(page.getByTestId("rail-identity").first()).toBeVisible();
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await expect(page.getByTestId("instance-identity")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `51-identity/rail-and-chip-${vp.name}-${theme}.png`) });
      await page.getByRole("tab", { name: "Actions" }).click();
      await page.getByRole("button", { name: /Resume|Relaunch/ }).first().click();
      await expect(page.getByTestId("identity-mismatch")).toBeVisible();
      await expect(page.getByTestId("identity-detected")).toHaveText("o…@home.example");
      await page.screenshot({ path: resolve(GALLERY, `51-identity/wrong-account-${vp.name}-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E1002 — 52-worktree: branch / uncommitted / ahead chips on the rail
// and the inspector's git line, the worktree offer for a second launch,
// and the dirty-removal dialog — 1280×800 both themes; the removal
// dialog also at 1024×680.
// ---------------------------------------------------------------------

const worktreeFixture = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session, index) => ({
    ...session,
    git_branch: index % 2 === 0 ? `juggler/${(index * 4111).toString(16).slice(0, 6)}` : "main",
    git_dirty_count: index % 3 === 0 ? 3 : 0,
    git_ahead: index % 2 === 0 ? 2 : 0,
    git_behind: 0,
    git_worktree: index % 2 === 0 ? true : undefined,
    cwd: "/Users/dev/ai-juggler",
  })),
};

const gitStatusFor = Object.fromEntries(
  allStatuses.sessions.map((session) => [
    session.id,
    {
      branch: "juggler/a1b2c3", changed: 2, untracked: 1, ahead: 2, behind: 0, worktree: true,
      created_by_juggler: true, path: "/Users/dev/ai-juggler.juggler/a1b2c3",
    },
  ]),
);

for (const theme of THEMES) {
  for (const vp of [
    { name: "1280x800", width: 1280, height: 800 },
    { name: "1024x680", width: 1024, height: 680 },
  ] as const) {
    test(`52 worktree — chips, offer, dirty removal — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t, t0, git }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_GIT_STATUS__ = git;
          w.__AIJ_ISOLATION_OFFER__ = {
            project_name: "ai-juggler", in_use_by: ["Personal"], available: true, unavailable_reason: null,
            proposed_path: "/Users/dev/ai-juggler.juggler/a1b2c3", proposed_branch: "juggler/a1b2c3", base_branch: "main",
          };
          w.__AIJ_WORKTREE_REMOVAL__ = {
            path: "/Users/dev/ai-juggler.juggler/a1b2c3", folder: "a1b2c3", branch: "juggler/a1b2c3",
            changed: 2, untracked: 1, needs_confirmation: true, in_use: false,
            summary: "Discards 2 changed and 1 untracked files — they are not committed anywhere. The branch juggler/a1b2c3 stays with its commits.",
          };
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: worktreeFixture, t: theme, t0: T0, git: gitStatusFor },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expectRail(page); // D-25: the list, or its icon column when narrow
      await expect(page.getByTestId("git-badge").first()).toBeVisible();
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await expect(page.getByTestId("instance-git")).toContainText("3 uncommitted files");
      if (vp.name === "1280x800") {
        await page.screenshot({ path: resolve(GALLERY, `52-worktree/chips-1280x800-${theme}.png`) });
      }
      await page.getByTestId("worktree-remove").click();
      await expect(page.getByTestId("worktree-remove-summary")).toContainText("Discards 2 changed");
      await page.screenshot({ path: resolve(GALLERY, `52-worktree/dirty-removal-${vp.name}-${theme}.png`) });
      await page.keyboard.press("Escape");
      if (vp.name === "1280x800") {
        await page.getByRole("tab", { name: "Actions" }).click();
        await page.getByRole("button", { name: /Resume|Relaunch/ }).first().click();
        await expect(page.getByTestId("isolation-offer")).toBeVisible();
        await page.screenshot({ path: resolve(GALLERY, `52-worktree/offer-1280x800-${theme}.png`) });
      }
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E1003 — 53-parked: a parked row with its countdown as text, the
// inspector's park card, and the one reset offer (Resume / Remind me in
// 30 min) — 1280×800, light/dark.
// ---------------------------------------------------------------------

const parkedFixture = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) =>
    session.status === "quota_blocked"
      ? { ...session, parked: "measured", parked_resets_at_ms: T0 + 2 * 3_600_000 + 13 * 60_000 }
      : session,
  ),
};

for (const theme of THEMES) {
  test(`53 parked — row, card and the reset offer — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t, t0 }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_PARKED_OFFERS__ = [
          {
            session_id: "sess-other", account_label: "Personal",
            text: "Personal has reset. Resume the work you parked — the same conversation, on the same Account.", last: false,
          },
        ];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: parkedFixture, t: theme, t0: T0 },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expectRail(page); // D-25: the list, or its icon column when narrow
    await expect(page.getByTestId("rail-parked").first()).toContainText("Parked 2h 13m");
    await expect(page.getByTestId("parked-offer")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `53-parked/rail-and-offer-1280x800-${theme}.png`) });
    await page.getByTestId("rail-parked").first().click();
    await expect(page.getByTestId("park-state")).toContainText("Parked until the quota resets");
    await page.screenshot({ path: resolve(GALLERY, `53-parked/inspector-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E1004 — 54-summary: the morning summary on the home view —
// severity-ordered groups with words and an icon, a row opening its
// Instance, the rail's needs-you badge alongside — 1440×900 and
// 1024×680, light/dark.
// ---------------------------------------------------------------------

const summaryRow = (id: string, label: string, group: string, status: string, words: string, over: Record<string, unknown> = {}) => ({
  session_id: id, account_label: label, provider: "claude-code", project: "ai-juggler", group, status, words,
  transitions: [
    { from: null, to: "starting", at_ms: T0 - 8 * 3_600_000 },
    { from: "starting", to: "running", at_ms: T0 - 8 * 3_600_000 + 4_000 },
    { from: "running", to: status, at_ms: T0 - 2 * 3_600_000 },
  ],
  tokens: null, leftovers: 0, ...over,
});

const morningSummary = {
  since_ms: T0 - 9 * 3_600_000,
  generated_ms: T0,
  rows: [
    summaryRow("sess-8", "Work Max", "crashed", "failed", "Crashed", { leftovers: 2 }),
    summaryRow("sess-3", "Personal", "needs_you", "waiting_user", "Needs you — waiting for an answer"),
    summaryRow("sess-4", "Work Max", "needs_you", "stuck", "Needs you — no output for a while"),
    summaryRow("sess-5", "Personal", "needs_you", "quota_blocked", "Needs you — quota reached"),
    summaryRow("sess-2", "Work Max", "running", "running", "Still running", { tokens: 19_134 }),
    summaryRow("sess-6", "Work Max", "finished", "finished", "Finished"),
    summaryRow("sess-7", "Personal", "ended", "stopped", "Ended"),
  ],
  counts: { crashed: 1, needs_you: 3, running: 1, finished: 1, ended: 1 },
  leftovers: 2,
};

for (const theme of THEMES) {
  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x680", width: 1024, height: 680 },
  ] as const) {
    test(`54 summary — home view and a row into its Instance — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t, t0, summary }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_SUMMARY__ = summary;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t: theme, t0: T0, summary: morningSummary },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expect(page.getByTestId("summary-headline")).toContainText("1 crashed · 3 needs you");
      await page.screenshot({ path: resolve(GALLERY, `54-summary/home-${vp.name}-${theme}.png`) });
      await page.getByTestId("summary-row").first().click();
      await expect(page.getByRole("tab", { name: "Quality / Performance" })).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `54-summary/opened-${vp.name}-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E1005 / AJ-E1021 — 55-triage: the morning triage, now on Home (D-25
// amendment): needs you, parked, running, finished — not reviewed — and
// empty, at 1440×900 and 1024×680, light/dark, comfortable and compact.
// ---------------------------------------------------------------------

for (const theme of THEMES) {
  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x680", width: 1024, height: 680 },
  ] as const) {
    for (const density of ["comfortable", "compact"] as const) {
      test(`55 triage — populated — ${vp.name} — ${density} — ${theme}`, async ({ page }) => {
        await page.addInitScript(
          ({ fixture, t, t0, d }) => {
            const w = window as unknown as Record<string, unknown>;
            w.__AIJ_FIXTURE__ = fixture;
            localStorage.setItem("aij.theme", t);
            localStorage.setItem("aij.density", d);
            localStorage.setItem("aij.onboarding.dismissed", "1");
            sessionStorage.setItem("aij.ready.skipped", "1");
            Date.now = () => t0;
          },
          { fixture: parkedFixture, t: theme, t0: T0, d: density },
        );
        await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.goto("/");
        await expect(page.getByTestId("home-parked")).toContainText("2h 13m");
        await expect(page.getByTestId("home-needs").getByRole("button").first()).toBeVisible();
        await page.screenshot({ path: resolve(GALLERY, `55-triage/home-populated-${vp.name}-${density}-${theme}.png`) });
      });
    }
    test(`55 triage — empty — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t, t0 }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: { ...allStatuses, sessions: [] }, t: theme, t0: T0 },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expect(page.getByTestId("home")).toContainText("Ready when you are");
      await page.screenshot({ path: resolve(GALLERY, `55-triage/home-empty-${vp.name}-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E1007 — 56-history: search results with a transcript snippet, the
// Archived list, the read-only replay, the delete dialog's exact
// consequence copy, and Settings › Storage & retention with its dry run —
// 1280×800 and 1024×680, light/dark.
// ---------------------------------------------------------------------

const historyFixture = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) =>
    session.id === "sess-7" ? { ...session, lifecycle: "archived", lifecycle_at_ms: T0 - 3_600_000 } : session,
  ),
};

const historySeeds = {
  hits: [{ session_id: "sess-6", snippet: "…we traced the flaky Stripe webhook retry to clock skew on the queue worker…" }],
  replay: {
    session_id: "sess-6",
    transcript_path: "/Users/dev/.claude/projects/storefront/sess-6.jsonl",
    transcript: "",
    truncated: false,
    bytes: 48_213,
    sha256: "9f2c1e7a4b3d8e6f0a1b2c3d4e5f60718293a4b5c6d7e8f9",
    entries: [
      { role: "user", text: "The Stripe webhook retries twice on every event. Find out why." },
      { role: "assistant", text: "The queue worker's clock is 40 s behind, so the signature looks expired and the first delivery is rejected. Fixing the NTP config on the worker." },
      { role: "user", text: "Ship it and add a test." },
      { role: "assistant", text: "Done: worker clock synced, and a test that rejects a signature older than 5 minutes." },
    ],
    history: [
      { kind: "status", what: "starting", at_ms: T0 - 6 * 3_600_000 },
      { kind: "status", what: "running", at_ms: T0 - 6 * 3_600_000 + 4_000 },
      { kind: "status", what: "finished", at_ms: T0 - 5 * 3_600_000 },
      { kind: "lifecycle", what: "reviewed", at_ms: T0 - 4 * 3_600_000 },
    ],
    lineage: [{ direction: "in", other_account: "Personal", other_provider: "claude-code", state: "confirmed", fidelity: "F1" }],
    processes: [
      { pid: 48211, role: "root", state: "exited", exit_kind: "clean", first_seen_ms: T0 - 6 * 3_600_000, last_seen_ms: T0 - 5 * 3_600_000 },
      { pid: 48230, role: "helper", state: "exited", exit_kind: "reaped", first_seen_ms: T0 - 6 * 3_600_000, last_seen_ms: T0 - 5 * 3_600_000 },
    ],
  },
  storage: [
    { account_id: "claude-work", label: "Work Max", retention_days: 90, instances: 42, history_events: 318, index_lines: 18_240, transcript_bytes: 212 * 1024 * 1024 },
    { account_id: "claude-personal", label: "Personal", retention_days: null, instances: 17, history_events: 96, index_lines: 5_120, transcript_bytes: 64 * 1024 * 1024 },
  ],
  dryRun: {
    account_id: "claude-work", retention_days: 90, token: "dry",
    deletes: [
      { session_id: "sess-2026-06-02-a1", project: "storefront", last_active_ms: T0 - 130 * 86_400_000, kept_because: null },
      { session_id: "sess-2026-06-19-c4", project: "ai-juggler", last_active_ms: T0 - 112 * 86_400_000, kept_because: null },
    ],
    kept: [{ session_id: "sess-2026-05-30-b7", project: "storefront", last_active_ms: T0 - 140 * 86_400_000, kept_because: "It is part of a switch history — deleting it would break that lineage. Archive it instead." }],
    keeps: ["The Provider's own transcript files", "The process and cleanup audit", "Usage in your usage totals"],
  },
};

for (const theme of THEMES) {
  for (const vp of [
    { name: "1280x800", width: 1280, height: 800 },
    { name: "1024x680", width: 1024, height: 680 },
  ] as const) {
    test(`56 history — search, archived, replay, delete, retention — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t, t0, seeds }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_TRANSCRIPT_HITS__ = seeds.hits;
          w.__AIJ_REPLAY__ = seeds.replay;
          w.__AIJ_ACCOUNT_STORAGE__ = seeds.storage;
          w.__AIJ_RETENTION_PREVIEW__ = seeds.dryRun;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: historyFixture, t: theme, t0: T0, seeds: historySeeds },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await page.getByRole("button", { name: "History" }).click();
      await page.getByTestId("history-search").fill("stripe");
      await expect(page.getByTestId("history-snippet")).toContainText("Stripe webhook");
      await page.screenshot({ path: resolve(GALLERY, `56-history/search-${vp.name}-${theme}.png`) });
      await page.getByTestId("history-search").fill("");
      await page.getByTestId("history-tab-archived").click();
      await expect(page.getByTestId("history-row")).toHaveCount(1);
      await page.screenshot({ path: resolve(GALLERY, `56-history/archived-${vp.name}-${theme}.png`) });
      await page.getByTestId("history-replay").first().click();
      await expect(page.getByTestId("replay-process-list")).toContainText("root 48211");
      await page.screenshot({ path: resolve(GALLERY, `56-history/replay-${vp.name}-${theme}.png`) });
      await page.getByTestId("replay-close").click();
      await page.getByTestId("history-tab-ended").click();
      await page.getByTestId("history-delete").first().click();
      await expect(page.getByTestId("delete-keeps")).toContainText("transcript file");
      await page.screenshot({ path: resolve(GALLERY, `56-history/delete-${vp.name}-${theme}.png`) });
      await page.keyboard.press("Escape");
      await page.getByRole("button", { name: "Settings" }).click();
      await page.getByTestId("settings-storage").scrollIntoViewIfNeeded();
      await page.screenshot({ path: resolve(GALLERY, `56-history/retention-${vp.name}-${theme}.png`) });
      await page.getByTestId("storage-review").first().click();
      await expect(page.getByTestId("dry-run-deletes")).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `56-history/dry-run-${vp.name}-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E1009 — 57-backup: Settings › Backup & restore — export in
// progress, export done (what was written and withheld), the verified
// restore plan with the other-Mac identity warning, and a refused
// (damaged) backup naming the file and reason — 1280×800, light/dark.
// ---------------------------------------------------------------------

const backupManifest = {
  format: "ai-juggler-backup", format_version: 1, schema_version: 27, app_version: "0.0.1",
  created_at_ms: T0 - 2 * 86_400_000,
  origin: { fingerprint: "studio", label: "Studio Mac" },
  counts: { accounts: 3, launch_settings: 4, projects: 6, instances: 128, history_events: 912, switch_edges: 14, epochs: 22 },
  files: [], excluded: [],
  withheld: ["Environment values of the Launch settings “Main” (Work Max): ANTHROPIC_API_KEY"],
};

for (const theme of THEMES) {
  for (const state of ["progress", "done", "plan", "refused"] as const) {
    test(`57 backup — ${state} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t, t0, kind, manifest }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_BACKUP_DEST__ = "/Users/dev/Backups/AI Juggler backup 2026-10-18.ajbackup";
          w.__AIJ_BACKUP_FILE__ = "/Users/dev/Backups/AI Juggler backup 2026-10-16.ajbackup";
          if (kind === "progress") {
            w.__AIJ_BACKUP_PROGRESS__ = [[0, 1000], [420, 1000]];
            w.__AIJ_BACKUP_HOLD__ = true;
          }
          w.__AIJ_BACKUP_EXPORT__ = { path: "", bytes: 18_400_000, manifest };
          w.__AIJ_RESTORE_PLAN__ = {
            path: "", manifest, other_machine: true, token: "t",
            notes: [
              "This backup comes from Studio Mac. It carries that Mac's data, never its identity: this Mac keeps its own and must pair again with your other Macs.",
              "Restoring replaces AI Juggler's data on this Mac. What is here now is kept beside it, so it can be put back by hand.",
              "Sign in to each Account again afterwards — sign-ins are never in a backup.",
              "Re-enter what was withheld: Environment values of the Launch settings “Main” (Work Max): ANTHROPIC_API_KEY.",
            ],
          };
          if (kind === "refused") {
            w.__AIJ_RESTORE_PLAN_ERROR__ =
              "invalid restore: /Users/dev/Backups/AI Juggler backup 2026-10-16.ajbackup can't be restored: db/juggler.sqlite does not match its checksum — the file is damaged";
          }
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        { fixture: allStatuses, t: theme, t0: T0, kind: state, manifest: backupManifest },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.goto("/");
      await page.getByRole("button", { name: "Settings" }).click();
      await page.getByTestId("settings-backup").scrollIntoViewIfNeeded();
      if (state === "progress" || state === "done") {
        await page.getByTestId("backup-export").click();
        await expect(page.getByTestId(state === "progress" ? "backup-status" : "backup-done")).toContainText(
          state === "progress" ? "42%" : "Backed up",
        );
      } else {
        await page.getByTestId("backup-restore").click();
        await expect(page.getByTestId(state === "plan" ? "restore-other-machine" : "backup-failed")).toBeVisible();
      }
      await page.screenshot({ path: resolve(GALLERY, `57-backup/${state}-1280x800-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E1010 — 58-updates: Settings › About & updates — update available
// (version + changes before installing, the Beta label), downloading,
// failed (plain words, Details, Try again; the version untouched), a
// failed check, installed with the restart held back (and why), ready to
// restart, up to date, and a development build — and on Home, the
// launch check's notice and the "Restart to update" prompt, each with
// its Beta label. 1280×800, light/dark. The card is scrolled into view.
// ---------------------------------------------------------------------

const updateBase = {
  current_version: "0.1.0", channel: "beta", edition: "main", app_name: "AI Juggler", channels: ["stable", "beta"],
  configured: true, check_on_launch: true,
  available: null, progress: null, error: null, error_kind: null, error_detail: null, restart_wait: null,
  backup_path: null, checked_at_ms: T0 - 60_000,
};
const updateAvailable = {
  version: "0.2.0",
  date_ms: T0 - 86_400_000,
  notes: "Morning triage, History and backups.\n\n## New\n- **Triage**: one sorted view of what needs you, across every project.\n- **History**: search any past session, replay it read-only, archive it.\n- **Backup & restore**: one file to move AI Juggler to a new Mac.\n\n## Fixed\n- The home view no longer hides its top under the header on small windows.",
};
const updateBackup = "/Users/dev/Library/Application Support/AI Juggler/backups/before-update-0.1.0-to-0.2.0.ajbackup";
const updateStates: Record<string, Record<string, unknown>> = {
  available: { ...updateBase, state: "available", available: updateAvailable },
  downloading: { ...updateBase, state: "downloading", available: updateAvailable, progress: 64, backup_path: updateBackup },
  failed: {
    ...updateBase, state: "failed", available: updateAvailable, backup_path: updateBackup,
    error: "The download didn't pass the signature check — it was damaged on the way or isn't from AI Juggler — so it wasn't installed. AI Juggler 0.1.0 is untouched.",
    error_kind: "signature", error_detail: "the signature verification failed: Invalid signature",
  },
  "check-failed": {
    ...updateBase, state: "failed",
    error: "Couldn't reach the update server. Check your internet connection, then try again.",
    error_kind: "network", error_detail: "error sending request for url (https://github.com/figurehead-ai/AI-juggler/releases/download/beta-channel/latest.json)",
  },
  "restart-waiting": {
    ...updateBase, state: "installed", available: updateAvailable, progress: 100, backup_path: updateBackup,
    restart_wait: "A switch is in progress. Restart to update is offered once it finishes.",
  },
  "restart-ready": { ...updateBase, state: "installed", available: updateAvailable, progress: 100, backup_path: updateBackup },
  "up-to-date": { ...updateBase, state: "up_to_date" },
  // Go back to an earlier version: the copy kept before the upgrade, the
  // choice asked once, inline.
  "going-back-confirm": { ...updateBase, state: "up_to_date" },
  development: { ...updateBase, configured: false, state: "idle" },
  // D-23: the editions installed next to AI Juggler.
  "nightly-edition": {
    ...updateBase, edition: "nightly", app_name: "AI Juggler Nightly", channel: "nightly", channels: ["nightly"],
    state: "available", available: updateAvailable,
  },
  "dev-edition": {
    ...updateBase, edition: "dev", app_name: "AI Juggler Dev", channel: "nightly", channels: [], configured: false,
    state: "idle",
  },
};
const editionAppInfo: Record<string, Record<string, string>> = {
  "nightly-edition": { version: "0.1.0", name: "AI Juggler Nightly", edition: "nightly" },
  "dev-edition": { version: "0.1.0", name: "AI Juggler Dev", edition: "dev" },
};
const updateNothingLive = {
  ...allStatuses,
  sessions: allStatuses.sessions.filter((session) =>
    ["finished", "stopped", "failed", "handed_over"].includes(String(session.status)),
  ),
};
/** The prompts on Home: the launch check found a version; one installed
 * and it is safe to restart. */
const updateHomeStates: Record<string, Record<string, unknown>> = {
  "home-available": updateStates.available,
  "home-restart-ready": updateStates["restart-ready"],
};

for (const theme of THEMES) {
  for (const [name, status] of Object.entries({ ...updateStates, ...updateHomeStates })) {
    test(`58 updates — ${name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, t, t0, seeded, info, goingBack }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_UPDATE_STATUS__ = seeded;
          if (info) w.__AIJ_APP_INFO__ = info;
          if (goingBack) w.__AIJ_GOING_BACK__ = goingBack;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        {
          // Restart is only offered with no session live, as the backend
          // decides: those captures have only ended sessions.
          fixture: name.endsWith("restart-ready") ? updateNothingLive : allStatuses,
          t: theme, t0: T0, seeded: status, info: editionAppInfo[name] ?? null,
          goingBack: name.startsWith("going-back") ? [{ schema: 27, version: "0.0.7", kept_at_ms: T0 - 3 * 86_400_000 }] : null,
        },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.goto("/");
      if (name.startsWith("home-")) {
        const prompt = page.getByTestId(name === "home-available" ? "update-notice" : "update-ready");
        await expect(prompt).toBeVisible();
        await expect(prompt.getByText("Beta", { exact: true })).toBeVisible();
      } else {
        await page.getByRole("button", { name: "Settings" }).click();
        await expect(page.getByTestId("beta-label")).toBeVisible();
        await page.getByTestId("settings-updates").evaluate((card) => card.scrollIntoView({ block: "start" }));
        if (name === "going-back-confirm") {
          await page.getByText("Go back to an earlier version", { exact: true }).click();
          await page.getByTestId("going-back-27").click();
          await expect(page.getByTestId("going-back-confirm")).toBeVisible();
        }
      }
      await page.screenshot({ path: resolve(GALLERY, `58-updates/${name}-1280x800-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E1204 — 59-vault: the recovery-code sheet the first time Cloud
// backup is on (code as selectable text, Copy and Save, the warning,
// confirm required), and the waiting row after "Later" — 1280×800,
// light/dark.
// ---------------------------------------------------------------------

for (const theme of THEMES) {
  test(`59 vault — recovery code sheet and waiting row — ${theme}`, async ({ page }) => {
    await openSettings(page, theme, { width: 1280, height: 800 }, {
      __AIJ_FIGUREHEAD__: FH_LINKED,
      __AIJ_VAULT_STATUS__: { has_key: false, code_saved: false },
      __AIJ_VAULT_CODE__: "MZXW-6YTB-OI3D-EZLM-NFXG-K4TU-OJSW-Y5DF-MFZG-K3TF-ON2G-K3DF-NZTW-Q2AB",
    });
    await expect(page.getByTestId("recovery-sheet")).toBeVisible();
    await expect(page.getByTestId("recovery-code")).toContainText("MZXW-6YTB");
    await page.screenshot({ path: resolve(GALLERY, `59-vault/recovery-code-1280x800-${theme}.png`) });
    await page.getByTestId("recovery-confirm").check();
    await page.screenshot({ path: resolve(GALLERY, `59-vault/confirmed-1280x800-${theme}.png`) });
    await page.getByRole("button", { name: /Later/ }).click();
    await page.getByTestId("vault-pending").scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(GALLERY, `59-vault/waiting-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E1205 — 60-cloud-backup: the Cloud backup panel (last backup,
// schedule), an upload in progress, the backup list on a new Mac with
// the recovery-code field, and the restore preview — the same dialog as
// a local restore — 1280×800, light/dark.
// ---------------------------------------------------------------------

const cloudList = [
  { id: "bak_3", label: "AI Juggler · Studio Mac", size_bytes: 418_000, created_at: "2026-10-18T09:00:00Z", this_mac: false },
  { id: "bak_2", label: "AI Juggler · Studio Mac", size_bytes: 402_000, created_at: "2026-10-11T09:00:00Z", this_mac: false },
  { id: "bak_1", label: "AI Juggler · MacBook Air", size_bytes: 388_000, created_at: "2026-10-04T09:00:00Z", this_mac: false },
];
const cloudPlan = {
  path: "/data/from-cloud.ajbackup", token: "t", other_machine: true,
  notes: [
    "This backup comes from Studio Mac. It carries that Mac's data, never its identity: this Mac keeps its own and must pair again with your other Macs.",
    "Restoring replaces AI Juggler's data on this Mac. What is here now is kept beside it, so it can be put back by hand.",
    "Sign in to each Account again afterwards — sign-ins are never in a backup.",
  ],
  manifest: {
    format: "ai-juggler-backup", format_version: 1, schema_version: 27, app_version: "0.1.0",
    created_at_ms: T0 - 86_400_000, origin: { fingerprint: "studio", label: "Studio Mac" },
    counts: { accounts: 3, launch_settings: 4, projects: 6, instances: 128, history_events: 912, switch_edges: 14, epochs: 22 },
    files: [], withheld: [], excluded: [],
  },
};

for (const theme of THEMES) {
  test(`60 cloud backup — panel, uploading, list, restore preview — ${theme}`, async ({ page }) => {
    await openSettings(page, theme, { width: 1280, height: 800 }, {
      __AIJ_FIGUREHEAD__: FH_LINKED,
      __AIJ_CLOUD_BACKUP__: {
        enabled: true, code_saved: true, schedule: "weekly", uploading: false, limit_bytes: 25 * 1024 * 1024,
        last: { at_ms: T0 - 2 * 3_600_000, ok: true, size_bytes: 418_000, error: null },
      },
      __AIJ_CLOUD_BACKUPS__: cloudList,
      __AIJ_CLOUD_RESTORE_PLAN__: cloudPlan,
      __AIJ_CLOUD_BACKUP_HOLD__: true,
    });
    const panel = page.getByTestId("cloud-backup-panel");
    await panel.scrollIntoViewIfNeeded();
    await expect(page.getByTestId("cloud-backup-last")).toContainText("Last backup");
    await page.screenshot({ path: resolve(GALLERY, `60-cloud-backup/panel-1280x800-${theme}.png`) });
    await page.getByTestId("cloud-backup-now").click();
    await expect(page.getByTestId("cloud-backup-last")).toContainText("Backing up");
    await page.screenshot({ path: resolve(GALLERY, `60-cloud-backup/uploading-1280x800-${theme}.png`) });
  });

  test(`60 cloud backup — restore on a new Mac — ${theme}`, async ({ page }) => {
    await openSettings(page, theme, { width: 1280, height: 800 }, {
      __AIJ_FIGUREHEAD__: FH_LINKED,
      __AIJ_VAULT_STATUS__: { has_key: false, code_saved: true },
      __AIJ_CLOUD_BACKUP__: { enabled: true, code_saved: true, schedule: "weekly", uploading: false, limit_bytes: 25 * 1024 * 1024, last: null },
      __AIJ_CLOUD_BACKUPS__: cloudList,
      __AIJ_CLOUD_RESTORE_PLAN__: cloudPlan,
    });
    await page.getByTestId("cloud-backup-panel").scrollIntoViewIfNeeded();
    await page.getByTestId("cloud-backup-restore").click();
    await page.getByTestId("cloud-backup-code").fill("MZXW-6YTB-OI3D-EZLM-NFXG-K4TU-OJSW-Y5DF-MFZG-K3TF-ON2G-K3DF-NZTW-Q2AB");
    await page.getByTestId("cloud-backup-list").scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(GALLERY, `60-cloud-backup/list-1280x800-${theme}.png`) });
    await page.getByTestId("cloud-backup-pick").first().click();
    await expect(page.getByTestId("restore-plan")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `60-cloud-backup/restore-preview-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E1206 — 61-account-macs: Fleet with your Macs found through your
// Figurehead account (states in words), the fingerprint comparison
// before pairing, and an account-paired peer labelled as such — 1280×800,
// light/dark.
// ---------------------------------------------------------------------

const ACCOUNT_MACS = {
  this_fingerprint: "3F2A 9C01 B7E4 55D0 12AF 8C3E 0D91 66B2",
  read_at_ms: T0,
  macs: [
    { device_id: "dev_mini", name: "Mac mini", platform: "macos", fingerprint: "A71C 0E9B 44D2 F3A8 6B15 9C7E 2D40 E118", state: "not_paired", last_seen_at: null },
    { device_id: "dev_air", name: "MacBook Air", platform: "macos", fingerprint: "5D3E 81AF C2B9 07E6 4F18 A9C3 6E2D 0B57", state: "waiting", last_seen_at: null },
    { device_id: "dev_old", name: "Old laptop", platform: "macos", fingerprint: null, state: "no_key", last_seen_at: null },
  ],
};
const FH_DISCOVERY = { ...FH_LINKED, features: FH_LINKED.features.map((f) => (f.key === "fleetDiscovery" ? { ...f, on: true } : f)) };

for (const theme of THEMES) {
  test(`61 account macs — list, compare, account-paired peer — ${theme}`, async ({ page }) => {
    await openFleet(page, theme, { name: "1280x800", width: 1280, height: 800 }, {
      __AIJ_FIGUREHEAD__: FH_DISCOVERY,
      __AIJ_ACCOUNT_MACS__: ACCOUNT_MACS,
      __AIJ_FLEET_PEERS__: [{ ...FLEET_PEER, machine_id: "studio-real", label: "Studio", permission: "view", permission_label: "View only", paired_via: "account" }],
    });
    await expect(page.getByTestId("account-mac")).toHaveCount(3);
    await page.screenshot({ path: resolve(GALLERY, `61-account-macs/list-1280x800-${theme}.png`), fullPage: true });
    await page.getByTestId("account-mac-compare").click();
    await expect(page.getByTestId("compare-other")).toHaveText("A71C 0E9B 44D2 F3A8 6B15 9C7E 2D40 E118");
    await page.getByTestId("compare-match").check();
    await page.screenshot({ path: resolve(GALLERY, `61-account-macs/compare-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E1105 — 62-privacy: Settings › Privacy in every state of spec §8
// (off, local, share, overridden, not included, an upload error, and a
// report open in the read-only viewer); 63-consent: the consent-version
// re-ask — 1280×800 and 1024×680, light/dark.
// ---------------------------------------------------------------------

const STATS_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x680", width: 1024, height: 680 },
];

const statsInfo = (patch: Record<string, unknown> = {}) => ({
  included: true,
  chosen: "off",
  effective: "off",
  can_upload: false,
  reason: null,
  decided_at_ms: null,
  consent_version: 0,
  current_consent_version: 1,
  ...patch,
});

const STATS_ROWS = [
  { category: "environment", metric: "accounts.configured", dims: "provider: claude-code", value: "2" },
  { category: "environment", metric: "projects.count", dims: "", value: "4-5" },
  { category: "environment", metric: "ui.theme", dims: "", value: "system" },
  { category: "features", metric: "app.launch", dims: "", value: "6" },
  { category: "features", metric: "instance.started", dims: "kind: terminal · origin: new · provider: claude-code", value: "14" },
  { category: "features", metric: "switch.completed", dims: "from: claude-code · kind: account · mechanism: transplant · outcome: success · to: claude-code · trigger: suggestion", value: "3" },
  { category: "features", metric: "ui.screen_opened", dims: "screen: cleanup", value: "2" },
  { category: "reliability", metric: "perf.cold_start_ms", dims: "bucket: 750-1500", value: "6" },
];

const statsOverview = (info: Record<string, unknown>, patch: Record<string, unknown> = {}) => ({
  info: statsInfo(info),
  week: "2026-W42",
  rows: [],
  reports: [],
  next_upload_ms: null,
  last_error: null,
  docs_url: "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/usage-stats.md",
  sync: { offered: false, on: false, email: null, notice: null },
  ...patch,
});

const LOCAL = statsOverview(
  { chosen: "local", effective: "local", decided_at_ms: T0 - 30 * 86_400_000, consent_version: 1 },
  {
    rows: STATS_ROWS,
    reports: [
      { week: "2026-W41", status: "skipped", reason: "Counted while stats stayed on this Mac — never sent", upload_at_ms: null, next_try_ms: null, attempts: 0, bytes: 1_204, synced: false },
      { week: "2026-W40", status: "skipped", reason: "Counted while stats stayed on this Mac — never sent", upload_at_ms: null, next_try_ms: null, attempts: 0, bytes: 988, synced: false },
    ],
  },
);

const SHARE = statsOverview(
  { chosen: "share", effective: "share", can_upload: true, decided_at_ms: T0 - 30 * 86_400_000, consent_version: 1 },
  {
    rows: STATS_ROWS,
    next_upload_ms: T0 + 2 * 86_400_000,
    reports: [
      { week: "2026-W41", status: "pending", reason: null, upload_at_ms: T0 + 2 * 86_400_000, next_try_ms: null, attempts: 0, bytes: 1_204, synced: false },
      { week: "2026-W40", status: "sent", reason: null, upload_at_ms: null, next_try_ms: null, attempts: 0, bytes: 988, synced: false },
      { week: "2026-W39", status: "skipped", reason: "Sharing didn't cover the whole week — never sent", upload_at_ms: null, next_try_ms: null, attempts: 0, bytes: 1_051, synced: false },
    ],
  },
);

const UPLOAD_ERROR = {
  ...SHARE,
  last_error: "Couldn't reach Figurehead.",
  next_upload_ms: T0 + 3 * 3_600_000,
  reports: [
    { week: "2026-W41", status: "pending", reason: null, upload_at_ms: T0 - 3_600_000, next_try_ms: T0 + 3 * 3_600_000, attempts: 2, bytes: 1_204, synced: false },
    ...SHARE.reports.slice(1),
  ],
};

const REPORT_JSON = JSON.stringify({
  schema: 1,
  report_id: "0b6f2d4e-6a3c-4f0e-9d7b-3c1e5a9f8b21",
  week: "2026-W41",
  sample_rate: 1,
  app: { version: "0.3.1", channel: "beta" },
  os: { name: "macos", major: 15, arch: "arm64" },
  install_age: "1-4w",
  days_active: 5,
  consent_version: 1,
  entries: [
    { m: "app.launch", d: {}, v: 6 },
    { m: "instance.started", d: { kind: "terminal", origin: "new", provider: "claude-code" }, v: 14 },
    { m: "switch.duration_s", d: { kind: "account" }, h: [1, 2, 0, 0] },
    { m: "projects.count", d: {}, g: "4-5" },
  ],
});

const PRIVACY_STATES: { name: string; seams: Record<string, unknown> }[] = [
  { name: "off", seams: { __AIJ_USAGE_STATS__: statsOverview({}) } },
  { name: "local", seams: { __AIJ_USAGE_STATS__: LOCAL } },
  { name: "share", seams: { __AIJ_USAGE_STATS__: SHARE } },
  {
    name: "overridden",
    seams: {
      __AIJ_USAGE_STATS__: statsOverview({
        chosen: "share",
        effective: "off",
        reason: "Off because DO_NOT_TRACK is set",
        decided_at_ms: T0 - 30 * 86_400_000,
        consent_version: 1,
      }),
    },
  },
  {
    name: "not-included",
    seams: { __AIJ_USAGE_STATS__: statsOverview({ included: false, reason: "Not included in this build" }) },
  },
  { name: "upload-error", seams: { __AIJ_USAGE_STATS__: UPLOAD_ERROR } },
  {
    name: "synced",
    seams: {
      __AIJ_USAGE_STATS__: {
        ...SHARE,
        sync: { offered: true, on: true, email: "you@example.com", notice: null },
        reports: SHARE.reports.map((row, index) => (index === 1 ? { ...row, synced: true } : row)),
      },
    },
  },
];

for (const theme of THEMES) {
  for (const vp of STATS_VP) {
    for (const state of PRIVACY_STATES) {
      test(`62 privacy — ${state.name} — ${vp.name} — ${theme}`, async ({ page }) => {
        await openSettings(page, theme, vp, state.seams);
        await expect(page.getByTestId("privacy-state")).toBeVisible();
        await page.getByTestId("settings-privacy").evaluate((card) => card.scrollIntoView({ block: "start" }));
        await page.mouse.move(0, 0);
        await page.screenshot({ path: resolve(GALLERY, `62-privacy/${state.name}-${vp.name}-${theme}.png`) });
      });
    }

    test(`62 privacy — a report in the read-only viewer — ${vp.name} — ${theme}`, async ({ page }) => {
      await openSettings(page, theme, vp, {
        __AIJ_USAGE_STATS__: SHARE,
        __AIJ_USAGE_STATS_REPORTS__: { "pending/2026-W41": REPORT_JSON },
      });
      await page.getByTestId("privacy-report-view").first().click();
      await expect(page.getByTestId("privacy-report-json")).toContainText('"week": "2026-W41"');
      await page.getByTestId("privacy-report-json").evaluate((viewer) => viewer.scrollIntoView({ block: "end" }));
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(GALLERY, `62-privacy/report-json-${vp.name}-${theme}.png`) });
    });

    test(`63 consent — asked again after a consent change — ${vp.name} — ${theme}`, async ({ page }) => {
      // The dialog opens over the home screen at launch.
      await page.addInitScript(
        ({ fixture, t, stats, t0 }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_USAGE_STATS__ = stats;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        {
          fixture: allStatuses,
          t: theme,
          t0: T0,
          stats: statsOverview({
            chosen: "share",
            effective: "share",
            decided_at_ms: T0 - 90 * 86_400_000,
            consent_version: 1,
            current_consent_version: 2,
          }),
        },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await expect(page.getByTestId("usage-stats-reconsent")).toBeVisible();
      await expect(page.locator("[data-testid='usage-stats-reconsent'] input:checked")).toHaveAttribute("value", "share");
      await page.screenshot({ path: resolve(GALLERY, `63-consent/reask-${vp.name}-${theme}.png`) });
    });
  }
}

// ---------------------------------------------------------------------
// AJ-E1107 — 64-crash: the crash-report setting in Settings › Privacy
// and the next-launch question with the full report — 1280×800,
// light/dark.
// ---------------------------------------------------------------------

const CRASH_REPORT = {
  id: "a1b2c3d4e5f60718",
  at_ms: T0 - 3_600_000,
  signature: "9f2c1a0b7e3d4c5f",
  message: "called `Option::unwrap()` on a `None` value",
  location: "crates/juggler-app/src/commands.rs:4211",
  frames: [
    "0: std::panicking::begin_panic_handler",
    "1: core::option::unwrap_failed",
    "2: juggler_app::commands::switch_execute_core",
    "at ~/Dev/AIJuggler/crates/juggler-app/src/commands.rs:4211:41",
    "3: juggler_app::commands::switch_execute",
  ],
  app_version: "0.3.1",
  channel: "beta",
  os: "macos",
};

for (const theme of THEMES) {
  test(`64 crash — the setting (ask, nothing sendable yet) — ${theme}`, async ({ page }) => {
    await openSettings(page, theme, { width: 1280, height: 800 }, {
      __AIJ_USAGE_STATS__: SHARE,
      __AIJ_CRASH__: { setting: "ask", effective: "ask", reason: null, asking: [], queued: 1, can_send: false },
    });
    await page.getByTestId("privacy-crash-reports").evaluate((row) => row.scrollIntoView({ block: "center" }));
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(GALLERY, `64-crash/setting-1280x800-${theme}.png`) });
  });

  test(`64 crash — the next-launch question — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t, crash, t0 }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_CRASH__ = crash;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t: theme,
        t0: T0,
        crash: { setting: "ask", effective: "ask", reason: null, asking: [CRASH_REPORT], queued: 0, can_send: false },
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expect(page.getByTestId("crash-prompt")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `64-crash/ask-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// Wireframe 26 — 65-settings-sections: the section list beside the
// policies, each new section jumped to (Context packs & secrets,
// Cleanup, Accounts & renewal dates, Notifications, Fleet permissions)
// — 1280×800, plus the list at the 1024×680 minimum — light/dark.
// ---------------------------------------------------------------------

const SETTINGS_PEER = {
  machine_id: "machine-mini",
  label: "Mac mini",
  fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
  permission: "control",
  permission_label: "View + control",
  ask_each_time: true,
  paired_via: "code",
  paired_at_ms: T0 - 86_400_000,
  last_seen_ms: T0 - 60_000,
  connection: "link_off",
  protocol: null,
  connection_detail: null,
};
const SETTINGS_SECTIONS: Array<[string, string]> = [
  ["sessions", "Sessions"],
  ["packs", "Context packs & secrets"],
  ["cleanup", "Cleanup"],
  ["renewal", "Accounts & renewal dates"],
  ["notifications", "Notifications"],
  ["fleet", "Fleet permissions"],
];

for (const theme of THEMES) {
  for (const [name, title] of SETTINGS_SECTIONS) {
    test(`65 settings sections — ${name} — ${theme}`, async ({ page }) => {
      await openSettings(page, theme, { width: 1280, height: 800 }, { __AIJ_FLEET_PEERS__: [SETTINGS_PEER] });
      await page.getByRole("navigation", { name: "Settings sections" }).getByRole("button", { name: title }).click();
      await expect(page.locator(`#settings-${name}-title`)).toBeFocused();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(GALLERY, `65-settings-sections/${name}-1280x800-${theme}.png`) });
    });
  }

  test(`65 settings sections — the list at the minimum window — ${theme}`, async ({ page }) => {
    await openSettings(page, theme, { width: 1024, height: 680 }, {});
    await expect(page.getByTestId("settings-nav")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `65-settings-sections/list-1024x680-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E1503 — 75-feedback: the one toast (a held Stop with Undo, undone,
// Mark reviewed), the quit sheet, and the loading placeholders for the
// rail, Home and Accounts — 1280×800, light/dark. Toasts are paused for
// the photo (__AIJ_TOAST_PAUSED__); the lists are held loading with
// __AIJ_LOAD_HANG__.
// ---------------------------------------------------------------------

async function openFeedback(
  page: import("@playwright/test").Page,
  theme: string,
  seams: Record<string, unknown> = {},
) {
  await page.addInitScript(
    ({ fixture, t, t0, seams }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_TOAST_PAUSED__ = true;
      for (const [key, value] of Object.entries(seams)) w[key] = value;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t: theme, t0: T0, seams },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

for (const theme of THEMES) {
  test(`75 feedback — a held stop, with Undo — ${theme}`, async ({ page }) => {
    await openFeedback(page, theme);
    await page.locator('[data-instance-id="sess-2"]').first().click();
    await page.getByTestId("inspector-stop").click();
    await expect(page.getByTestId("stop-undo-toast")).toContainText("Undo keeps it running.");
    await expect(page.getByTestId("stop-undo")).toContainText("Undo (5s)");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(GALLERY, `75-feedback/stop-held-1280x800-${theme}.png`) });
    await page.getByTestId("stop-undo").click();
    await expect(page.getByTestId("toast-undone")).toContainText("keeps running.");
    await page.screenshot({ path: resolve(GALLERY, `75-feedback/stop-undone-1280x800-${theme}.png`) });
  });

  test(`75 feedback — mark reviewed, with Undo — ${theme}`, async ({ page }) => {
    await openFeedback(page, theme);
    await page.getByTestId("home").getByRole("button", { name: /^Mark reviewed/ }).first().click();
    await expect(page.getByTestId("toast")).toContainText("reviewed.");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(GALLERY, `75-feedback/reviewed-1280x800-${theme}.png`) });
  });

  test(`75 feedback — the quit sheet — ${theme}`, async ({ page }) => {
    await openFeedback(page, theme);
    await expect(page.getByTestId("home")).toBeVisible();
    await page.evaluate(() =>
      window.dispatchEvent(new CustomEvent("aij:quit-requested", { detail: { live: 3, end_on_quit: 2, keep_running: 1 } })),
    );
    await expect(page.getByTestId("quit-sheet")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `75-feedback/quit-sheet-1280x800-${theme}.png`) });
  });

  test(`75 feedback — loading placeholders — ${theme}`, async ({ page }) => {
    await openFeedback(page, theme, { __AIJ_LOAD_HANG__: true });
    await expect(page.getByTestId("placeholder-rail")).toBeVisible();
    await expect(page.getByTestId("home-loading")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `75-feedback/loading-home-1280x800-${theme}.png`) });
    await goTo(page, "Accounts");
    await expect(page.getByTestId("placeholder-card")).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `75-feedback/loading-accounts-1280x800-${theme}.png`) });
  });
}

// ---------------------------------------------------------------------
// AJ-E1504 — 76-keys: the ? sheet (every registered shortcut, by area)
// and the inspector folded to its header line (⌘⇧I) — 1280×800 and the
// 1024×680 minimum, light/dark.
// ---------------------------------------------------------------------

for (const theme of THEMES) {
  for (const vp of [{ name: "1280x800", width: 1280, height: 800 }, { name: "1024x680", width: 1024, height: 680 }]) {
    test(`76 keys — the ? sheet — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFeedback(page, theme);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await expect(page.getByTestId("home")).toBeVisible();
      await page.keyboard.press("Shift+Slash");
      await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
      await page.screenshot({ path: resolve(GALLERY, `76-keys/sheet-${vp.name}-${theme}.png`) });
    });

    test(`76 keys — the inspector folded — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFeedback(page, theme);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.locator('[data-instance-id="sess-2"]').first().click();
      await expect(page.getByRole("tablist")).toBeVisible();
      await page.keyboard.press("ControlOrMeta+Shift+I");
      await expect(page.getByTestId("right-inspector")).toHaveAttribute("data-folded", "true");
      await expect(page.getByRole("tablist")).toBeHidden();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(GALLERY, `76-keys/folded-${vp.name}-${theme}.png`) });
    });
  }
}

// AJ-E1505: Settings › Sessions with the Terminal disclosure open and a
// changed value (its Reset shown) — 1280×800, light/dark.
for (const theme of THEMES) {
  test(`65 settings sections — sessions, terminal open — ${theme}`, async ({ page }) => {
    await openSettings(page, theme, { width: 1280, height: 800 }, {
      __AIJ_SESSION_PREFS__: { stuck_after_minutes: 10, terminal_font_size: 12.5, terminal_scrollback_lines: 5000 },
    });
    await page.getByRole("navigation", { name: "Settings sections" }).getByRole("button", { name: "Sessions" }).click();
    await page.getByTestId("settings-terminal").locator("summary").click();
    await expect(page.getByLabel("Scrollback")).toBeVisible();
    await expect(page.getByTestId("pref-stuck_after_minutes-reset")).toHaveText("Reset to 5");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(GALLERY, `65-settings-sections/sessions-terminal-1280x800-${theme}.png`) });
  });
}
