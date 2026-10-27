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
 * `test-results/`, which CI uploads and summarises. Comparisons are soft:
 * a screen that differs doesn't hide the ones after it in its test, so
 * one run leaves the actual of every capture that changed.
 *
 * Run: `just visual-gate` (or `visual-baselines` to approve changes,
 * with a per-screen note in the PR — scripts/check_baseline_notes.py).
 */
import { type Page } from "@playwright/test";
import { expect, GATE_NOW, settle, test } from "./axe-gallery";
import { allStatuses, goTo, projectLens, railViewAs, settingsPane } from "./fixtures";
import { placesConversations } from "../../src/preview/places";

/** The instant the harness pins the clock to (axe-gallery.ts). */
const T0 = GATE_NOW;
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
    ({ fixture, t, d, selftest, firstRun, seams }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      for (const [key, value] of Object.entries(seams)) w[key] = value;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.density", d);
      if (!firstRun) {
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      }
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
      selftest: process.env.AIJ_VISUAL_SELFTEST === "1",
      firstRun: options.firstRun ?? false,
      seams: options.seams ?? {},
    },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  if (!options.firstRun) await expect(page.getByRole("button", { name: "Settings" }).first()).toBeVisible();
  await settle(page);
}

for (const theme of THEMES) {
  for (const density of DENSITIES) {
    for (const vp of VIEWPORTS) {
      const cell = `${theme}-${density}-${vp.name}`;

      test(`rail + home — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`home-${cell}.png`, SCREEN);
        // AJ-E2104: the rail's search and View.
        if (density !== "minimal") await expect.soft(page.getByTestId("rail-toolbar")).toHaveScreenshot(`chrome-rail-toolbar-${cell}.png`, CHROME);
        await settle(page);
        await expect.soft(page.getByRole("button", { name: "Settings" }).first()).toHaveScreenshot(
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
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`home-scrolled-${cell}.png`, SCREEN);
      });

      test(`inspector — Instance overview — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page.getByTestId("instance-board")).toBeVisible(); // D-25: one rail, always the list
        await page.getByRole("button", { name: /Work Max/ }).first().click();
        // AJ-E1533: a session's page opens on its body: its terminal, its Overview.
        await expect(page.getByTestId("page-tab-terminal")).toHaveAttribute("aria-selected", "true");
        // The terminal loads on first use (#173): a slow runner shot its
        // empty pane (#175).
        await expect(page.getByTestId("terminal-toolbar")).toBeVisible();
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`inspector-instance-${cell}.png`, SCREEN);
      });

      // AJ-E803 (wider coverage): the views and sheets the D-25 rebuild added or changed,
      // so a regression there fails the gate, not only the gallery.
      test(`inspector — tool view — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page.getByTestId("instance-board")).toBeVisible();
        await page.getByRole("button", { name: /Work Max/ }).first().click();
        await page.getByRole("button", { name: "About Claude Code" }).click();
        // 0.1.5: the tool's view is Settings › Accounts' Tools on this Mac.
        await expect(page.getByTestId("settings-tool").first()).toBeVisible();
        // It opens on that section once its rows are drawn (the jump
        // focuses the section's title), and with what's installed read.
        await expect(page.locator("#settings-tools-title")).toBeFocused();
        await expect(page.getByTestId("settings-tool").first()).not.toContainText("checking what's installed");
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`inspector-tool-${cell}.png`, SCREEN);
      });

      test(`inspector — account view — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await goTo(page, "Accounts");
        await page.getByTestId("profile-open").first().click();
        await expect(page.getByTestId("account-overview")).toBeVisible();
        await page.waitForLoadState("networkidle");
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`inspector-account-${cell}.png`, SCREEN);
      });

      test(`inspector — project view — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        // AJ-E2104: projects' headers are the Project lens's (the default
        // groups Open now by place).
        await projectLens(page);
        await expect(page.getByTestId("instance-board")).toBeVisible();
        await page.getByTestId("rail").getByRole("button", { name: /^ai-juggler/ }).first().click();
        // AJ-E1533: a project's page opens on its body, its sessions.
        await expect(page.getByTestId("page-panel-sessions").getByTestId("page-session-row").first()).toBeVisible();
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`inspector-project-${cell}.png`, SCREEN);
      });

      test(`add account — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, allStatuses, { seams: { __AIJ_SCAN_REPORT__: EMPTY_SCAN } });
        await goTo(page, "Accounts");
        await page.getByTestId("add-account").click();
        await expect(page.getByTestId("first-run-fresh-name")).toHaveValue(/^Claude Code/);
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`add-account-${cell}.png`, SCREEN);
      });

      test(`limit dialog — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.getByTestId("home-needs").getByRole("button", { name: /^Continue elsewhere/ }).first().click();
        const dialog = page.getByTestId("limit-dialog");
        await expect(dialog).toBeVisible();
        await expect(dialog.getByRole("button", { name: /^Continue on / })).toBeVisible();
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`limit-dialog-${cell}.png`, SCREEN);
      });

      test(`new session — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
        await expect(page.getByTestId("new-session-sheet")).toBeVisible();
        await page.waitForLoadState("networkidle");
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`new-session-${cell}.png`, SCREEN);
      });

      test(`first run — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, { accounts: [], profiles: [], sessions: [], projects: [] }, {
          firstRun: true,
          seams: { __AIJ_SCAN_REPORT__: FIRST_RUN_SCAN },
        });
        await expect(page.getByTestId("first-run-found")).toBeVisible();
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`first-run-${cell}.png`, SCREEN);
      });

      test(`legend — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.getByTestId("rail").getByRole("button", { name: "Indicator legend" }).click();
        const legend = page.getByTestId("indicator-legend").first();
        await expect(legend).toBeVisible();
        await settle(page);
        await expect.soft(legend).toHaveScreenshot(`chrome-legend-${cell}.png`, SCREEN);
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
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`toast-stop-${cell}.png`, SCREEN);
      });

      test(`quit sheet — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.evaluate(() =>
          window.dispatchEvent(new CustomEvent("aij:quit-requested", { detail: { live: 3, end_on_quit: 3, keep_running: 0 } })),
        );
        await expect(page.getByTestId("quit-sheet")).toBeVisible();
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`quit-sheet-${cell}.png`, SCREEN);
      });

      test(`loading placeholders — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, allStatuses, { seams: { __AIJ_LOAD_HANG__: true } });
        await expect(page.getByTestId("home-loading")).toBeVisible();
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`loading-home-${cell}.png`, SCREEN);
        await goTo(page, "Accounts");
        await expect(page.getByTestId("placeholder-card")).toBeVisible();
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`loading-accounts-${cell}.png`, SCREEN);
      });

      // AJ-E1524: the welcome Home for a new install (no accounts).
      test(`welcome — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp, { accounts: [], profiles: [], sessions: [], projects: [] });
        await expect(page.getByTestId("welcome-found")).not.toContainText("Looking");
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`welcome-${cell}.png`, SCREEN);
      });

      // AJ-E1505: Settings › Sessions, the Terminal disclosure open.
      test(`settings sessions — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await goTo(page, "Settings");
        await await settingsPane(page, "Sessions");
        await page.getByTestId("settings-terminal").locator("summary").click();
        await expect(page.getByLabel("Scrollback")).toBeVisible();
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page.getByTestId("settings-sessions")).toHaveScreenshot(`settings-sessions-${cell}.png`, SCREEN);
      });

      // AJ-E1504: the ? sheet and the folded inspector.
      test(`shortcut sheet — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await page.keyboard.press("Shift+Slash");
        await expect(page.getByTestId("shortcut-sheet")).toBeVisible();
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`shortcut-sheet-${cell}.png`, SCREEN);
      });

      test(`inspector folded — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page.getByTestId("instance-board")).toBeVisible();
        await page.locator('[data-instance-id="sess-2"]').first().click();
        await page.keyboard.press("Meta+Shift+I");
        await expect(page.getByTestId("right-inspector")).toHaveAttribute("data-folded", "true");
        // The terminal loads on first use (#173): without this the shot
        // raced it, the empty pane on some runs and the terminal on others.
        await expect(page.getByTestId("terminal-toolbar")).toBeVisible();
        await expect(page.locator(".xterm-rows")).toContainText("Ask anything");
        await page.mouse.move(0, 0);
        await settle(page);
        await expect.soft(page).toHaveScreenshot(`inspector-folded-${cell}.png`, SCREEN);
      });

      // D-25 amendment (AJ-E1021): Triage and Overview folded into Home.
      // AJ-E1528: This Mac, the page.
      // AJ-E2404: Processes took Cleanup's place.
      for (const screen of ["History", "Accounts", "Usage", "Fleet", "Processes", "Settings", "This Mac"] as const) {
        test(`${screen.toLowerCase()} — ${cell}`, async ({ page }) => {
          await open(page, theme, density, vp);
          await goTo(page, screen); // D-25: some screens sit under More
          await page.waitForLoadState("networkidle");
          // Screens past Home load on first use (lib/lazy): the shot raced
          // the screen's code and caught a blank page on most runs.
          await expect(page.getByRole("main")).toHaveText(/\S/);
          await page.waitForLoadState("networkidle");
          await page.mouse.move(0, 0);
          await settle(page);
          await expect.soft(page).toHaveScreenshot(`${screen.toLowerCase().replace(" ", "-")}-${cell}.png`, SCREEN);
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
    await page.getByRole("button", { name: "Profile actions for personal-max", exact: true }).click();
    await expect(page.getByRole("menu", { name: "Actions for personal-max" })).toBeVisible();
    await page.mouse.move(0, 0);
    await settle(page);
    await expect.soft(page).toHaveScreenshot(`account-menu-${cell}.png`, SCREEN);
  });

  test(`remove account — a copy — ${cell}`, async ({ page }) => {
    await open(page, theme, "comfortable", VIEWPORTS[0], MANAGE);
    await goTo(page, "Accounts");
    await page.getByRole("button", { name: "Profile actions for personal-max", exact: true }).click();
    await page.getByRole("menuitem", { name: "Remove…" }).click();
    await expect(page.getByTestId("remove-account")).toHaveAttribute("data-kind", "copy");
    await page.mouse.move(0, 0);
    await settle(page);
    await expect.soft(page).toHaveScreenshot(`remove-account-copy-${cell}.png`, SCREEN);
  });
}

// AJ-E1502: a named, pinned session leads its group, a pinned project's
// group leads the rail; a project's row menu. AJ-E2104: by place (the
// default), the pinned session stays under its place once ended, first
// and marked; in the Project lens, the pinned project still leads.
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
    const rail = page.getByTestId("rail");
    const pinned = rail.locator('[data-instance-id="sess-6"]');
    await expect(pinned).toContainText("the login fix");
    // A row is two layers (its page's button, and the visible line over
    // it): the mark is on the line, the row's other child.
    await expect(rail.locator("[data-rail-row]").filter({ has: page.locator('[data-instance-id="sess-6"]') }).getByTestId("pin-mark")).toBeVisible();
    await expect(pinned).toHaveAccessibleName(/^Ended, the login fix, pinned, /);
    // Under AI Juggler's place in Open now, its first row — not folded
    // into "Also: n ended today".
    const lead = await page.getByTestId("place-board").evaluate((board) => {
      const lines = [...board.querySelectorAll<HTMLElement>('[data-testid="place-header"], [data-rail-item]')];
      const header = lines.findIndex((line) => line.dataset.place === "ai-juggler");
      return header < 0 ? null : (lines[header + 1]?.dataset.instanceId ?? null);
    });
    expect(lead, "the pinned session leads its place").toBe("sess-6");
    await page.mouse.move(0, 0);
    await settle(page);
    await expect.soft(rail).toHaveScreenshot(`rail-pins-${cell}.png`, SCREEN);

    await projectLens(page);
    await expect(rail.getByTestId("rail-project").first()).toContainText("storefront");
    await expect(rail.getByTestId("rail-project").first().getByTestId("pin-mark")).toBeVisible();
    await expect(rail.locator('[data-instance-id="sess-6"]')).toContainText("the login fix");
    await page.mouse.move(0, 0);
    await settle(page);
    await expect.soft(rail).toHaveScreenshot(`rail-pins-project-${cell}.png`, SCREEN);
  });

  test(`project menu — ${cell}`, async ({ page }) => {
    await open(page, theme, "comfortable", VIEWPORTS[0], PINNED);
    await projectLens(page); // AJ-E2104: a project's row is the Project lens's
    await page.getByRole("button", { name: "Project actions for ai-juggler" }).click();
    await expect(page.getByRole("menu", { name: "Actions for ai-juggler" })).toBeVisible();
    await page.mouse.move(0, 0);
    await settle(page);
    await expect.soft(page).toHaveScreenshot(`project-menu-${cell}.png`, SCREEN);
  });
}

// D-47: Open now in Your order, the default: a pinned row (one another
// app runs, pinned in AI Juggler's own record) leads its place; one that
// asks stays in its place, amber, and is in Needs you too; a row's handle
// and pin on hover; the order menu open.
const ORDER = {
  ...allStatuses,
  sessions: allStatuses.sessions.filter((session) => session.id === "sess-2" || session.id === "sess-3"),
  conversations: placesConversations(T0),
};
const ORDER_SEAMS = { __AIJ_APP_SETTINGS__: { rail_order: JSON.stringify({ mode: "yours", pins: ["c:claude-personal/outside-5"] }) } };
for (const theme of THEMES) {
  const cell = `${theme}-comfortable-1280x800`;
  test(`rail order — ${cell}`, async ({ page }) => {
    await open(page, theme, "comfortable", VIEWPORTS[0], ORDER, { seams: ORDER_SEAMS });
    const rail = page.getByTestId("rail");
    await expect(rail.getByTestId("rail-order-button")).toHaveText("Your order");
    const personal = rail.locator('[data-order-group="Claude|Personal"][data-order-row]');
    await expect(personal.first()).toHaveAttribute("data-order-row", "c:claude-personal/outside-5");
    await expect(personal.first().getByTestId("pin-mark")).toBeVisible();
    const asks = rail.locator('[data-order-row="c:claude-work/outside-3"]');
    await expect(asks.getByTestId("state-word")).toHaveText("Asks you");
    await expect(rail.getByTestId("rail-needs-you")).toContainText("4");
    await page.mouse.move(0, 0);
    await settle(page);
    await expect.soft(rail).toHaveScreenshot(`rail-order-${cell}.png`, SCREEN);

    const row = rail.locator('[data-order-row="c:claude-work/outside-1"]');
    await row.hover();
    await expect(row.getByTestId("row-tools")).toHaveCSS("opacity", "1");
    await expect(row.getByTestId("order-grip")).toBeVisible();
    await expect(row.getByTestId("row-pin")).toBeVisible();
    await settle(page);
    await expect.soft(rail).toHaveScreenshot(`rail-order-hover-${cell}.png`, SCREEN);

    await rail.getByTestId("rail-order-button").click();
    const menu = page.getByRole("menu", { name: "Order of Open now" });
    await expect(menu.getByRole("menuitemradio")).toHaveCount(3);
    await expect(menu.getByRole("menuitemradio", { checked: true })).toContainText("Your order");
    await settle(page);
    await expect.soft(page).toHaveScreenshot(`rail-order-menu-${cell}.png`, SCREEN);
  });
}

// WCAG 1.4.1: a state shown by colour must also say it in words and in
// its accessible name — a colour-only change fails even when its pixel
// diff is small. The fixture holds a session in every status (the ended
// ones ended today); with ended and quiet ones shown, every session is a
// row, so every state dot and every Needs you mark the rail draws is
// checked, in AJ-E2104's words.
const TODAY = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session, n) =>
    ["finished", "stopped", "failed", "handed_over"].includes(session.status) ? { ...session, last_activity_ms: T0 - (n + 1) * 60_000 } : session,
  ),
};
test("status is never colour-only", async ({ page }) => {
  await open(page, "dark", "comfortable", VIEWPORTS[0], TODAY);
  await railViewAs(page, { group: "place", show: { ended: true, quiet: true } });
  const rail = page.getByTestId("rail");
  await expect(rail.getByTestId("place-board")).toBeVisible();
  const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  // Open now and Ended today: a dot in the state's colour, its word beside
  // it in words (E21's vocabulary), the row's name leading with that word.
  const vocabulary = /^(Asks you|Probably asks you|Working|Idle|Done, review it|Ended)$/;
  const drawn = new Set<string>();
  const dots = await rail.locator("[data-state-dot]").all();
  // A row is its button (its name) under the line it draws (the dot and
  // the words), in one [data-rail-row].
  for (const dot of dots) {
    const line = dot.locator("xpath=ancestor::*[@data-rail-row][1]");
    const row = line.locator("[data-rail-item]");
    const word = (await line.getByTestId("state-word").innerText()).trim();
    const id = (await row.getAttribute("data-rail-item")) ?? "a row";
    expect.soft(word, `${id}: its state in words`).toMatch(vocabulary);
    await expect.soft(row, `${id}: its name says its state`).toHaveAccessibleName(new RegExp(`^${escape(word)}, `));
    drawn.add((await dot.getAttribute("data-state-dot"))!);
  }
  // Needs you: why, in amber and in words, and first in the row's name.
  const needs = await rail.getByTestId("need-word").all();
  for (const mark of needs) {
    const row = mark.locator("xpath=ancestor::*[@data-rail-row][1]").locator("[data-rail-item]");
    const word = (await mark.innerText()).trim();
    const id = (await row.getAttribute("data-rail-item")) ?? "a row";
    expect.soft(word, `${id}: why it needs you, in words`).not.toBe("");
    await expect.soft(row, `${id}: its name says why`).toHaveAccessibleName(new RegExp(`^${escape(word)}, `));
  }
  // Not vacuous: every session in the fixture is a row, and every row is
  // one of those checked (a dot or a Needs you mark); the dots include at
  // least the working and the ended state. D-47: one that needs you and
  // is still open is in Needs you and in its place, so it's drawn twice.
  const twice = new Set(["waiting_user", "stuck", "quota_blocked"]);
  for (const session of TODAY.sessions) {
    await expect.soft(rail.locator(`[data-instance-id="${session.id}"]`), `${session.id} (${session.status}) is drawn`).toHaveCount(twice.has(session.status) ? 2 : 1);
  }
  expect(await rail.locator("[data-rail-item]").count(), "every row shows its state").toBe(dots.length + needs.length);
  expect(needs.length, "Needs you rows").toBeGreaterThan(0);
  expect([...drawn].sort(), "states drawn as dots").toEqual(expect.arrayContaining(["ended", "working"]));

  // The Project lens says it the same way (AJ-E2104 Req.3): every state
  // glyph in its colour has its word beside it and in its row's name.
  await projectLens(page);
  const glyphs = await rail.locator("[data-state-glyph]").all();
  expect(glyphs.length, "the Project lens's rows").toBeGreaterThan(0);
  for (const glyph of glyphs) {
    const row = glyph.locator("xpath=ancestor::button[1]");
    const id = (await row.getAttribute("data-instance-id")) ?? "a row";
    const word = (await row.locator('[data-testid="rail-state-word"], [data-testid="rail-parked"]').innerText()).trim();
    expect.soft(word, `${id}: its state in words`).not.toBe("");
    await expect.soft(row, `${id}: its name says its state`).toHaveAccessibleName(new RegExp(escape(word)));
  }
  await expect(rail, "one vocabulary: no old state words").not.toContainText(/Running|Starting|Waiting for you|Looks stuck|Hit its limit|Handed over/);
});
