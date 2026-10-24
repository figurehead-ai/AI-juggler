/**
 * AJ-E803 — the visual regression gate.
 *
 * A deterministic matrix (mocked bridge, frozen clock, no animation)
 * compared pixel by pixel against approved baselines rendered in the
 * pinned Playwright container — the same image locally
 * (`just visual-baselines`) and in CI — so a diff is the code, not the
 * machine. Thresholds: whole screens ≤ 0.1 % differing pixels; text-
 * bearing chrome (the rail's quick filters, a nav button, a status badge)
 * 0 pixels. A failure leaves expected/actual/diff (the triptych) in
 * `test-results/`, which CI uploads and summarises.
 *
 * Run: `just visual-gate` (or `visual-baselines` to approve changes,
 * with a per-screen note in the PR — scripts/check_baseline_notes.py).
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses, goTo, settingsPane } from "./fixtures";

const T0 = 1_728_800_000_000;
const THEMES = ["dark", "light"] as const;
const DENSITIES = ["comfortable", "compact"] as const;
const VIEWPORTS = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x680", width: 1024, height: 680 },
] as const;

/** allStatuses with its limit-hit sessions parked, the reset reached. */
const PARKED = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) =>
    session.status === "quota_blocked" ? { ...session, parked: "measured", parked_resets_at_ms: T0 - 60_000 } : session,
  ),
};

const SCREEN = { maxDiffPixelRatio: 0.001, animations: "disabled", caret: "hide" } as const;
const CHROME = { maxDiffPixels: 0, animations: "disabled", caret: "hide" } as const;

/** Nothing new on this Mac: Add account opens at a fresh account. */
const EMPTY_SCAN = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false };

/** First run on a Mac that already has Claude Code: two setups found. */
const FIRST_RUN_SCAN = {
  installs: [
    ["/Users/you/.claude", "vendor-default", "Main", "Active this week"],
    ["/Users/you/Library/Application Support/ClaudeCodeProfiles/work", "manager-tool", "Work", "Dormant 12 d"],
  ].map(([path, origin, purpose, words]) => ({
    harness: "claude-code",
    path,
    origin,
    login: "logged_in",
    bytes: 30_000_000,
    last_used_secs: Math.floor(T0 / 1000) - 86_400,
    managed: false,
    notes: [],
    inference: {
      purpose,
      purpose_confidence: "likely",
      login: "signed-in",
      login_confidence: "found",
      activity: words.startsWith("Active") ? "active" : "dormant",
      activity_confidence: "found",
      activity_words: words,
      keychain_bound: false,
    },
    signals: [origin],
  })),
  probes: [
    { label: "Your standard setup (~/.claude)", health: "ok", detail: "1 found", reason: null },
    { label: "Claude desktop profiles", health: "ok", detail: "1 profile", reason: null },
  ],
  seeds: [],
  desktop_apps: ["Claude"],
  cancelled: false,
};

async function open(
  page: Page,
  theme: string,
  density: string,
  vp: { width: number; height: number },
  fixture: unknown = allStatuses,
  options: { firstRun?: boolean; seams?: Record<string, unknown> } = {},
) {
  await page.addInitScript(
    ({ fixture, t, d, t0, selftest, firstRun, seams }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      for (const [key, value] of Object.entries(seams)) w[key] = value;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.density", d);
      if (!firstRun) {
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      }
      Date.now = () => t0;
      // The gate's self-test: a seeded 2 px shift of every button must
      // fail the comparison (AJ-E803 quality gate).
      if (selftest) {
        document.addEventListener("DOMContentLoaded", () => {
          const style = document.createElement("style");
          style.textContent = "button { transform: translateX(2px); }";
          document.head.append(style);
        });
      }
    },
    {
      fixture,
      t: theme,
      d: density,
      t0: T0,
      selftest: process.env.AIJ_VISUAL_SELFTEST === "1",
      firstRun: options.firstRun ?? false,
      seams: options.seams ?? {},
    },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  if (!options.firstRun) await expect(page.getByRole("button", { name: "Settings" }).first()).toBeVisible();
}

for (const theme of THEMES) {
  for (const density of DENSITIES) {
    for (const vp of VIEWPORTS) {
      const cell = `${theme}-${density}-${vp.name}`;

      test(`rail + home — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page).toHaveScreenshot(`home-${cell}.png`, SCREEN);
        // AJ-E2104: the rail's search and View.
        if (density !== "minimal") await expect(page.getByTestId("rail-toolbar")).toHaveScreenshot(`chrome-rail-toolbar-${cell}.png`, CHROME);
        await expect(page.getByRole("button", { name: "Settings" }).first()).toHaveScreenshot(
          `chrome-nav-button-${cell}.png`,
          CHROME,
        );
      });

      // AJ-E1021: Home's lower half — parked, pick up where you left off
      // (Mark reviewed, Show all) and the keyboard line — scrolled into view.
      test(`home scrolled — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, PARKED);
        const home = page.getByTestId("home");
        await expect(home.getByTestId("home-parked")).toBeVisible();
        await home.evaluate((node) => {
          const scroller = node.parentElement!;
          scroller.scrollTop = scroller.scrollHeight;
        });
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`home-scrolled-${cell}.png`, SCREEN);
      });

      test(`inspector — Instance overview — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page.getByTestId("instance-board")).toBeVisible(); // D-25: one rail, always the list
        await page.getByRole("button", { name: /Work Max/ }).first().click();
        // AJ-E1533: a session's page opens on its body, the Terminal tab.
        await expect(page.getByRole("tab", { name: "Terminal" })).toHaveAttribute("aria-selected", "true");
        // The terminal loads on first use (#173): a slow runner shot its
        // empty pane (#175).
        await expect(page.getByTestId("terminal-toolbar")).toBeVisible();
        await expect(page).toHaveScreenshot(`inspector-instance-${cell}.png`, SCREEN);
      });

      // AJ-E803 (wider coverage): the views and sheets the D-25 rebuild added or changed,
      // so a regression there fails the gate, not only the gallery.
      test(`inspector — tool view — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page.getByTestId("instance-board")).toBeVisible();
        await page.getByRole("button", { name: /Work Max/ }).first().click();
        await page.getByRole("button", { name: "About Claude Code" }).click();
        await expect(page.getByTestId("provider-install")).toBeVisible();
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`inspector-tool-${cell}.png`, SCREEN);
      });

      test(`inspector — account view — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await goTo(page, "Accounts");
        await page.getByRole("button", { name: "Inspect Claude Code" }).first().click();
        await expect(page.getByTestId("account-overview")).toBeVisible();
        await page.waitForLoadState("networkidle");
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`inspector-account-${cell}.png`, SCREEN);
      });

      test(`inspector — project view — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page.getByTestId("instance-board")).toBeVisible();
        await page.getByTestId("rail").getByRole("button", { name: /^ai-juggler/ }).first().click();
        // AJ-E1533: a project's page opens on its body, its sessions.
        await expect(page.getByTestId("page-panel-sessions").getByTestId("page-session-row").first()).toBeVisible();
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`inspector-project-${cell}.png`, SCREEN);
      });

      test(`add account — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, allStatuses, { seams: { __AIJ_SCAN_REPORT__: EMPTY_SCAN } });
        await goTo(page, "Accounts");
        await page.getByTestId("add-account").click();
        await expect(page.getByTestId("first-run-fresh-name")).toHaveValue(/^Claude Code/);
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`add-account-${cell}.png`, SCREEN);
      });

      test(`limit dialog — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.getByTestId("home-needs").getByRole("button", { name: /^Continue elsewhere/ }).first().click();
        const dialog = page.getByTestId("limit-dialog");
        await expect(dialog).toBeVisible();
        await expect(dialog.getByRole("button", { name: /^Continue on / })).toBeVisible();
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`limit-dialog-${cell}.png`, SCREEN);
      });

      test(`new session — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
        await expect(page.getByTestId("new-session-sheet")).toBeVisible();
        await page.waitForLoadState("networkidle");
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`new-session-${cell}.png`, SCREEN);
      });

      test(`first run — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, { accounts: [], profiles: [], sessions: [], projects: [] }, {
          firstRun: true,
          seams: { __AIJ_SCAN_REPORT__: FIRST_RUN_SCAN },
        });
        await expect(page.getByTestId("first-run-found")).toBeVisible();
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`first-run-${cell}.png`, SCREEN);
      });

      test(`legend — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.getByTestId("rail").getByRole("button", { name: "Indicator legend" }).click();
        const legend = page.getByTestId("indicator-legend").first();
        await expect(legend).toBeVisible();
        await expect(legend).toHaveScreenshot(`chrome-legend-${cell}.png`, SCREEN);
      });

      // AJ-E1503: the one toast (a held Stop), the quit sheet, and the
      // loading placeholders (rail, Home, Accounts).
      test(`toast — held stop — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, allStatuses, { seams: { __AIJ_TOAST_PAUSED__: true } });
        await expect(page.getByTestId("instance-board")).toBeVisible();
        await page.locator('[data-instance-id="sess-2"]').first().click();
        await page.getByTestId("inspector-stop").click();
        await expect(page.getByTestId("stop-undo")).toContainText("Undo (5s)");
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`toast-stop-${cell}.png`, SCREEN);
      });

      test(`quit sheet — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.evaluate(() =>
          window.dispatchEvent(new CustomEvent("aij:quit-requested", { detail: { live: 3, end_on_quit: 3, keep_running: 0 } })),
        );
        await expect(page.getByTestId("quit-sheet")).toBeVisible();
        await expect(page).toHaveScreenshot(`quit-sheet-${cell}.png`, SCREEN);
      });

      test(`loading placeholders — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, allStatuses, { seams: { __AIJ_LOAD_HANG__: true } });
        await expect(page.getByTestId("home-loading")).toBeVisible();
        await expect(page).toHaveScreenshot(`loading-home-${cell}.png`, SCREEN);
        await goTo(page, "Accounts");
        await expect(page.getByTestId("placeholder-card")).toBeVisible();
        await expect(page).toHaveScreenshot(`loading-accounts-${cell}.png`, SCREEN);
      });

      // AJ-E1524: the welcome Home for a new install (no accounts).
      test(`welcome — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, { accounts: [], profiles: [], sessions: [], projects: [] });
        await expect(page.getByTestId("welcome-found")).not.toContainText("Looking");
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`welcome-${cell}.png`, SCREEN);
      });

      // AJ-E1505: Settings › Sessions, the Terminal disclosure open.
      test(`settings sessions — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await goTo(page, "Settings");
        await await settingsPane(page, "Sessions");
        await page.getByTestId("settings-terminal").locator("summary").click();
        await expect(page.getByLabel("Scrollback")).toBeVisible();
        await page.mouse.move(0, 0);
        await expect(page.getByTestId("settings-sessions")).toHaveScreenshot(`settings-sessions-${cell}.png`, SCREEN);
      });

      // AJ-E1504: the ? sheet and the folded inspector.
      test(`shortcut sheet — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.keyboard.press("Shift+Slash");
        await expect(page.getByTestId("shortcut-sheet")).toBeVisible();
        await expect(page).toHaveScreenshot(`shortcut-sheet-${cell}.png`, SCREEN);
      });

      test(`inspector folded — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page.getByTestId("instance-board")).toBeVisible();
        await page.locator('[data-instance-id="sess-2"]').first().click();
        await page.keyboard.press("Meta+Shift+I");
        await expect(page.getByTestId("right-inspector")).toHaveAttribute("data-folded", "true");
        await page.mouse.move(0, 0);
        await expect(page).toHaveScreenshot(`inspector-folded-${cell}.png`, SCREEN);
      });

      // D-25 amendment (AJ-E1021): Triage and Overview folded into Home.
      // AJ-E1528: This Mac, the page.
      for (const screen of ["History", "Accounts", "Usage", "Fleet", "Cleanup", "Settings", "This Mac"] as const) {
        test(`${screen.toLowerCase()} — ${cell}`, async ({ page }) => {
          await open(page, theme, density, vp);
          await goTo(page, screen); // D-25: some screens sit under More
          await page.waitForLoadState("networkidle");
          await page.mouse.move(0, 0);
          await expect(page).toHaveScreenshot(`${screen.toLowerCase().replace(" ", "-")}-${cell}.png`, SCREEN);
        });
      }
    }
  }
}

// AJ-E1501: the account card's menu and the remove sheet (a copy: keep
// its folder or send it to the Trash), at the main size in each theme.
const MANAGE = {
  ...allStatuses,
  accounts: [
    ...allStatuses.accounts,
    { id: "claude-max", provider: "claude-code", label: "personal-max", plan: "max-5x", login_state: "logged_in", in_place: false, import_source_path: "/Users/you/Library/Application Support/ClaudeCodeProfiles/max" },
  ],
};
for (const theme of THEMES) {
  const cell = `${theme}-comfortable-1280x800`;
  test(`account menu — ${cell}`, async ({ page }) => {
    await open(page, theme, "comfortable", VIEWPORTS[0], MANAGE);
    await goTo(page, "Accounts");
    await page.getByRole("button", { name: "Account actions for personal-max", exact: true }).click();
    await expect(page.getByRole("menu", { name: "Actions for personal-max" })).toBeVisible();
    await page.mouse.move(0, 0);
    await expect(page).toHaveScreenshot(`account-menu-${cell}.png`, SCREEN);
  });

  test(`remove account — a copy — ${cell}`, async ({ page }) => {
    await open(page, theme, "comfortable", VIEWPORTS[0], MANAGE);
    await goTo(page, "Accounts");
    await page.getByRole("button", { name: "Account actions for personal-max", exact: true }).click();
    await page.getByRole("menuitem", { name: "Remove…" }).click();
    await expect(page.getByTestId("remove-account")).toHaveAttribute("data-kind", "copy");
    await page.mouse.move(0, 0);
    await expect(page).toHaveScreenshot(`remove-account-copy-${cell}.png`, SCREEN);
  });
}

// AJ-E1502: a named, pinned session leads its group, a pinned project's
// group leads the rail; a project's row menu.
const PINNED = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) =>
    session.id === "sess-6" ? { ...session, label: "the login fix", pinned: true } : session,
  ),
  projects: allStatuses.projects.map((project) => (project.id === "proj-2" ? { ...project, pinned: true } : project)),
};
for (const theme of THEMES) {
  const cell = `${theme}-comfortable-1280x800`;
  test(`rail with pins — ${cell}`, async ({ page }) => {
    await open(page, theme, "comfortable", VIEWPORTS[0], PINNED);
    await expect(page.locator('[data-instance-id="sess-6"]')).toContainText("the login fix");
    await page.mouse.move(0, 0);
    await expect(page.getByTestId("rail")).toHaveScreenshot(`rail-pins-${cell}.png`, SCREEN);
  });

  test(`project menu — ${cell}`, async ({ page }) => {
    await open(page, theme, "comfortable", VIEWPORTS[0], PINNED);
    await page.getByRole("button", { name: "Project actions for ai-juggler" }).click();
    await expect(page.getByRole("menu", { name: "Actions for ai-juggler" })).toBeVisible();
    await page.mouse.move(0, 0);
    await expect(page).toHaveScreenshot(`project-menu-${cell}.png`, SCREEN);
  });
}

// WCAG 1.4.1: a status shown by colour must also say it in words or an
// accessible name — a colour-only change fails even when its pixel diff
// is small. The fixture holds an Instance in every status.
test("status is never colour-only", async ({ page }) => {
  await open(page, "dark", "comfortable", VIEWPORTS[0]);
  await expect(page.getByTestId("instance-board")).toBeVisible(); // D-25: one rail, always the list
  // D-25: the rail's own words for each state.
  const words = ["Starting", "Running", "Waiting for you", "Looks stuck", "Hit its limit", "Done", "Stopped", "Failed", "Handed over"];
  const missing: string[] = [];
  for (const word of words) {
    const asText = await page.getByText(word, { exact: true }).count();
    const asName = await page.getByRole("button", { name: word }).count();
    if (asText + asName === 0) missing.push(word);
  }
  expect(missing, "statuses with no words or accessible name").toEqual([]);
});
