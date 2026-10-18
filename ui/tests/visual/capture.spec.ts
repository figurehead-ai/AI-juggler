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
import { test, expect } from "@playwright/test";
import { mkdirSync, writeFileSync, existsSync, appendFileSync } from "node:fs";
import { resolve } from "node:path";

// Playwright runs with cwd = config dir (ui/).
const GALLERY_ROOT = resolve(process.cwd(), "../docs/qa/evidence/_gallery");
import { allStatuses, answerPolicy, honestEmpty, readyHome } from "./fixtures";

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
  for (const screen of ["01-rail", "01-rail-density", "01-rail-sort", "05-session-panel", "06-inspector", "07-inspector-content", "08-inspector-aggregate", "09-quality", "10-processes", "11-renewal", "12-model", "13-desktop", "14-focus", "15-preview", "16-onboarding-detect", "17-ledger", "18-ports", "19-cleanup", "20-kill", "21-tiers", "22-recovery", "23-alerts", "24-fleet", "25-costs", "26-quota", "27-ranking", "28-switch", "29-door", "30-switchback", "31-redaction", "32-graph", "33-beta", "34-wizard", "35-candidates", "36-recommend", "37-flows", "38-flows-cde", "39-import", "40-login", "41-policy"]) {
    mkdirSync(resolve(GALLERY, screen), { recursive: true });
  }
  writeFileSync(NOT_CAPTURETED, "# Not captured (explicit omissions)\n\n| Screen | State | Reason |\n|---|---|---|\n");
  // Loading/error states do not exist yet — recorded, not hidden.
  for (const screen of ["01-rail", "05-session-panel", "16-onboarding-detect"]) {
    noteNotCaptured(screen, "loading (skeleton)", "pending AJ-E405+ (state emission not yet implemented)");
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
      await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
      await page.getByRole("tab", { name: "Sessions" }).click();
      await expect(page.getByText("Handed over").first()).toBeVisible();
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
      await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
      await page.getByRole("tab", { name: "Sessions" }).click();
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
      await expect(page.getByText("Nothing selected", { exact: false })).toBeVisible();
      await expect(page.getByText("Lowest quota")).toBeVisible();
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
      await expect(page.getByText("Get your accounts ready")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `01-rail/ready-home-${vp.name}-${theme}.png`),
      });
    });

    test(`onboarding detect — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, theme }) => {
          (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
          localStorage.setItem("aij.theme", theme);
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: honestEmpty, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      // AJ-E301: Step 0 (the install gate) now precedes the scan —
      // the preview default has Claude installed, so Continue is
      // enabled and advances; the not-installed state is captured in
      // the dedicated Step-0 shots below.
      await expect(page.getByTestId("wizard-step-install")).toBeVisible();
      await expect(page.getByTestId("install-continue")).toBeEnabled();
      await page.getByTestId("install-continue").click();
      await expect(page.getByText("Let's look at what", { exact: false })).toBeVisible();
      await page.waitForTimeout(400); // scan mock resolves
      await expect(page.getByText(/signed in|not signed in/).first()).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `16-onboarding-detect/populated-${vp.name}-${theme}.png`),
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
      await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
      await page.getByRole("tab", { name: "Sessions" }).click();
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
  await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.selectOption('select[aria-label="Sort by"]', "quota");
  await expect(page.getByText("no data").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "01-rail-sort/quota-risk-1440x900.png") });
});

test("rail — group by project with rollups", async ({ page }) => {
  await seedRail(page, {});
  await page.selectOption('select[aria-label="Group by"]', "project");
  await expect(page.getByText(/running/).first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "01-rail-sort/group-by-project-1440x900.png") });
});

test("rail — empty search state", async ({ page }) => {
  await seedRail(page, {});
  await page.locator('input[type="search"]').fill("does-not-exist");
  await expect(page.getByText("No instances match")).toBeVisible();
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
  await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
  await page.getByRole("tab", { name: "Sessions" }).click();
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
    await expect(page.getByText("Nothing selected", { exact: false })).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `06-inspector/empty-${vp.name}.png`) });
  });

  test(`inspector — Instance / Overview — ${vp.name}`, async ({ page }) => {
    await seedInspectorPage(page, vp);
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await expect(page.getByRole("tab", { name: "Quality / Performance" })).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `06-inspector/instance-overview-${vp.name}.png`) });
  });

  test(`inspector — Account / Usage honest empty — ${vp.name}`, async ({ page }) => {
    await seedInspectorPage(page, vp);
    await page.getByRole("button", { name: "Accounts" }).click();
    await page.getByRole("button", { name: "Inspect claude-code" }).first().click();
    await expect(page.getByRole("tab", { name: "Usage" })).toBeVisible();
    await page.getByRole("tab", { name: "Usage" }).click();
    await expect(page.getByText("No measured usage yet", { exact: false })).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `06-inspector/account-usage-${vp.name}.png`) });
  });

  test(`inspector — Project / Processes — ${vp.name}`, async ({ page }) => {
    await seedInspectorPage(page, vp);
    await page.getByRole("tab", { name: "Projects" }).click();
    await page.getByRole("button", { name: /ai-juggler/ }).first().click();
    await expect(page.getByRole("tab", { name: "Processes" })).toBeVisible();
    await page.screenshot({ path: resolve(GALLERY, `06-inspector/project-processes-${vp.name}.png`) });
  });
}

// AJ-E407 inspector-content captures (Instance + Session tabs).
test("inspector — Instance Overview fields (populated)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await expect(page.getByTestId("instance-overview-fields")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "07-inspector-content/instance-overview-populated.png") });
});

test("inspector — Instance Usage honest chart empties", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Usage" }).click();
  await expect(page.locator('section:not([hidden])').getByText('No data — arrives with statusline').first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "07-inspector-content/instance-usage.png") });
});

test("inspector — Instance Actions disabled-with-reason", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Actions" }).click();
  await expect(page.getByText("AJ-E202").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "07-inspector-content/instance-actions.png") });
});

test("inspector — Session Overview ended stamp", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Personal/ }).first().click();
  await expect(page.getByText(/Ended|Conversation|Transcript not readable yet/).first()).toBeVisible();
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
  await page.getByRole("button", { name: "Accounts" }).click();
  await page.getByRole("button", { name: "Inspect claude-code" }).first().click();
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
  await page.getByRole("button", { name: "Accounts" }).click();
  await page.getByRole("button", { name: "Inspect claude-code" }).nth(1).click();
  await expect(page.getByTestId("account-signin-card")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "08-inspector-aggregate/account-logged-out-1280x800.png") });
});

test("inspector — Project Overview On-it-now card", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Projects" }).click();
  await page.getByRole("button", { name: /ai-juggler/ }).first().click();
  await expect(page.getByTestId("project-on-it-now")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "08-inspector-aggregate/project-overview-1440x900.png") });
});

// AJ-E409 quality-tab captures.
test("inspector — Quality tab populated", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Quality / Performance" }).click();
  await expect(page.getByTestId("quality-tokens")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "09-quality/populated-1440x900.png") });
});

test("inspector — Quality tab View as table", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Processes" }).click();
  await expect(page.getByTestId("processes-spark")).toBeVisible();
  await expect(page.getByTestId("orphan-row-39871")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "10-processes/live-tree-1440x900.png") });
});

test("inspector — Processes tab Stop confirm popover (PID named)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.getByRole("tab", { name: "Processes" }).click();
  await expect(page.getByTestId("orphan-row-39871")).toBeVisible();
  await page.getByRole("button", { name: "Stop process 39871" }).click();
  await expect(page.getByTestId("orphan-confirm-39871")).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "10-processes/stop-confirm-1440x900.png") });
});

test("inspector — Processes tab gone verification state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("button", { name: "Accounts" }).click();
  await page.getByRole("button", { name: "Inspect claude-code" }).first().click();
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
  await expect(page.getByText("Entered by you - Juggler does not read billing")).toBeVisible();
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
  await page.getByRole("button", { name: "Accounts" }).click();
  await expect(page.getByText("Renews in 6 days").first()).toBeVisible();
  await page.screenshot({ path: resolve(GALLERY, "11-renewal/accounts-row-chip-1440x900.png") });
});

// AJ-E412 running-model captures: I-03 plain vs drift (sess-8 carries
// the drift fixture: launched opus, observed sonnet-5), the Overview
// dual-model line, and the not-reported inspector state.
test("model — I-03 drift marker on the rail row", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  const marker = page.getByTestId("model-drift").first();
  await expect(marker).toBeVisible();
  await expect(marker).toHaveAttribute("aria-label", "Launched with opus - now running sonnet-5");
  await marker.hover();
  await page.screenshot({ path: resolve(GALLERY, "12-model/rail-drift-1440x900.png") });
});

test("model — I-03 plain (launch == running rows show no marker)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  // The plain rows carry launch-only models; the marker count is exactly
  // the drift rows (one), asserted before the screenshot.
  expect(await page.getByTestId("model-drift").count()).toBe(1);
  await page.screenshot({ path: resolve(GALLERY, "12-model/rail-plain-1440x900.png") });
});

test("model — Instance Overview dual-model line", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await expect(page.getByTestId("instance-model-line")).toBeVisible();
  await expect(page.getByTestId("instance-model-line")).toContainText(
    "Launched with opus - now running sonnet-5",
  );
  await page.screenshot({ path: resolve(GALLERY, "12-model/overview-drift-1440x900.png") });
});

test("model — not reported by Provider inspector state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1440, height: 900 });
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Personal/ }).first().click();
  await expect(page.getByTestId("instance-model-line")).toContainText(
    "Model: not reported by claude-code",
  );
  await page.screenshot({ path: resolve(GALLERY, "12-model/not-reported-1440x900.png") });
});

// AJ-E413 desktop captures: the panel surface (no terminal card), the
// rail `app` chip, and the state matrix (running, exited, crashed,
// single-account warning) at the two required viewports.
test("desktop — Instance panel (running app, isolation label, no terminal)", async ({ page }) => {
  await seedInspectorPage(page, { width: 1280, height: 800 });
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /finished/i })
    .click();
  await expect(page.getByTestId("desktop-surface")).toBeVisible();
  await expect(page.getByTestId("desktop-surface")).toContainText("app exited");
  await page.screenshot({ path: resolve(GALLERY, "13-desktop/panel-exited-1280x800.png") });
});

test("desktop — crashed app state", async ({ page }) => {
  await seedInspectorPage(page, { width: 1280, height: 800 });
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
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
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page
    .getByRole("button", { name: /Fake Desk/ })
    .filter({ has: page.getByTestId("rail-app-chip") })
    .filter({ hasText: /finished/i })
    .click();
  await expect(page.getByTestId("desktop-surface")).toContainText("app exited");
  await expect(page.getByTestId("desktop-preview")).toHaveCount(0);
  await page.screenshot({ path: resolve(GALLERY, "15-preview/panel-exited-1280x800.png") });
});

// AJ-E501: the ledger-degraded banner (the Cleanup Centre stub's only
// surface) — copy exact, both themes, on the Overview screen.
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
    await page.getByRole("button", { name: "Overview" }).click();
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
    await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
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
      await page.getByRole("tab", { name: "Sessions" }).click();
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
    await page.getByRole("tab", { name: "Sessions" }).click();
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
const CLEANUP_ROW = (pid: number, state: string) => ({
  pid,
  pgid: 8100,
  role: "helper",
  state,
  state_label:
    state === "orphaned"
      ? "Still running after its Instance closed"
      : state === "adopted"
        ? "Running — re-adopted"
        : state === "unverifiable"
          ? "Unverifiable — will not be touched"
          : "Running",
  role_guess: "python3 (guess)",
  exe_path: "/usr/bin/python3",
  cmdline: "python3 -m http.server 5173",
  age_ms: 9_000_000,
  rss_bytes: 48 * 1024 * 1024,
  cpu_pct: 1.4,
  ports: state === "orphaned" ? "5173" : null,
  hog: pid === 8124,
  hog_rules: pid === 8124 ? ["tree-rss>p95(provider)"] : [],
  evidence: {
    root_pid: 8100,
    parent_pid: 8100,
    pgid: 8100,
    escaped_group: false,
    verification: {
      verdict: state === "unverifiable" ? "unverifiable" : "verified",
      detail:
        state === "unverifiable"
          ? "live start-time or exe differs from the record"
          : "start-time and exe match the live process",
    },
    first_seen: T0,
    last_seen: T0 + 90_000,
  },
});

function cleanupFixture(state: string) {
  if (state === "empty") {
    return { groups: [], clean_instances: 3, ledger_degraded: false };
  }
  if (state === "degraded") {
    return { groups: [], clean_instances: 0, ledger_degraded: true };
  }
  // The FILTER states map to row states: needs-attention shows the
  // orphan + the hog; re-adopted shows the adopted row; unverifiable
  // shows the flagged one; all-managed shows the mix.
  const firstState =
    state === "needs-attention"
      ? "orphaned"
      : state === "re-adopted"
        ? "adopted"
        : "running";
  const rows = [
    CLEANUP_ROW(8123, firstState),
    CLEANUP_ROW(8124, state === "unverifiable" ? "unverifiable" : "running"),
  ];
  return {
    groups: [
      {
        session: "s-1",
        provider: "claude-code",
        account: "Work Max",
        project: "web",
        machine: "machine-local",
        healthy: false,
        rows,
      },
    ],
    clean_instances: 2,
    ledger_degraded: false,
  };
}

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
        await page.getByRole("button", { name: "Cleanup" }).click();
        await expect(page.getByTestId("cleanup-centre")).toBeVisible();
        if (state === "needs-attention") {
          await expect(page.getByTestId("cleanup-row-8123")).toContainText(
            "Still running after its Instance closed",
          );
          await expect(page.getByTestId("cleanup-row-8124")).toContainText("hog");
        }
        if (state === "empty") {
          await expect(page.getByTestId("cleanup-empty")).toContainText(
            "Nothing left behind — 3 Instances running clean.",
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
    await page.getByRole("button", { name: "Cleanup" }).click();
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
    await page.getByRole("button", { name: "Cleanup" }).click();
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
        await page.getByRole("button", { name: "Cleanup" }).click();
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
      await page.getByRole("button", { name: "Cleanup" }).click();
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
      await page.getByRole("button", { name: "Cleanup" }).click();
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
      await page.getByRole("button", { name: "Cleanup" }).click();
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
      await page.getByRole("button", { name: "Cleanup" }).click();
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
    "2 Instances re-adopted, 1 helpers left behind, 1 processes could not be verified and were left alone.",
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
    await page.getByRole("tab", { name: "Sessions" }).click();
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
    await page.getByRole("button", { name: "Cleanup" }).click();
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
  text: "node (vite) under Work Max holds 3.1 GiB — review in Cleanup Centre.",
  pid: 8123,
  session: "s-1",
};
const IDLE_ALERT = {
  kind: "idle",
  text: "python3 has been idle for 2h — review it in the Cleanup Centre.",
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
      await page.getByRole("button", { name: "Cleanup" }).click();
      await expect(page.getByTestId("cleanup-alerts")).toBeVisible();
      await expect(page.getByTestId("hog-alert-8123")).toContainText(
        "node (vite) under Work Max holds 3.1 GiB — review in Cleanup Centre.",
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
      await page.getByRole("button", { name: "Cleanup" }).click();
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
      await page.getByRole("button", { name: "Cleanup" }).click();
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
  await page.getByRole("button", { name: "Cleanup" }).click();
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
          state_label: "Still running after its Instance closed",
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
          state_label: "Still running after its Instance closed",
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

for (const theme of THEMES) {
  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x700", width: 1024, height: 700 },
  ]) {
    test(`fleet — machine column + filter + foreign display-only — ${vp.name} — ${theme}`, async ({ page }) => {
      await seedRecoveryPage(page, theme, { __AIJ_CLEANUP__: FLEET_CLEANUP });
      await page.getByRole("button", { name: "Cleanup" }).click();
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
        // The matrix rides under the WorkPanels empty state (no
        // sessions needed — it renders from the seam).
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
        await page.getByRole("tab", { name: "Sessions" }).click();
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
      await page.getByRole("button", { name: "New", exact: true }).click();
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
      await page.getByRole("button", { name: "New", exact: true }).click();
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
      await page.getByRole("tab", { name: "Sessions" }).click();
      await page.getByRole("button", { name: /Work Max/ }).first().click();
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
      await page.getByRole("tab", { name: "Sessions" }).click();
      await page.getByRole("button", { name: /Work Max/ }).first().click();
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
      await page.getByRole("tab", { name: "Sessions" }).click();
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
      await expect(page.getByTestId("switch-back-sf7")).toContainText("fresh instance");
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
      await page.getByRole("tab", { name: "Sessions" }).click();
      await page.getByRole("button", { name: /Work Max/ }).first().click();
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
      await page.getByRole("tab", { name: "Sessions" }).click();
      await page.getByRole("button", { name: /Work Max/ }).first().click();
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
      await page.getByRole("tab", { name: "Projects" }).click();
      await page.getByRole("button", { name: /ai-juggler/ }).first().click();
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
      await page.getByRole("tab", { name: "Projects" }).click();
      await page.getByRole("button", { name: /ai-juggler/ }).first().click();
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
      await page.getByRole("tab", { name: "Projects" }).click();
      await page.getByRole("button", { name: /ai-juggler/ }).first().click();
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
      await page.getByRole("tab", { name: "Projects" }).click();
      await page.getByRole("button", { name: /ai-juggler/ }).first().click();
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
      await page.getByRole("tab", { name: "Sessions" }).click();
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
      await page.getByRole("button", { name: "Accounts" }).click();
      await expect(page.getByTestId("beta-matrix")).toBeVisible();
    };

    test(`beta — matrix, all pairs open debt — ${vp.name} — ${theme}`, async ({ page }) => {
      await openProfiles(page);
      await expect(page.getByTestId("beta-matrix-summary")).toContainText(
        "All 11 provider pairs ship in beta",
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
      await page.getByRole("tab", { name: "Sessions" }).click();
      await page.getByRole("button", { name: /Work Max/ }).first().click();
      await page.getByTestId("handover-section")
        .locator("button[data-row]:not(:disabled)")
        .first()
        .click();
      await expect(page.getByTestId("switch-wizard")).toBeVisible();
      await expect(page.getByTestId("switch-wizard")).toContainText("· beta");
      await page.screenshot({
        path: resolve(GALLERY, `33-beta/badge-in-situ-${vp.name}-${theme}.png`),
      });
    });
  }
}

/**
 * AJ-E301 gallery 34 — the wizard's Step-0 install gate: installed /
 * not-installed (install help open) / unverified rows, plus the
 * in-flight close confirm — at 1280×800 and 390×844 (narrow sheet) ×
 * light+dark. The gate precedes everything: a missing CLI never
 * reaches Identity.
 */
const WIZARD_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "390x844", width: 390, height: 844 },
] as const;

const openWizardStep0 = async (
  page: import("@playwright/test").Page,
  installs: unknown[],
  theme: string,
  vp: { width: number; height: number },
) => {
  await page.addInitScript(
    ({ fixture, installs: rows, theme: themeName }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROVIDER_INSTALLS__ = rows;
      localStorage.setItem("aij.theme", themeName);
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: honestEmpty, installs, theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  await expect(page.getByTestId("wizard-step-install")).toBeVisible();
};

for (const theme of THEMES) {
  for (const vp of WIZARD_VP) {
    test(`wizard step0 — installed — ${vp.name} — ${theme}`, async ({ page }) => {
      await openWizardStep0(page, [
        {
          provider: "claude-code",
          display_name: "Claude Code",
          install_url: "https://claude.com/download",
          status: "installed",
          version: "2.1.296",
          binary: "/usr/local/bin/claude",
          notes: [],
        },
      ], theme, vp);
      await expect(page.getByTestId("install-continue")).toBeEnabled();
      await page.screenshot({
        path: resolve(GALLERY, `34-wizard/installed-${vp.name}-${theme}.png`),
      });
    });

    test(`wizard step0 — not installed (gate + help) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openWizardStep0(page, [
        {
          provider: "claude-code",
          display_name: "Claude Code",
          install_url: "https://claude.com/download",
          status: "not_installed",
          version: null,
          binary: null,
          notes: [],
        },
      ], theme, vp);
      await expect(page.getByTestId("install-continue")).toBeDisabled();
      await page.locator("[data-testid='provider-row'] button").first().click();
      await expect(page.getByTestId("install-help")).toContainText("Install Claude Code →");
      await page.screenshot({
        path: resolve(GALLERY, `34-wizard/not-installed-${vp.name}-${theme}.png`),
      });
    });

    test(`wizard step0 — unverified — ${vp.name} — ${theme}`, async ({ page }) => {
      await openWizardStep0(page, [
        {
          provider: "claude-code",
          display_name: "Claude Code",
          install_url: "https://claude.com/download",
          status: "unverified",
          version: "2.2.7",
          binary: "/usr/local/bin/claude",
          notes: ["version drift: 2.2.7; this adapter was verified against 2.1.296"],
        },
      ], theme, vp);
      await expect(page.getByTestId("provider-row")).toContainText("unverified");
      await expect(page.getByTestId("install-continue")).toBeEnabled();
      await page.screenshot({
        path: resolve(GALLERY, `34-wizard/unverified-${vp.name}-${theme}.png`),
      });
    });
  }

  test(`wizard — close confirm while busy — 1280x800 — ${theme}`, async ({ page }) => {
    // The close-guard visual: the confirm card over a "busy" wizard.
    await page.addInitScript(
      ({ fixture, installs: rows, theme: themeName }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_PROVIDER_INSTALLS__ = rows;
        // createAccount hangs → the wizard stays "busy" in identity.
        w.__AIJ_CREATE_HANG__ = true;
        localStorage.setItem("aij.theme", themeName);
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      {
        fixture: honestEmpty,
        theme,
        installs: [
          {
            provider: "claude-code",
            display_name: "Claude Code",
            install_url: "https://claude.com/download",
            status: "installed",
            version: "2.1.296",
            binary: "/usr/local/bin/claude",
            notes: [],
          },
        ],
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByTestId("install-continue").click();
    await page.getByRole("button", { name: /fresh account/ }).click();
    await page.getByPlaceholder("e.g. Work, Personal, Side project").fill("Work");
    await answerPolicy(page);
    await page.getByRole("button", { name: /Create account/ }).click();
    await page.getByRole("button", { name: "Close setup" }).click({ force: true });
    await expect(page.getByTestId("onboarding-wizard")).toBeVisible();
    await page.screenshot({
      path: resolve(GALLERY, `34-wizard/close-guard-1280x800-${theme}.png`),
    });
  });
}

/**
 * AJ-E302 gallery 34 additions — the 10-signal scan in the wizard's
 * detect step: scanning-in-progress (the "n of 10" text), completed
 * with Details open, and the partial/denied state — 1280×800 ×
 * light+dark. Progress is text (aria-live), never colour-only.
 */
const SCAN_DENIED_REPORT = {
  installs: [],
  probes: [
    { id: "s1-config-dirs", label: "Config directories", health: "ok", reason: null, detail: "0 candidate setup(s)" },
    { id: "s2-env-overrides", label: "Environment overrides", health: "ok", reason: null, detail: "none set in this app's environment" },
    { id: "s3-manager-t3", label: "t3 profiles", health: "ok", reason: null, detail: "not present" },
    { id: "s4-manager-cc-switch", label: "cc-switch database", health: "denied", reason: "cc-switch database is locked or unreadable", detail: "database present (not readable — existence only)" },
    { id: "s5-shell-aliases", label: "Shell aliases", health: "partial", reason: "1 assignment points at a directory that no longer exists", detail: "1 assignment(s) (1 dangling)" },
    { id: "s6-installed-clis", label: "Installed CLIs", health: "ok", reason: null, detail: "claude-code: installed" },
    { id: "s7-login-metadata", label: "Login metadata", health: "ok", reason: null, detail: "0 of 0 signed in" },
    { id: "s8-usage-recency", label: "Usage recency", health: "ok", reason: null, detail: "0 setup(s) with recent activity" },
    { id: "s9-projects", label: "Projects", health: "ok", reason: null, detail: "0 project name(s)" },
    { id: "s10-desktop", label: "Desktop apps", health: "ok", reason: null, detail: "none of the known desktop apps present" },
  ],
  seeds: [],
  desktop_apps: [],
  cancelled: false,
};

for (const theme of THEMES) {
  const openDetect = async (
    page: import("@playwright/test").Page,
    scanReport?: unknown,
  ) => {
    await page.addInitScript(
      ({ fixture, installs, scan, theme: themeName }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_PROVIDER_INSTALLS__ = installs;
        if (scan !== undefined) w.__AIJ_SCAN_REPORT__ = scan;
        localStorage.setItem("aij.theme", themeName);
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      {
        fixture: honestEmpty,
        theme,
        installs: [
          {
            provider: "claude-code",
            display_name: "Claude Code",
            install_url: "https://claude.com/download",
            status: "installed",
            version: "2.1.296",
            binary: "/usr/local/bin/claude",
            notes: [],
          },
        ],
        scan: scanReport,
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByTestId("install-continue").click();
  };

  test(`scan — in progress (n of 10) — 1280x800 — ${theme}`, async ({ page }) => {
    await openDetect(page);
    await expect(page.getByTestId("scan-progress")).toContainText("of 10 checks");
    await expect(page.getByTestId("scan-progress")).toHaveAttribute("aria-live", "polite");
    await page.screenshot({
      path: resolve(GALLERY, `34-wizard/scan-progress-1280x800-${theme}.png`),
    });
  });

  test(`scan — completed with Details — 1280x800 — ${theme}`, async ({ page }) => {
    await openDetect(page);
    await page.getByTestId("scan-details-toggle").waitFor({ state: "visible" });
    await page.getByTestId("scan-details-toggle").click();
    await expect(page.getByTestId("scan-details")).toContainText("cc-switch");
    await page.screenshot({
      path: resolve(GALLERY, `34-wizard/scan-details-1280x800-${theme}.png`),
    });
  });

  test(`scan — denied state never reads empty — 1280x800 — ${theme}`, async ({ page }) => {
    await openDetect(page, SCAN_DENIED_REPORT);
    await expect(page.getByTestId("scan-denied")).toContainText("We couldn't check 1 place");
    await page.screenshot({
      path: resolve(GALLERY, `34-wizard/scan-denied-1280x800-${theme}.png`),
    });
  });
}

/**
 * AJ-E303 gallery 35 — the candidate-card matrix: Found/Likely/
 * Guessed × signed-in / not-signed-in / already-managed (9 states,
 * one screen), plus the deep t3 long-path truncation — 1280×800 and
 * 390×844 × light+dark. Confidence is WORDS; the managed card is
 * disabled inline.
 */
function cardFixture(
  path: string,
  confidence: "found" | "likely" | "guessed",
  login: "signed-in" | "not-signed-in",
  managed = false,
): Record<string, unknown> {
  return {
    harness: "claude-code",
    path,
    origin: "home-pattern",
    login: login === "signed-in" ? "logged_in" : "logged_out",
    bytes: 9_000_000,
    last_used_secs: Math.floor(Date.now() / 1000) - 86_400,
    managed,
    notes: ["A separate Claude configuration — created manually or by another tool."],
    inference: {
      purpose: managed ? "Managed" : confidence === "likely" ? "Work" : "Side",
      purpose_confidence: confidence,
      login,
      login_confidence: "found",
      activity: "active",
      activity_confidence: "found",
      activity_words: "Active this week",
      keychain_bound: login === "signed-in" && path.endsWith("-key"),
    },
    signals: ["home-pattern"],
  };
}

for (const theme of THEMES) {
  for (const vp of WIZARD_VP) {
    test(`candidates — 9-state matrix — ${vp.name} — ${theme}`, async ({ page }) => {
      const installs = [
        cardFixture("/Users/you/.claude", "found", "signed-in"),
        cardFixture("/Users/you/.claude-work", "likely", "signed-in"),
        cardFixture("/Users/you/.claude-side", "guessed", "signed-in"),
        cardFixture("/Users/you/.claude-api", "found", "not-signed-in"),
        cardFixture("/Users/you/.claude-personal", "likely", "not-signed-in"),
        cardFixture("/Users/you/.claude-lab", "guessed", "not-signed-in"),
        cardFixture("/Users/you/.claude-m1", "found", "signed-in", true),
        cardFixture("/Users/you/.claude-m2", "likely", "not-signed-in", true),
        cardFixture("/Users/you/.claude-m3", "guessed", "not-signed-in", true),
      ];
      await page.addInitScript(
        ({ fixture, installs: rows, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_PROVIDER_INSTALLS__ = [
            {
              provider: "claude-code",
              display_name: "Claude Code",
              install_url: "https://claude.com/download",
              status: "installed",
              version: "2.1.296",
              binary: "/usr/local/bin/claude",
              notes: [],
            },
          ];
          w.__AIJ_SCAN_REPORT__ = {
            installs: rows,
            probes: [],
            seeds: [],
            desktop_apps: [],
            cancelled: false,
          };
          localStorage.setItem("aij.theme", themeName);
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: honestEmpty, installs, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await page.getByTestId("install-continue").click();
      await expect(page.getByTestId("candidate-cards").first()).toBeVisible();
      await expect(page.getByTestId("candidate-cards").first()).toContainText("Likely: Work");
      await expect(page.getByTestId("candidate-cards").first()).toContainText("Already in AI Juggler");
      await page.screenshot({
        path: resolve(GALLERY, `35-candidates/matrix-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`candidates — deep t3 long path truncates — ${vp.name} — ${theme}`, async ({ page }) => {
      const deep =
        "/Users/you/Library/Application Support/ClaudeCodeProfiles/profile-42/projects/some-extremely-long-nested-name/with/more/segments";
      await page.addInitScript(
        ({ fixture, install, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_PROVIDER_INSTALLS__ = [
            {
              provider: "claude-code",
              display_name: "Claude Code",
              install_url: "https://claude.com/download",
              status: "installed",
              version: "2.1.296",
              binary: "/usr/local/bin/claude",
              notes: [],
            },
          ];
          w.__AIJ_SCAN_REPORT__ = {
            installs: [install],
            probes: [],
            seeds: [],
            desktop_apps: [],
            cancelled: false,
          };
          localStorage.setItem("aij.theme", themeName);
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        {
          fixture: honestEmpty,
          theme,
          install: cardFixture(deep, "guessed", "not-signed-in"),
        },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await page.getByTestId("install-continue").click();
      const card = page.getByTestId("candidate-card").first();
      await expect(card).toBeVisible();
      // The path truncates (one line) — the card never grows sideways.
      const box = await card.locator("span.truncate").boundingBox();
      expect(box).not.toBeNull();
      expect(box!.width).toBeLessThanOrEqual(vp.width - 80);
      await page.screenshot({
        path: resolve(GALLERY, `35-candidates/long-path-${vp.name}-${theme}.png`),
      });
    });
  }
}

/**
 * AJ-E304 gallery 36 — the recommended-set proposal: the teaser with
 * the worked-example sentence, Customise expanded (checkbox list with
 * rationales), accept-in-progress (per-item states; the add item hangs
 * so the intermediate list is reviewable), and the partial-failure end
 * state (item 2 of 4 rejects with a coded reason — 3 done, 1 failed,
 * 0 aborts) — 1280×800 and 390×844 × light+dark.
 */
const WORKED_EXAMPLE_RECOMMENDATION = {
  items: [
    {
      provider: "claude-code",
      label: "Work",
      action: "import",
      rationale: "Signed in — import keeps history",
      default_on: true,
      path: "/Users/you/.claude-work",
    },
    {
      provider: "claude-code",
      label: "Personal",
      action: "import",
      rationale: "Signed in — import keeps history",
      default_on: true,
      path: "/Users/you/.claude-personal",
    },
    {
      provider: "claude-code",
      label: "Side",
      action: "import",
      rationale: "Active — you'll sign in after import",
      default_on: true,
      path: "/Users/you/.claude-side",
    },
    {
      provider: "codex",
      label: "Codex",
      action: "add",
      rationale: "Add Codex (installed, not set up)",
      default_on: true,
      path: null,
    },
  ],
  summary:
    "Your juggling set: Claude ×3 (Work — signed in; Personal — signed in; Side — needs sign-in) + Codex ×1 (signed in).",
  start_here: "-users-dev-api",
};

const openProposalDetect = async (
  page: import("@playwright/test").Page,
  theme: string,
  vp: { width: number; height: number },
  extraWindows: Record<string, unknown> = {},
  rec: unknown = WORKED_EXAMPLE_RECOMMENDATION,
) => {
  await page.addInitScript(
    ({ fixture, rec, theme: themeName, extra }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROVIDER_INSTALLS__ = [
        {
          provider: "claude-code",
          display_name: "Claude Code",
          install_url: "https://claude.com/download",
          status: "installed",
          version: "2.1.296",
          binary: "/usr/local/bin/claude",
          notes: [],
        },
      ];
      w.__AIJ_SCAN_REPORT__ = {
        installs: [
          {
            harness: "claude-code",
            path: "/Users/you/.claude-work",
            origin: "home-pattern",
            login: "logged_in",
            bytes: 9_000_000,
            last_used_secs: Math.floor(Date.now() / 1000) - 86_400,
            managed: false,
            notes: [],
            inference: {
              purpose: "Work",
              purpose_confidence: "found",
              login: "signed-in",
              login_confidence: "found",
              activity: "active",
              activity_confidence: "found",
              activity_words: "Active this week",
              keychain_bound: false,
            },
            signals: ["home-pattern"],
          },
        ],
        probes: [],
        seeds: [],
        desktop_apps: [],
        cancelled: false,
      };
      w.__AIJ_RECOMMENDATION__ = rec;
      Object.assign(w, extra);
      localStorage.setItem("aij.theme", themeName);
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: honestEmpty, rec, theme, extra: extraWindows },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  await page.getByTestId("install-continue").click();
  await expect(page.getByTestId("proposal-teaser")).toBeVisible();
};

for (const theme of THEMES) {
  for (const vp of WIZARD_VP) {
    test(`recommend — teaser + customise — ${vp.name} — ${theme}`, async ({ page }) => {
      // Customise shows the all-signed-in set AND the collapsed
      // "Found but quiet" group (dormant Lab, default OFF).
      const allSignedIn = {
        items: [
          ...WORKED_EXAMPLE_RECOMMENDATION.items.slice(0, 3),
          WORKED_EXAMPLE_RECOMMENDATION.items[3],
          {
            provider: "claude-code",
            label: "Lab",
            action: "import",
            rationale: "Dormant — include when you want it",
            default_on: false,
            path: "/Users/you/.claude-lab",
          },
        ],
        summary:
          "Your juggling set: Claude ×3 (Work — signed in; Personal — signed in; Side — signed in) + Codex ×1 (signed in).",
        start_here: "-users-dev-api",
      };
      await openProposalDetect(page, theme, vp, {}, allSignedIn);
      await expect(page.getByTestId("proposal-summary")).toContainText(
        "Your juggling set: Claude ×3",
      );
      await page.screenshot({
        path: resolve(GALLERY, `36-recommend/proposal-${vp.name}-${theme}.png`),
        fullPage: true,
      });
      await page.getByTestId("proposal-customise").click();
      const block = page.getByTestId("proposal-block");
      await expect(block).toBeVisible();
      await expect(page.getByTestId("proposal-toggle")).toHaveCount(5);
      // The dormant item rides under the quiet group, unchecked.
      const quiet = page.getByTestId("proposal-quiet");
      await expect(quiet).toContainText("Found but quiet");
      await expect(quiet.getByTestId("proposal-toggle")).toHaveCount(1);
      await expect(quiet.getByTestId("proposal-toggle")).not.toBeChecked();
      await expect(page.getByTestId("accept-setup")).toContainText("Accept setup → (4)");
      await page.screenshot({
        path: resolve(GALLERY, `36-recommend/customise-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`recommend — accept in progress (per-item states) — ${vp.name} — ${theme}`, async ({
      page,
    }) => {
      await openProposalDetect(page, theme, vp, { __AIJ_CREATE_HANG__: true });
      await page.getByTestId("use-recommended").click();
      await answerPolicy(page);
      await page.getByTestId("accept-setup").click();
      await expect(page.getByTestId("accept-progress")).toBeVisible();
      await expect(page.getByTestId("accept-item")).toHaveCount(4);
      // The three imports resolve; the add hangs — the list freezes reviewable.
      await expect(
        page.locator("[data-testid='accept-item'][data-state='done']"),
      ).toHaveCount(3);
      await expect(
        page.locator("[data-testid='accept-item'][data-state='pending']"),
      ).toHaveCount(1);
      await page.screenshot({
        path: resolve(GALLERY, `36-recommend/accept-progress-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`recommend — one item fails, the rest complete (0 aborts) — ${vp.name} — ${theme}`, async ({
      page,
    }) => {
      await openProposalDetect(page, theme, vp, {
        __AIJ_IMPORT_FAIL_PATHS__: ["personal"],
      });
      await page.getByTestId("use-recommended").click();
      await answerPolicy(page);
      await page.getByTestId("accept-setup").click();
      // AJ-E305: the three logged_out successes queue for sign-ins —
      // Skip walks through (never blocks the Summary).
      await expect(page.getByTestId("signin-queue")).toBeVisible();
      for (let card = 0; card < 3; card += 1) {
        await page.getByTestId("signin-skip").click();
      }
      const summary = page.getByTestId("set-summary");
      await expect(summary).toBeVisible();
      // All 4 picked items appear — a failure never aborts the run.
      await expect(page.getByTestId("set-summary-item")).toHaveCount(4);
      await expect(
        page.locator("[data-testid='set-summary-item'][data-state='done']"),
      ).toHaveCount(3);
      const failed = page.locator(
        "[data-testid='set-summary-item'][data-state='failed']",
      );
      await expect(failed).toHaveCount(1);
      await expect(failed).toContainText("Personal");
      await expect(failed).toContainText("Disk full");
      await expect(page.getByTestId("start-here")).toContainText("-users-dev-api");
      await page.screenshot({
        path: resolve(GALLERY, `36-recommend/set-summary-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });
  }
}

/**
 * AJ-E305 gallery 37 — the two flows end to end. Flow (a)'s five
 * screens (gate → scan+proposal → confirm → queued sign-ins →
 * summary), the zero-found collapse, flow (b)'s two steps (tiles →
 * preselected account), and the no-Account inline detour — at
 * 1280×800, 1024×768, 390×844 × light+dark.
 */
const FLOW_VP = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x768", width: 1024, height: 768 },
  { name: "390x844", width: 390, height: 844 },
] as const;

const openFirstInstall = async (
  page: import("@playwright/test").Page,
  theme: string,
  vp: { width: number; height: number },
) => {
  await page.addInitScript(
    ({ fixture, theme: themeName }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROVIDER_INSTALLS__ = [
        {
          provider: "claude-code",
          display_name: "Claude Code",
          install_url: "https://claude.com/download",
          status: "installed",
          version: "2.1.296",
          binary: "/usr/local/bin/claude",
          notes: [],
        },
      ];
      w.__AIJ_RECOMMENDATION__ = {
        items: [
          {
            provider: "claude-code",
            label: "Work",
            action: "import",
            rationale: "Signed in — import keeps history",
            default_on: true,
            path: "/Users/you/.claude-work",
          },
          {
            provider: "claude-code",
            label: "Side",
            action: "import",
            rationale: "Active — you'll sign in after import",
            default_on: true,
            path: "/Users/you/.claude-side",
          },
        ],
        summary:
          "Your juggling set: Claude ×2 (Work — signed in; Side — needs sign-in).",
        start_here: "/Users/you/dev-api",
      };
      localStorage.setItem("aij.theme", themeName);
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: honestEmpty, theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  // The first-run wizard opens itself (the Accounts entry is the
  // LOCKED flow (d) since AJ-E306 — no Step-0 gate there).
  await page.goto("/");
  await expect(page.getByTestId("wizard-step-install")).toBeVisible();
};

for (const theme of THEMES) {
  for (const vp of FLOW_VP) {
    test(`flow a — all 5 screens — ${vp.name} — ${theme}`, async ({ page }) => {
      await openFirstInstall(page, theme, vp);
      // 1: the gate.
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/a1-gate-${vp.name}-${theme}.png`),
        fullPage: true,
      });
      await page.getByTestId("install-continue").click();
      // 2: scan + proposal.
      await expect(page.getByTestId("proposal-teaser")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/a2-proposal-${vp.name}-${theme}.png`),
        fullPage: true,
      });
      // 3: confirm (customise — the pre-filled identity).
      await page.getByTestId("proposal-customise").click();
      await expect(page.getByTestId("proposal-block")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/a3-confirm-${vp.name}-${theme}.png`),
        fullPage: true,
      });
      await answerPolicy(page);
      await page.getByTestId("accept-setup").click();
      // 4: the queued sign-ins.
      await expect(page.getByTestId("signin-queue")).toBeVisible();
      await expect(page.getByTestId("signin-queue")).toContainText("1 of 2");
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/a4-signins-${vp.name}-${theme}.png`),
        fullPage: true,
      });
      for (let card = 0; card < 2; card += 1) {
        await page.getByTestId("signin-skip").click();
      }
      // 5: the summary with its three exits.
      await expect(page.getByTestId("summary-exits")).toBeVisible();
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/a5-summary-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow a — zero-found collapse — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, theme: themeName }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_PROVIDER_INSTALLS__ = [
            {
              provider: "claude-code",
              display_name: "Claude Code",
              install_url: "https://claude.com/download",
              status: "installed",
              version: "2.1.296",
              binary: "/usr/local/bin/claude",
              notes: [],
            },
            {
              provider: "codex",
              display_name: "Codex",
              install_url: "https://developers.openai.com/codex",
              status: "not_installed",
              version: null,
              binary: null,
              notes: [],
            },
          ];
          w.__AIJ_SCAN_REPORT__ = {
            installs: [],
            probes: [],
            seeds: [],
            desktop_apps: [],
            cancelled: false,
          };
          w.__AIJ_RECOMMENDATION__ = { items: [], summary: "", start_here: null };
          localStorage.setItem("aij.theme", themeName);
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: honestEmpty, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      // First-run wizard (unlocked — the picker must show here).
      await page.goto("/");
      await page.getByTestId("install-continue").click();
      await expect(page.getByTestId("zero-found")).toBeVisible();
      await expect(page.getByTestId("zero-found")).toContainText("Create your first account");
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/zero-found-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

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
      await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
      await page.keyboard.press("Meta+n");
      await expect(page.getByRole("heading", { name: "New project" })).toBeVisible();
      // Step 1: the tiles.
      await page.screenshot({
        path: resolve(GALLERY, `37-flows/b1-tiles-${vp.name}-${theme}.png`),
        fullPage: true,
      });
      await page
        .getByRole("button", { name: /Work in ai-juggler — last used with Work Max/ })
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
      await page.keyboard.press("Meta+n");
      await expect(page.getByRole("heading", { name: "New project" })).toBeVisible();
      await page.getByRole("button", { name: /Work in ai-juggler/ }).click();
      await expect(page.getByTestId("no-account-detour")).toBeVisible();
      await page.getByRole("button", { name: "Add an account" }).click();
      await expect(page.getByTestId("onboarding-wizard")).toBeVisible();
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
      await page.getByRole("button", { name: "Accounts" }).click();
      await page.getByTestId("add-provider").click();
      const cards = page.getByTestId("provider-cards");
      await expect(cards).toBeVisible();
      await expect(cards).toContainText("Verified against 2.0.x, 2.1.x");
      await expect(cards).toContainText("Unverified — features may drift");
      await expect(cards).toContainText("one Instance at a time");
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
      await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
      await page.getByRole("button", { name: /^New$/ }).first().click();
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
      await page.getByRole("button", { name: /^New$/ }).first().click();
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
 * AJ-E306's successor gallery 39 — AJ-E307 flow (f): the
 * Confirm-source panel (normal / oversize / non-config), the
 * duplicate block, and the vanished-source state — 1280×800 +
 * 390×844 × light+dark.
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

const openConfirmSource = async (
  page: import("@playwright/test").Page,
  theme: string,
  vp: { width: number; height: number },
  inspect: unknown,
) => {
  await page.addInitScript(
    ({ fixture, inspect: probe, theme: t }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROVIDER_INSTALLS__ = [
        {
          provider: "claude-code",
          display_name: "Claude Code",
          install_url: "https://claude.com/download",
          status: "installed",
          version: "2.1.296",
          binary: "/usr/local/bin/claude",
          notes: [],
        },
      ];
      w.__AIJ_SCAN_REPORT__ = {
        installs: [
          {
            harness: "claude-code",
            path: "/Users/you/.claude-work",
            origin: "home-pattern",
            login: "logged_in",
            bytes: 9_000_000,
            last_used_secs: 86_400,
            managed: false,
            notes: [],
            inference: {
              purpose: "Work",
              purpose_confidence: "found",
              login: "signed-in",
              login_confidence: "found",
              activity: "active",
              activity_confidence: "found",
              activity_words: "Active this week",
              keychain_bound: true,
            },
            signals: ["home-pattern"],
          },
        ],
        probes: [],
        seeds: [],
        desktop_apps: [],
        cancelled: false,
      };
      w.__AIJ_RECOMMENDATION__ = { items: [], summary: "", start_here: null };
      w.__AIJ_SOURCE_INSPECT__ = probe;
      localStorage.setItem("aij.theme", t);
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: honestEmpty, inspect, theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  await page.getByTestId("install-continue").click();
  await expect(page.getByTestId("candidate-card").first()).toBeVisible();
  await page.getByTestId("import-as").first().click();
  await expect(page.getByTestId("confirm-source")).toBeVisible();
};

for (const theme of THEMES) {
  for (const vp of CDE_VP) {
    test(`flow f — confirm-source normal — ${vp.name} — ${theme}`, async ({ page }) => {
      await openConfirmSource(page, theme, vp, NORMAL_INSPECT);
      const panel = page.getByTestId("confirm-source");
      await expect(panel).toContainText("projects/");
      await expect(panel).toContainText("macOS Keychain");
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
      await expect(page.getByTestId("import-settings-only")).toBeVisible();
      await expect(page.getByTestId("add-fresh-instead")).toBeVisible();
      await expect(page.getByTestId("confirm-source")).not.toContainText("ceiling");
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
      await page.screenshot({
        path: resolve(GALLERY, `39-import/f-non-config-${vp.name}-${theme}.png`),
        fullPage: true,
      });
    });

    test(`flow f — duplicate block with [Open it] — ${vp.name} — ${theme}`, async ({ page }) => {
      await page.addInitScript(
        ({ fixture, inspect: probe, theme: t }) => {
          const w = window as unknown as Record<string, unknown>;
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_PROVIDER_INSTALLS__ = [
            {
              provider: "claude-code",
              display_name: "Claude Code",
              install_url: "https://claude.com/download",
              status: "installed",
              version: "2.1.296",
              binary: "/usr/local/bin/claude",
              notes: [],
            },
          ];
          w.__AIJ_SCAN_REPORT__ = {
            installs: [],
            probes: [],
            seeds: [],
            desktop_apps: [],
            cancelled: false,
          };
          w.__AIJ_RECOMMENDATION__ = { items: [], summary: "", start_here: null };
          w.__AIJ_SOURCE_INSPECT__ = probe;
          // The manual-folder entry point with a duplicate rejection.
          w.__AIJ_IMPORT_DUPLICATE_LABEL__ = "Work";
          w.__AIJ_PICK_DIRECTORY__ = "/Users/you/.claude-manual";
          localStorage.setItem("aij.theme", t);
          sessionStorage.setItem("aij.ready.skipped", "1");
        },
        { fixture: honestEmpty, inspect: NORMAL_INSPECT, theme },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto("/");
      await page.getByTestId("install-continue").click();
      await expect(page.getByTestId("zero-found")).toBeVisible();
      await page.getByTestId("import-specific-folder").click();
      await expect(page.getByTestId("confirm-source")).toBeVisible();
      // The folder label arrives prefilled; attempt the import.
      await answerPolicy(page);
      await page.getByRole("button", { name: "Import account" }).click();
      await expect(page.getByTestId("duplicate-blocked")).toBeVisible();
      await expect(page.getByTestId("duplicate-blocked")).toContainText("Already imported");
      await page.screenshot({
        path: resolve(GALLERY, `39-import/f-duplicate-${vp.name}-${theme}.png`),
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
            // Ready Home IS the surface — do not skip it.
            localStorage.setItem("aij.theme", t);
          },
          { fixture: readyHome, state, theme },
        );
        await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.goto("/");
        // Ready Home lists the not-signed account; expanding its
        // card renders the shared queue card in the probed state.
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

/** The isolation rows (pass + one forced fail) for the Summary. */
const isolationRows = (fail: boolean) => [
  { id: "managed_dir_private", label: "Account folder is owner-only (0700)", pass: true, detail: "checked" },
  { id: "marker_present", label: "The managed-folder marker is present", pass: true, detail: "checked" },
  { id: "launch_env_managed_only", label: "Launch environment points only at this account's folder", pass: true, detail: "checked" },
  { id: "source_free_of_juggler_writes", label: "The imported-from folder is untouched (no AI Juggler writes)", pass: true, detail: "checked" },
  { id: "probe_independence", label: "Login state reads only this account's folder", pass: true, detail: "checked" },
  {
    id: "no_writes_outside_managed",
    label: "No stray files outside managed folders",
    pass: !fail,
    detail: fail ? "stray.txt found at the accounts root" : "checked",
  },
];

for (const theme of THEMES) {
  for (const vp of CDE_VP) {
    for (const fail of [false, true]) {
      test(`isolation checklist — ${fail ? "forced fail blocks Launch" : "6/6 pass"} — ${vp.name} — ${theme}`, async ({ page }) => {
        await page.addInitScript(
          ({ fixture, rows, theme: t }) => {
            const w = window as unknown as Record<string, unknown>;
            w.__AIJ_FIXTURE__ = fixture;
            w.__AIJ_PROVIDER_INSTALLS__ = [
              {
                provider: "claude-code",
                display_name: "Claude Code",
                install_url: "https://claude.com/download",
                status: "installed",
                version: "2.1.296",
                binary: "/usr/local/bin/claude",
                notes: [],
              },
            ];
            w.__AIJ_RECOMMENDATION__ = {
              items: [
                {
                  provider: "claude-code",
                  label: "Work",
                  action: "import",
                  rationale: "Signed in — import keeps history",
                  default_on: true,
                  path: "/Users/you/.claude-work",
                },
              ],
              summary: "Your juggling set: Claude ×1 (Work — signed in).",
              start_here: null,
            };
            w.__AIJ_ISOLATION__ = rows;
            w.__AIJ_LOGIN_STATE__ = "signed_in";
            localStorage.setItem("aij.theme", t);
            sessionStorage.setItem("aij.ready.skipped", "1");
          },
          { fixture: honestEmpty, theme, rows: isolationRows(fail) },
        );
        await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.goto("/");
        await page.getByTestId("install-continue").click();
        await expect(page.getByTestId("proposal-teaser")).toBeVisible();
        await page.getByTestId("use-recommended").click();
        await answerPolicy(page);
        await page.getByTestId("accept-setup").click();
        // The queued card's poll reads the injected signed-in state
        // and AUTO-ADVANCES to the Summary (Req.2 — "we detect it").
        await expect(page.getByTestId("isolation-checklist")).toBeVisible({ timeout: 15_000 });
        const failing = page.locator("[data-testid='isolation-row'][data-pass='false']");
        if (fail) {
          await expect(failing).toHaveCount(1);
          await expect(failing).toContainText("stray.txt");
          await expect(page.getByTestId("launch-first-session")).toBeDisabled();
        } else {
          await expect(failing).toHaveCount(0);
          await expect(page.getByTestId("launch-first-session")).toBeEnabled();
        }
        await page.screenshot({
          path: resolve(
            GALLERY,
            `40-login/summary-${fail ? "fail" : "pass"}-${vp.name}-${theme}.png`,
          ),
          fullPage: true,
        });
      });
    }
  }
}

// ---------------------------------------------------------------------
// AJ-E309 — 41-policy: the first-run pack-policy question (unchosen /
// chosen / Decide later), the Confirm row with and without a renewal
// date, [Expert setup…] open, the Summary restatement, and Accounts →
// Pack secrets — at 1280×800 and 390×844, both themes.
// ---------------------------------------------------------------------

const POLICY_INSTALLS = [
  {
    provider: "claude-code",
    display_name: "Claude Code",
    install_url: "https://claude.com/download",
    status: "installed",
    version: "2.1.296",
    binary: "/usr/local/bin/claude",
    notes: [],
  },
];

const POLICY_RECOMMENDATION = {
  items: [
    { provider: "claude-code", label: "Work", action: "import", rationale: "Signed in — import keeps history", default_on: true, path: "/Users/you/.claude-work" },
    { provider: "claude-code", label: "Side", action: "import", rationale: "Active — you'll sign in after import", default_on: true, path: "/Users/you/.claude-side" },
  ],
  summary: "Your juggling set: Claude ×2 (Work — signed in; Side — needs sign-in).",
  start_here: "/Users/you/dev-api",
};

const openPolicyRun = async (
  page: import("@playwright/test").Page,
  theme: string,
  vp: { width: number; height: number },
  recommendation: unknown = POLICY_RECOMMENDATION,
) => {
  await page.addInitScript(
    ({ fixture, installs, rec, t }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROVIDER_INSTALLS__ = installs;
      w.__AIJ_RECOMMENDATION__ = rec;
      w.__AIJ_SCAN_REPORT__ = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false };
      w.__AIJ_LOGIN_STATE__ = "signed_in";
      localStorage.setItem("aij.theme", t);
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: honestEmpty, installs: POLICY_INSTALLS, rec: recommendation, t: theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  await page.getByTestId("install-continue").click();
};

const shot41 = (page: import("@playwright/test").Page, name: string) =>
  page.screenshot({ path: resolve(GALLERY, `41-policy/${name}.png`), fullPage: true });

for (const theme of THEMES) {
  for (const vp of CDE_VP) {
    test(`41 policy — unchosen (Accept waits) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openPolicyRun(page, theme, vp);
      await page.getByTestId("use-recommended").click();
      await expect(page.getByTestId("policy-panel")).toBeVisible();
      await expect(page.locator("[data-testid='policy-panel'] input:checked")).toHaveCount(0);
      await expect(page.getByTestId("accept-setup")).toBeDisabled();
      await page.getByTestId("policy-panel").scrollIntoViewIfNeeded();
      await shot41(page, `policy-unchosen-${vp.name}-${theme}`);
    });

    test(`41 policy — chosen (Strip) — ${vp.name} — ${theme}`, async ({ page }) => {
      await openPolicyRun(page, theme, vp);
      await page.getByTestId("use-recommended").click();
      await answerPolicy(page, "strip");
      await expect(page.getByTestId("accept-setup")).toBeEnabled();
      await shot41(page, `policy-chosen-${vp.name}-${theme}`);
    });

    test(`41 policy — Decide later — ${vp.name} — ${theme}`, async ({ page }) => {
      await openPolicyRun(page, theme, vp);
      await page.getByTestId("use-recommended").click();
      await answerPolicy(page, "decide_later");
      await expect(page.getByTestId("accept-setup")).toBeEnabled();
      await shot41(page, `policy-decide-later-${vp.name}-${theme}`);
    });

    test(`41 policy — Confirm row, no renewal date — ${vp.name} — ${theme}`, async ({ page }) => {
      await openPolicyRun(page, theme, vp, { items: [], summary: "", start_here: null });
      await page.getByTestId("zero-found-provider").first().click();
      await expect(page.getByTestId("renewal-date")).toHaveAttribute("data-state", "unknown");
      await expect(page.getByTestId("create-account")).toBeDisabled();
      await shot41(page, `confirm-renewal-empty-${vp.name}-${theme}`);
    });

    test(`41 policy — Confirm row with a renewal date — ${vp.name} — ${theme}`, async ({ page }) => {
      await openPolicyRun(page, theme, vp, { items: [], summary: "", start_here: null });
      await page.getByTestId("zero-found-provider").first().click();
      await page.getByTestId("renewal-date-add").click();
      await page.getByTestId("renewal-date-input").fill("2026-11-30");
      await answerPolicy(page, "strip");
      await expect(page.getByTestId("create-account")).toBeEnabled();
      await shot41(page, `confirm-renewal-set-${vp.name}-${theme}`);
    });

    test(`41 policy — Expert setup open — ${vp.name} — ${theme}`, async ({ page }) => {
      await openPolicyRun(page, theme, vp);
      await expect(page.getByTestId("expert-hatch")).toBeVisible();
      await page.getByTestId("expert-hatch").click();
      await expect(page.getByTestId("expert-state-dir")).toBeVisible();
      await page.getByTestId("expert-panel").scrollIntoViewIfNeeded();
      await shot41(page, `expert-open-${vp.name}-${theme}`);
    });

    test(`41 policy — Summary restatement — ${vp.name} — ${theme}`, async ({ page }) => {
      await openPolicyRun(page, theme, vp);
      await page.getByTestId("use-recommended").click();
      await page.getByTestId("renewal-date-add").first().click();
      await page.getByTestId("renewal-date-input").fill("2026-11-30");
      await answerPolicy(page, "strip");
      await page.getByTestId("accept-setup").click();
      await expect(page.getByTestId("summary-exits")).toBeVisible({ timeout: 15_000 });
      await expect(page.getByTestId("policy-restatement")).toHaveText(
        "Packs: strip secrets (all Providers)",
      );
      await expect(page.getByTestId("renewal-restatement").nth(1)).toHaveText("Side: expiry unknown");
      await shot41(page, `summary-restated-${vp.name}-${theme}`);
    });

  }
}

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
      await page.getByRole("button", { name: "Accounts" }).first().click();
      await expect(page.getByTestId("pack-secrets")).toBeVisible();
      await expect(page.getByTestId("pack-secrets-unset")).toBeVisible();
      await page.getByRole("button", { name: /Edit launch settings/ }).first().click();
      await page.getByTestId("recipe-env-toggle").first().click();
      await expect(page.getByTestId("env-overlay-editor")).toBeVisible();
      await shot41(page, `accounts-pack-secrets-${vp.name}-${theme}`);
    });
  }
}
