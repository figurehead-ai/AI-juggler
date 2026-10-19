import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { allStatuses, answerPolicy, honestEmpty, type FixtureSet } from "./fixtures";

/**
 * AJ-E402 quality gate: axe-core reports 0 critical/serious violations on
 * the rail fixture page (all 9 states rendered) in BOTH themes (D-16).
 * Full a11y programme lands with AJ-E804; this is the row's entry gate.
 */

/**
 * AJ-E806 flake record (2026-10-16): under the FULL parallel suite,
 * axe color-contrast intermittently reports serious violations on the
 * kill sheet (refused) and the recovery dialog (light theme) — the
 * same tests pass in isolation and on the clean tree; the flagged
 * elements belong to dialogs whose transitions settle after the scan
 * under load. Fix: ONE confirmatory re-scan after a settle beat. A
 * real violation reproduces deterministically and still fails the
 * gate; a load-induced false positive does not survive the re-scan.
 */
async function scanSeriousWithConfirm(page: import("@playwright/test").Page) {
  const seriousOf = async () => {
    const results = await new AxeBuilder({ page }).analyze();
    return results.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
  };
  let serious = await seriousOf();
  if (serious.length > 0) {
    await page.waitForTimeout(400);
    serious = await seriousOf();
  }
  return serious;
}

const T0 = 1_728_800_000_000;

declare global {
  interface Window {
    __AIJ_FIXTURE__?: Partial<FixtureSet>;
  }
}

for (const theme of ["dark", "light"] as const) {
  test(`rail — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t }) => {
        (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
    await page.getByRole("tab", { name: "Sessions" }).click();
    await expect(page.getByText("Handed over").first()).toBeVisible();

    // AJ-E405: the badge tooltip carries the structured reason (rule +
    // measured inputs) — asserted in the DOM (native titles are not
    // screenshot-able).
    await expect(page.locator('[title*="waiting_prompt"]').first()).toBeVisible();
    await expect(page.locator('[title*="stuck_no_output"]').first()).toBeVisible();
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();
    let serious = results.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
    if (serious.length > 0) {
        // AJ-E806: confirmatory re-scan — load-induced contrast false
        // positives do not survive; real violations still fail.
        await page.waitForTimeout(400);
        const confirm = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
          .analyze();
        serious = confirm.violations.filter((v) =>
          ["serious", "critical"].includes(v.impact ?? ""),
        );
    }
    if (serious.length > 0) {
      // Report honestly: id + impact + selectors, one line each.
      const summary = serious
        .map((v) => `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`)
        .join("\n");
      throw new Error(`axe serious/critical violations on the rail (${theme}):\n${summary}`);
    }
    expect(serious).toHaveLength(0);
  });
}

// AJ-E402 layout gate: no row may overflow its rail width — the provider
// abbrev, model, age and resource text yield (truncate) before the disc,
// state chip text or glyphs ever clip. Deliberately UNPINNED clock so
// pathological durations ("wait 730d") are exercised too.
test("rail — no row overflows its width", async ({ page }) => {
  await page.addInitScript((fixture) => {
    (window as never as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = fixture;
    localStorage.setItem("aij.theme", "dark");
    localStorage.setItem("aij.onboarding.dismissed", "1");
    sessionStorage.setItem("aij.ready.skipped", "1");
  }, allStatuses);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await page.getByRole("tab", { name: "Sessions" }).click();
  await expect(page.getByText("Handed over").first()).toBeVisible();
  const overflows = await page.evaluate(() => {
    const bad: string[] = [];
    for (const row of document.querySelectorAll("aside button")) {
      if (row.scrollWidth > row.clientWidth + 1) {
        bad.push(`${(row.getAttribute("aria-label") ?? "?").slice(0, 40)}: ${row.scrollWidth}px > ${row.clientWidth}px`);
      }
    }
    return bad;
  });
  expect(overflows).toEqual([]);
});


// AJ-E507 quality gate: axe-core 0 critical/serious on the kill
// sheet — the ready (interactive) state and the failed-verification
// explanation. The suite rides the same cleanup fixture as the
// 20-kill gallery; the sheet opens the real way (row affordance).
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

for (const theme of ["dark", "light"] as const) {
  for (const state of ["ready", "refused"] as const) {
    test(`kill-sheet — ${state} — axe 0 critical/serious — ${theme}`, async ({ page }) => {
      const cleanup = {
        groups: [
          {
            session: "s-1",
            provider: "claude-code",
            account: "Work Max",
            project: "web",
            machine: "machine-local",
            healthy: false,
            rows: [
              {
                pid: 8123,
                pgid: 8100,
                role: "helper",
                state: "orphaned",
                state_label: "Still running after its Instance closed",
                role_guess: "python3 (guess)",
                exe_path: "/usr/bin/python3",
                cmdline: "python3 -m http.server 5173",
                age_ms: 9_000_000,
                rss_bytes: 48 * 1024 * 1024,
                cpu_pct: 1.4,
                ports: "5173",
                hog: false,
                hog_rules: [],
                evidence: {
                  root_pid: 8100,
                  parent_pid: 8100,
                  pgid: 8100,
                  escaped_group: false,
                  verification: {
                    verdict: "verified",
                    detail: "start-time and exe match the live process",
                  },
                  first_seen: T0,
                  last_seen: T0 + 90_000,
                },
              },
            ],
          },
        ],
        clean_instances: 2,
        ledger_degraded: false,
      };
      await page.addInitScript(
        ({ fixture, t0, cleanup: c, theme: t, verify }) => {
          const w = window as unknown as {
            __AIJ_FIXTURE__?: unknown;
            __AIJ_CLEANUP__?: unknown;
            __AIJ_KILL_VERIFY__?: unknown;
          };
          w.__AIJ_FIXTURE__ = fixture;
          w.__AIJ_CLEANUP__ = c;
          w.__AIJ_KILL_VERIFY__ = () => verify;
          localStorage.setItem("aij.theme", t);
          localStorage.setItem("aij.onboarding.dismissed", "1");
          sessionStorage.setItem("aij.ready.skipped", "1");
          Date.now = () => t0;
        },
        {
          fixture: allStatuses,
          t0: T0,
          cleanup,
          theme,
          verify: state === "ready" ? KILL_VERIFY_OK : KILL_VERIFY_FAILED,
        },
      );
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.goto("/");
      await page.getByRole("button", { name: "Cleanup" }).click();
      await expect(page.getByTestId("cleanup-centre")).toBeVisible();
      await page.getByTestId("cleanup-kill-8123").click();
      await expect(
        page.getByTestId(state === "ready" ? "kill-ready-8123" : "kill-refused-8123"),
      ).toBeVisible();
      const results = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
        .analyze();
      let serious = results.violations.filter((v) =>
        ["serious", "critical"].includes(v.impact ?? ""),
      );
      if (serious.length > 0) {
        // AJ-E806: confirmatory re-scan — load-induced contrast false
        // positives do not survive; real violations still fail.
        await page.waitForTimeout(400);
        const confirm = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
          .analyze();
        serious = confirm.violations.filter((v) =>
          ["serious", "critical"].includes(v.impact ?? ""),
        );
      }
      if (serious.length > 0) {
        const summary = serious
          .map(
            (v) =>
              `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`,
          )
          .join("\n");
        throw new Error(`axe serious/critical violations on the kill sheet (${state}/${theme}):\n${summary}`);
      }
      expect(serious).toHaveLength(0);
    });
  }
}


// AJ-E508: axe-core 0 critical/serious on the tier surfaces — the
// per-Account toggles (the automation consent) and the Tier-2
// notification (the Undo window).
for (const theme of ["dark", "light"] as const) {
  test(`tiers — account toggles — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_CLEANUP_TIERS__ = { tier1_opt_in: true, tier2_opt_in: false };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: "Accounts" }).click();
    await page.getByRole("button", { name: "Inspect claude-code" }).first().click();
    await expect(page.getByTestId("account-tier-toggles")).toBeVisible();
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();
    let serious = results.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
    if (serious.length > 0) {
        // AJ-E806: confirmatory re-scan — load-induced contrast false
        // positives do not survive; real violations still fail.
        await page.waitForTimeout(400);
        const confirm = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
          .analyze();
        serious = confirm.violations.filter((v) =>
          ["serious", "critical"].includes(v.impact ?? ""),
        );
    }
    if (serious.length > 0) {
      const summary = serious
        .map(
          (v) =>
            `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`,
        )
        .join("\n");
      throw new Error(`axe violations on the tier toggles (${theme}):\n${summary}`);
    }
    expect(serious).toHaveLength(0);
  });

  test(`tiers — tier-2 notification — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_CLEANUP_STATUS__ = {
          paused: true,
          suspended: false,
          suspended_until_ms: null,
          tier2_pending: [
            { pid: 8123, session: "s-1", cmdline_trunc: "node vite dev", deadline_ms: t0 + 30_000 },
          ],
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: "Cleanup" }).click();
    await expect(page.getByTestId("cleanup-tier2-notice-8123")).toBeVisible();
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();
    let serious = results.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
    if (serious.length > 0) {
        // AJ-E806: confirmatory re-scan — load-induced contrast false
        // positives do not survive; real violations still fail.
        await page.waitForTimeout(400);
        const confirm = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
          .analyze();
        serious = confirm.violations.filter((v) =>
          ["serious", "critical"].includes(v.impact ?? ""),
        );
    }
    if (serious.length > 0) {
      const summary = serious
        .map(
          (v) =>
            `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`,
        )
        .join("\n");
      throw new Error(`axe violations on the tier-2 notice (${theme}):\n${summary}`);
    }
    expect(serious).toHaveLength(0);
  });
}

// AJ-E510: axe-core 0 critical/serious on the one-time restart summary
// dialog (modal semantics + focus land inside it) — the crash moment's
// first surface.
for (const theme of ["dark", "light"] as const) {
  test(`recovery — summary dialog — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t, recovery }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_RECOVERY__ = recovery;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        recovery: {
          summary:
            "2 Instances re-adopted, 1 helpers left behind, 1 processes could not be verified and were left alone.",
          adopted: 2,
          left_behind: 1,
          unverifiable: 1,
        },
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await expect(page.getByTestId("recovery-summary")).toBeVisible();
    // Focus is INSIDE the dialog (not lost behind the overlay).
    const focusInDialog = await page.evaluate(() => {
      const dialog = document.querySelector('[data-testid="recovery-summary"]');
      return dialog !== null && dialog.contains(document.activeElement);
    });
    if (!focusInDialog) throw new Error("focus did not land inside the recovery dialog");
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();
    let serious = results.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
    if (serious.length > 0) {
      // AJ-E806: confirmatory re-scan (same rationale as the kill sheet).
      await page.waitForTimeout(400);
      const confirm = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
        .analyze();
      serious = confirm.violations.filter((v) =>
        ["serious", "critical"].includes(v.impact ?? ""),
      );
    }
    if (serious.length > 0) {
      const summary = serious
        .map(
          (v) =>
            `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`,
        )
        .join("\n");
      throw new Error(`axe violations on the recovery dialog (${theme}):\n${summary}`);
    }
    expect(serious).toHaveLength(0);
  });
}

// AJ-E512: axe-core 0 critical/serious on the passive alert surfaces
// (hog alert with its staged-sheet affordance + the idle nudge).
for (const theme of ["dark", "light"] as const) {
  test(`alerts — hog + idle — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t, alerts }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_CLEANUP_ALERTS__ = alerts;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        alerts: {
          alerts: [
            {
              kind: "hog",
              text: "node (vite) under Work Max holds 3.1 GiB — review in Cleanup Centre.",
              pid: 8123,
              session: "s-1",
            },
            {
              kind: "idle",
              text: "python3 has been idle for 2h — review it in the Cleanup Centre.",
              pid: 8124,
              session: "s-1",
            },
          ],
        },
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: "Cleanup" }).click();
    await expect(page.getByTestId("hog-alert-8123")).toBeVisible();
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();
    let serious = results.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
    if (serious.length > 0) {
        // AJ-E806: confirmatory re-scan — load-induced contrast false
        // positives do not survive; real violations still fail.
        await page.waitForTimeout(400);
        const confirm = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
          .analyze();
        serious = confirm.violations.filter((v) =>
          ["serious", "critical"].includes(v.impact ?? ""),
        );
    }
    if (serious.length > 0) {
      const summary = serious
        .map(
          (v) =>
            `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`,
        )
        .join("\n");
      throw new Error(`axe violations on the alert surfaces (${theme}):\n${summary}`);
    }
    expect(serious).toHaveLength(0);
  });
}


// AJ-E511: axe-core 0 critical/serious on the fleet machine surfaces
// (the machine column + the widened All-machines view with its
// display-only foreign row).
for (const theme of ["dark", "light"] as const) {
  test(`fleet — machine column + filter — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t, cleanup }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_CLEANUP__ = cleanup;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        cleanup: {
          groups: [
            {
              session: "s-local", provider: "claude-code", account: "Work Max",
              project: "web", machine: "machine-local", healthy: false,
              rows: [
                {
                  machine_id: "machine-local", pid: 8123, pgid: 8100, role: "helper",
                  state: "orphaned", state_label: "Still running after its Instance closed",
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
                {
                  machine_id: "machine-studio", pid: 9101, pgid: 9100, role: "helper",
                  state: "orphaned", state_label: "Still running after its Instance closed",
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
        },
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: "Cleanup" }).click();
    await page
      .getByRole("combobox", { name: "Machine filter" })
      .selectOption("all");
    await expect(page.getByTestId("cleanup-machine-9101")).toBeVisible();
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();
    let serious = results.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
    if (serious.length > 0) {
        // AJ-E806: confirmatory re-scan — load-induced contrast false
        // positives do not survive; real violations still fail.
        await page.waitForTimeout(400);
        const confirm = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
          .analyze();
        serious = confirm.violations.filter((v) =>
          ["serious", "critical"].includes(v.impact ?? ""),
        );
    }
    if (serious.length > 0) {
      const summary = serious
        .map(
          (v) =>
            `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`,
        )
        .join("\n");
      throw new Error(`axe violations on the fleet surfaces (${theme}):\n${summary}`);
    }
    expect(serious).toHaveLength(0);
  });
}


// AJ-E611: axe-core 0 critical/serious on the ranking + notification
// surfaces — the NewSession ranked list (labelled listbox, expander)
// and the notification region (polite live-region, queue rows with an
// explicit Resume click) in BOTH themes.
for (const theme of ["dark", "light"] as const) {
  test(`ranking — ranked list + notifications — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t, ranked, events, rows }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_RANKING__ = ranked;
        w.__AIJ_ALERTS__ = events;
        w.__AIJ_RESET_QUEUE__ = rows;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        ranked: [
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
            account: "claude-lab",
            filtered_reason: "logged out — sign in from the Ready home",
            top_factors: [],
            factors: [],
            headroom_unknown: true,
            confidence: "Estimate",
            suggested: false,
          },
        ],
        events: [
          { kind: "actionable", account: "claude-work", text: "claude-work at 92% used (Measured), five-hour window — hand over or queue at reset", at_ms: T0 - 60_000 },
          { kind: "digest", account: "claude-personal", text: "Renews in 6 days — check your plan", at_ms: T0 - 3_600_000 },
        ],
        rows: [
          { id: "rq-1", session: "sess-1", account: "claude-work", resume_at_ms: T0 + 45 * 60_000, state: "queued" },
          { id: "rq-2", session: "sess-2", account: "claude-lab", resume_at_ms: T0 - 60_000, state: "ready" },
        ],
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    // The notification region renders in the work view (polite
    // live-region semantics assertable before the modal opens).
    await expect(page.getByTestId("alert-actionable")).toBeVisible();
    await expect(page.getByTestId("queue-ready")).toBeVisible();
    await page.getByRole("button", { name: "New", exact: true }).click();
    await expect(page.getByTestId("ranked-rows")).toBeVisible();
    const serious = await scanSeriousWithConfirm(page);
    if (serious.length > 0) {
      const summary = serious
        .map(
          (v) =>
            `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`,
        )
        .join("\n");
      throw new Error(`axe violations on the ranking surfaces (${theme}):\n${summary}`);
    }
    expect(serious).toHaveLength(0);
  });
}


// AJ-E205/E206: axe-core 0 critical/serious on the switch flow —
// the door (wizard with focus-trapped dialog, two radio freeze
// choices), the 1-click popover, the undo toast, and the
// source-safe failure banner — in BOTH themes.
const DOOR_WIZARD_FIXTURE = {
  mechanism: "transplant",
  fidelity: "F2",
  score: 0.78,
  warnings: ["This conversation was compacted. Early detail carries as text, not as live memory — it may ask about early decisions."],
  zombies: [],
  diff_snapshotted: true,
  source_live: true,
  experimental_copy: "Experimental: only the main transcript moves. Subagent files and rewind history stay on the source. The source stays resumable.",
  door: "wizard" as const,
  wizard_steps: ["Target", "What's carried", "Confirm"],
  wizard_reasons: ["This isn't the usual account for this project."],
  usual_account: false,
  carry: {
    carried: ["chat transcript", "rules & guidelines", "uncommitted git diff"],
    regenerated: [],
    dropped: ["todo list"],
  },
};

const DOOR_ONECLICK_FIXTURE = {
  ...DOOR_WIZARD_FIXTURE,
  fidelity: "F4",
  warnings: [],
  door: "one-click" as const,
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: {
    carried: ["chat transcript", "rules & guidelines", "uncommitted git diff", "MCP config", "skills & commands", "model settings"],
    regenerated: [],
    dropped: [],
  },
};

for (const theme of ["dark", "light"] as const) {
  test(`switch flow — wizard door + failure — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t, preflight, fail }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_SWITCH_PREFLIGHT__ = preflight;
        w.__AIJ_SWITCH_EXECUTE_FAIL__ = fail;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        preflight: DOOR_WIZARD_FIXTURE,
        fail: "S-F5: target launch failed — staged for retry (terminal spawn failed).",
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await page.getByTestId("handover-section")
      // AJ-E209: the ranked, badged row is the switch entry point.
      .locator("button[data-row]:not(:disabled)")
      .first()
      .click();
    await expect(page.getByTestId("switch-flow")).toBeVisible();
    // Scope to the dialog while a modal is open: the backdrop
    // intentionally dims the app behind it (a transient rendering
    // state, not the design's contrast); the dialog itself is
    // scanned in full.
    const dialogScan = async () => {
      const results = await new AxeBuilder({ page })
        .include("[data-testid='switch-flow']")
        .analyze();
      let serious = results.violations.filter((v) =>
        ["serious", "critical"].includes(v.impact ?? ""),
      );
      if (serious.length > 0) {
        await page.waitForTimeout(400);
        const confirm = await new AxeBuilder({ page })
          .include("[data-testid='switch-flow']")
          .analyze();
        serious = confirm.violations.filter((v) =>
          ["serious", "critical"].includes(v.impact ?? ""),
        );
      }
      return serious;
    };
    const serious = await dialogScan();
    // Keyboard traversal through the wizard (steps announced via
    // aria-live; the confirm click lives on the LAST step).
    await page.getByTestId("switch-continue").click();
    await expect(page.getByTestId("switch-steps")).toContainText("Step 2 of 3");
    const seriousCarried = await dialogScan();
    await page.getByTestId("switch-continue").click();
    await expect(page.getByTestId("switch-step-confirm")).toBeVisible();
    const seriousConfirm = await dialogScan();
    // Then the failure banner surface.
    await page.getByTestId("switch-confirm").click();
    await expect(page.getByTestId("switch-failed")).toBeVisible();
    const serious2 = await dialogScan();
    const all = [...serious, ...seriousCarried, ...seriousConfirm, ...serious2];
    if (all.length > 0) {
      const summary = all
        .map((v) => `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`)
        .join("\n");
      throw new Error(`axe violations on the switch wizard (${theme}):\n${summary}`);
    }
    expect(all).toHaveLength(0);
  });

  test(`switch flow — 1-click popover + undo toast — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t, preflight }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_SWITCH_PREFLIGHT__ = preflight;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: allStatuses,
        t0: T0,
        theme,
        preflight: DOOR_ONECLICK_FIXTURE,
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await page.getByTestId("handover-section")
      // AJ-E209: the ranked, badged row is the switch entry point.
      .locator("button[data-row]:not(:disabled)")
      .first()
      .click();
    await expect(page.getByTestId("switch-oneclick")).toBeVisible();
    const scan = async (scope: string) => {
      const results = await new AxeBuilder({ page })
        .include(`[data-testid='${scope}']`)
        .analyze();
      let serious = results.violations.filter((v) =>
        ["serious", "critical"].includes(v.impact ?? ""),
      );
      if (serious.length > 0) {
        await page.waitForTimeout(400);
        const confirm = await new AxeBuilder({ page })
          .include(`[data-testid='${scope}']`)
          .analyze();
        serious = confirm.violations.filter((v) =>
          ["serious", "critical"].includes(v.impact ?? ""),
        );
      }
      return serious;
    };
    const seriousPopover = await scan("switch-flow");
    // Execute → the undo window (toast at the app root).
    await page.getByTestId("switch-confirm").click();
    await expect(page.getByTestId("switch-undo-toast")).toBeVisible();
    const seriousToast = await scan("switch-undo-toast");
    const all = [...seriousPopover, ...seriousToast];
    if (all.length > 0) {
      const summary = all
        .map((v) => `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`)
        .join("\n");
      throw new Error(`axe violations on the 1-click door (${theme}):\n${summary}`);
    }
    expect(all).toHaveLength(0);
  });
}

// AJ-E207: axe-core 0 critical/serious on the switch-back flow —
// the review (delta preview, per-file conflict radio trios with
// file-path labels, freeze-or-fork), the S-F7 dead-original state,
// and the S-F8 forked failure — in BOTH themes; the conflict set is
// resolvable keyboard-only (radios carry their labels).
const BACK_CARDS_A11Y = ["sess-1", "sess-2", "sess-4", "sess-6", "sess-8"].map(
  (session_id) => ({
    session_id,
    original_account: "Work Max",
    away_account: "Personal",
    copy: "Work Max has reset. Resume the original conversation there? Work from Personal arrives as one 'while you were away' update.",
  }),
);

for (const theme of ["dark", "light"] as const) {
  test(`switch-back — review + conflict radios — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t, cards }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_SWITCH_BACK_SUGGESTIONS__ = cards;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, theme, cards: BACK_CARDS_A11Y },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await expect(page.getByTestId("switch-back-card")).toBeVisible();
    const cardScan = await new AxeBuilder({ page })
      .include("[data-testid='switch-back-card']")
      .analyze();
    await page.getByTestId("switch-back-card").click();
    await expect(page.getByTestId("switch-back-review")).toBeVisible();
    // Every conflict radio resolves by labelled name (keyboard path).
    await page.getByLabel("Take the away version").check();
    const dialogScan = await new AxeBuilder({ page })
      .include("[data-testid='switch-back-flow']")
      .analyze();
    // The failure surface (S-F8 forked).
    await page.getByTestId("switch-back-confirm").click();
    await expect(page.getByTestId("switch-back-done")).toBeVisible();
    const doneScan = await new AxeBuilder({ page })
      .include("[data-testid='switch-back-flow']")
      .analyze();
    const all = [cardScan, dialogScan, doneScan]
      .flatMap((result) => result.violations)
      .filter((violation) => ["serious", "critical"].includes(violation.impact ?? ""));
    if (all.length > 0) {
      const summary = all
        .map((v) => `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`)
        .join("\n");
      throw new Error(`axe violations on the switch-back flow (${theme}):\n${summary}`);
    }
    expect(all).toHaveLength(0);
  });
}

// AJ-E208: axe-core 0 critical/serious on the redaction surfaces —
// the first-run chooser (labelled radios, no default) and the
// per-item un-redact list (masked previews, checkbox per item) —
// in BOTH themes.
for (const theme of ["dark", "light"] as const) {
  test(`redaction — chooser + un-redact list — axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_SWITCH_PREFLIGHT__ = {
          mechanism: "transplant",
          fidelity: "F2",
          score: 0.78,
          warnings: [],
          zombies: [],
          diff_snapshotted: true,
          source_live: true,
          experimental_copy: "Experimental copy.",
          door: "wizard",
          wizard_steps: ["Target", "What's carried", "Confirm"],
          wizard_reasons: ["This isn't the usual account for this project."],
          usual_account: false,
          carry: {
            carried: ["chat transcript"],
            regenerated: [],
            dropped: ["todo list"],
          },
          redaction: { global: null, override: null, effective: null, source: "unset", line: "" },
          redaction_items: [],
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await page.getByTestId("handover-section")
      // AJ-E209: the ranked, badged row is the switch entry point.
      .locator("button[data-row]:not(:disabled)")
      .first()
      .click();
    // The wizard renders the chooser on its carried/confirm steps —
    // walk to Confirm (Target → What's carried) first.
    await page.getByTestId("switch-continue").click();
    await page.getByTestId("switch-continue").click();
    await expect(page.getByTestId("switch-redaction-chooser")).toBeVisible();
    const chooserScan = await new AxeBuilder({ page })
      .include("[data-testid='switch-flow']")
      .analyze();

    // Switch to a set policy + items on the preview step and re-scan
    // (a second init script overwrites the seam on reload).
    await page.addInitScript(() => {
      const w = window as unknown as { __AIJ_SWITCH_PREFLIGHT__?: Record<string, unknown> };
      if (w.__AIJ_SWITCH_PREFLIGHT__) {
        w.__AIJ_SWITCH_PREFLIGHT__.redaction = {
          global: "strip",
          override: null,
          effective: "strip",
          source: "global",
          line: "Redaction for claude-code: Strip — replace keys with ••• (your global choice)",
        };
        w.__AIJ_SWITCH_PREFLIGHT__.redaction_items = [
          { kind: "sk-key", preview: "s•••5678", segment: 0 },
        ];
      }
    });
    await page.reload();
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await page.getByTestId("handover-section")
      // AJ-E209: the ranked, badged row is the switch entry point.
      .locator("button[data-row]:not(:disabled)")
      .first()
      .click();
    await page.getByTestId("switch-continue").click();
    await expect(page.getByTestId("switch-redaction-items")).toBeVisible();
    const itemsScan = await new AxeBuilder({ page })
      .include("[data-testid='switch-flow']")
      .analyze();
    const all = [chooserScan, itemsScan]
      .flatMap((result) => result.violations)
      .filter((violation) => ["serious", "critical"].includes(violation.impact ?? ""));
    if (all.length > 0) {
      const summary = all
        .map((v) => `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`)
        .join("\n");
      throw new Error(`axe violations on the redaction surfaces (${theme}):\n${summary}`);
    }
    expect(all).toHaveLength(0);
  });
}

/**
 * AJ-E209 a11y gate: the ranked + badged switch entry and the Switch
 * Graph — axe 0 critical/serious on both surfaces (scoped includes,
 * the E806 confirmatory re-scan pattern), every graph node and failed
 * edge reachable + operable keyboard-only, arrows labelled, and the
 * badge state legible as TEXT (never colour-only, WCAG 2.2 / D-11).
 */
for (const theme of ["dark", "light"] as const) {
  test(`switch targets + graph — axe 0 critical/serious + keyboard — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_SWITCH_GRAPH__ = [
          {
            title: "Login refactor",
            epochs: [
              { id: "ep-a", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-1" },
              { id: "ep-b", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-2" },
            ],
            edges: [
              { from: "ep-a", to: "ep-b", label: "transplant F1 · 12:04", switch_back: false },
              {
                from: "ep-b",
                to: null,
                label: "transplant F1 · target launch failed",
                failed_reason: "S-F6: target window closed mid-launch",
                switch_back: false,
              },
            ],
            collapsed: 1,
            hop_note: "3 switches so far ≈41k tokens",
          },
        ];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByRole("button", { name: /Work Max/ }).first().click();

    // The ranked entry: scoped scan (the rows + their explanations).
    await expect(page.getByTestId("switch-target-rows")).toBeVisible();
    const rowsScan = await new AxeBuilder({ page })
      .include("[data-testid='handover-section']")
      .analyze();

    // The graph: scoped scan on the project panel.
    await page.getByRole("tab", { name: "Projects" }).click();
    await page.getByRole("button", { name: /ai-juggler/ }).first().click();
    await expect(page.getByTestId("switch-graph")).toBeVisible();
    const graphScan = await new AxeBuilder({ page })
      .include("[data-testid='project-switch-graph-card']")
      .analyze();

    const all = [rowsScan, graphScan]
      .flatMap((result) => result.violations)
      .filter((violation) => ["serious", "critical"].includes(violation.impact ?? ""));
    if (all.length > 0) {
      await page.waitForTimeout(400);
      const recheck = await new AxeBuilder({ page })
        .include("[data-testid='project-switch-graph-card']")
        .analyze();
      const still = recheck.violations.filter((v) =>
        ["serious", "critical"].includes(v.impact ?? ""),
      );
      expect.soft(still, `axe violations survive the re-scan (${theme})`).toHaveLength(0);
    }
    expect(all).toHaveLength(0);

    // Keyboard-only: the failed edge is a reachable, labelled button
    // and opens its §9 reason; nodes follow by ↑/↓ roving focus.
    const failedEdge = page.getByTestId("switch-graph-failed-edge");
    await failedEdge.focus();
    await failedEdge.press("Enter");
    await expect(page.getByTestId("switch-graph-failed-detail")).toContainText("S-F6");
    const nodes = page.getByTestId("switch-graph-node");
    await nodes.first().focus();
    await nodes.first().press("ArrowDown");
    await expect(nodes.nth(1)).toBeFocused();
  });
}

/**
 * AJ-E210 a11y gate: the Beta validation matrix is a REAL table —
 * row headers (scope=row), scoped columns, status in TEXT (never
 * colour-only), and axe reports 0 critical/serious on the card in
 * both themes.
 */
for (const theme of ["dark", "light"] as const) {
  test(`beta matrix — real table, axe 0 critical/serious — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, t0, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: allStatuses, t0: T0, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Accounts" }).click();
    await expect(page.getByTestId("beta-matrix-table")).toBeVisible();
    // Real table semantics: scoped column + row headers.
    await expect(page.getByTestId("beta-matrix-table").locator("th[scope='col']").first()).toBeVisible();
    const rowHeader = page.getByTestId("beta-matrix-row").first().locator("th[scope='row']");
    await expect(rowHeader).toContainText("→");
    // Status in text, never colour-only.
    await expect(page.getByTestId("beta-matrix-row").first()).toContainText(
      "Beta — not yet validated",
    );
    const scan = await new AxeBuilder({ page })
      .include("[data-testid='beta-matrix']")
      .analyze();
    const serious = scan.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
    if (serious.length > 0) {
      await page.waitForTimeout(400);
      const recheck = await new AxeBuilder({ page })
        .include("[data-testid='beta-matrix']")
        .analyze();
      const still = recheck.violations.filter((v) =>
        ["serious", "critical"].includes(v.impact ?? ""),
      );
      expect(still).toHaveLength(0);
    }
    expect(serious).toHaveLength(0);
  });
}

/**
 * AJ-E301 a11y gate: the wizard dialog — role=dialog + aria-modal +
 * labelled, axe 0 critical/serious on Step 0 in both themes, the
 * keyboard walkthrough (Tab cycles inside the trap, the not-installed
 * row opens install help, Esc closes only when idle), and focus lands
 * on the primary button after a step change.
 */
for (const theme of ["dark", "light"] as const) {
  test(`wizard step0 — dialog semantics, axe 0, keyboard — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, installs, t0, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_PROVIDER_INSTALLS__ = installs;
        localStorage.setItem("aij.theme", t);
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      {
        fixture: honestEmpty,
        theme,
        t0: 1_728_800_000_000,
        installs: [
          {
            provider: "claude-code",
            display_name: "Claude Code",
            install_url: "https://claude.com/download",
            status: "not_installed",
            version: null,
            binary: null,
            notes: [],
          },
          {
            provider: "codex",
            display_name: "Codex",
            install_url: "https://developers.openai.com/codex",
            status: "installed",
            version: "0.9.4",
            binary: "/opt/homebrew/bin/codex",
            notes: [],
          },
        ],
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    // First-run wizard — the gate only lives on this path now (the
    // Accounts entry is flow (d)'s LOCKED wizard since AJ-E306).
    await page.goto("/");
    const dialog = page.getByTestId("onboarding-wizard");
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute("role", "dialog");
    await expect(dialog).toHaveAttribute("aria-modal", "true");

    // The gate: Continue disabled with the missing provider; the row
    // opens install help keyboard-first.
    await expect(page.getByTestId("install-continue")).toBeDisabled();
    await page.locator("[data-testid='provider-row'] button").first().focus();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("install-help")).toContainText("Install Claude Code →");

    // AJ-E806 (recorded 2026-10-17): under suite load the FIRST scan
    // can catch the theme class mid-application (dark ink on light
    // canvas, 1.11:1) — it self-heals within the settle beat and the
    // confirmatory re-scan is the verdict; a real violation
    // reproduces deterministically and still fails.
    const confirmed = await scanSeriousWithConfirm(page);
    expect(confirmed).toHaveLength(0);

    // Tab stays inside the dialog (the trap): after several tabs the
    // focused element is still a descendant.
    for (let i = 0; i < 12; i += 1) {
      await page.keyboard.press("Tab");
    }
    await expect(dialog.locator(":focus")).toHaveCount(1);

    // Esc closes when idle.
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });
}

/**
 * AJ-E303 a11y gate: the candidate cards — confidence words + login
 * state + activity in TEXT (never colour alone), descriptive
 * aria-labels on the import actions, the managed card disabled with
 * its inline label, and axe 0 critical/serious on the Detect step.
 */
for (const theme of ["dark", "light"] as const) {
  test(`candidate cards — text tiers, aria labels, axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
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
              bytes: 9,
              last_used_secs: Math.floor(Date.now() / 1000) - 86_400,
              managed: false,
              notes: [],
              inference: {
                purpose: "Work",
                purpose_confidence: "likely",
                login: "signed-in",
                login_confidence: "found",
                activity: "active",
                activity_confidence: "found",
                activity_words: "Active this week",
                keychain_bound: false,
              },
              signals: ["home-pattern"],
            },
            {
              harness: "claude-code",
              path: "/Users/you/.claude-managed",
              origin: "home-pattern",
              login: "logged_in",
              bytes: 1,
              last_used_secs: Math.floor(Date.now() / 1000) - 86_400,
              managed: true,
              notes: [],
              inference: {
                purpose: "Managed",
                purpose_confidence: "guessed",
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
        localStorage.setItem("aij.theme", t);
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: honestEmpty, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    // First-run wizard (Accounts is flow (d)'s locked entry now).
    await page.goto("/");
    await page.getByTestId("install-continue").click();
    const cards = page.getByTestId("candidate-cards");
    await expect(cards).toBeVisible();
    // Words, not colours: the tier + login + activity all in text.
    await expect(cards).toContainText("Likely: Work");
    await expect(cards).toContainText("Signed in");
    await expect(cards).toContainText("Active this week");
    // Descriptive aria-label on the import action, starting with the
    // words shown (WCAG 2.5.3).
    await expect(cards.getByTestId("import-as").first()).toHaveAttribute(
      "aria-label",
      /^Import as “Work” — /,
    );
    // The managed card is disabled inline with its label.
    const managedImport = cards.locator(
      "[data-testid='candidate-card'][data-managed='true'] [data-testid='import-as']",
    );
    await expect(managedImport).toBeDisabled();
    await expect(managedImport).toContainText("Already in AI Juggler");
    const scan = await new AxeBuilder({ page })
      .include("[data-testid='onboarding-wizard']")
      .analyze();
    const serious = scan.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact ?? ""),
    );
    if (serious.length > 0) {
      await page.waitForTimeout(400);
      const recheck = await new AxeBuilder({ page })
        .include("[data-testid='onboarding-wizard']")
        .analyze();
      const still = recheck.violations.filter((v) =>
        ["serious", "critical"].includes(v.impact ?? ""),
      );
      expect(still).toHaveLength(0);
    }
    expect(serious).toHaveLength(0);
  });
}

/**
 * AJ-E304 a11y gate: the recommended-set proposal reads as a list —
 * every checkbox carries a descriptive aria-label (label + rationale),
 * the item rows are list items, and the accept-progress list announces
 * itself politely. Axe 0 critical/serious on the Detect step in BOTH
 * themes (D-16), confirmatory re-scan per AJ-E806.
 */
for (const theme of ["dark", "light"] as const) {
  test(`recommendation proposal — list semantics, aria labels, axe 0 — ${theme}`, async ({
    page,
  }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
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
              label: "Lab",
              action: "import",
              rationale: "Dormant — include when you want it",
              default_on: false,
              path: "/Users/you/.claude-lab",
            },
          ],
          summary: "Your juggling set: Claude ×1 (Work — signed in).",
          start_here: "-users-dev-api",
        };
        localStorage.setItem("aij.theme", t);
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: honestEmpty, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    // First-run wizard (the Accounts entry is flow (d)'s LOCKED path
    // since AJ-E306 — no gate there to continue through).
    await page.goto("/");
    await page.getByTestId("install-continue").click();
    await expect(page.getByTestId("proposal-teaser")).toBeVisible();
    // The teaser buttons carry their own names.
    await expect(page.getByTestId("use-recommended")).toHaveAccessibleName(
      "Use recommended setup →",
    );
    // Customise exposes the checkbox list: list items, one per account.
    await page.getByTestId("proposal-customise").click();
    const block = page.getByTestId("proposal-block");
    await expect(block).toBeVisible();
    await expect(page.getByTestId("proposal-item")).toHaveCount(2);
    const toggles = page.getByTestId("proposal-toggle");
    await expect(toggles).toHaveCount(2);
    // Descriptive labels — label AND rationale, not "import".
    await expect(toggles.nth(0)).toHaveAccessibleName(
      "Work: Signed in — import keeps history",
    );
    await expect(toggles.nth(1)).toHaveAccessibleName(
      "Lab: Dormant — include when you want it",
    );
    await expect(page.getByTestId("accept-setup")).toHaveAccessibleName(
      /Accept setup → \(1\)/,
    );
    // AJ-E806: one confirmatory re-scan after a settle beat — a load
    // race (theme mid-application) doesn't survive it; a real violation
    // reproduces deterministically and still fails the gate.
    const confirmed = await scanSeriousWithConfirm(page);
    expect(confirmed).toHaveLength(0);
  });
}

/**
 * AJ-E305 a11y gate: flow (b) completes FULLY from the keyboard
 * (⌘N → arrow keys over the tiles → Enter → Tab to Start session →
 * Enter), and every step of flow (a) scans axe-clean
 * (0 critical/serious) — D-16, confirmatory re-scan per AJ-E806.
 */
for (const theme of ["dark", "light"] as const) {
  test(`flow (b) — full keyboard completion, axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
    // Summon by keyboard alone.
    // The ⌘N handler mounts with the shell — wait for it (AJ-E806:
    // pressing during the first frames after load is a test race).
    await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
    await page.keyboard.press("Meta+n");
    await expect(page.getByRole("heading", { name: "New project" })).toBeVisible();
    const confirmed = await scanSeriousWithConfirm(page);
    expect(confirmed).toHaveLength(0);
    // Arrow keys move across the tiles with VISIBLE focus; Enter picks.
    // Tile order (recency): ai-juggler, sacrificial, storefront, Choose.
    await page.getByRole("button", { name: /Choose folder/ }).focus();
    await page.keyboard.press("ArrowLeft");
    await expect(page.getByRole("button", { name: /Work in storefront/ })).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(page.getByRole("button", { name: /Work in sacrificial/ })).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(page.getByRole("button", { name: /Work in ai-juggler/ })).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("project-account")).toBeVisible();
    // Tab reaches Start session →; Enter launches (preview mode's
    // honest error proves the activation arrived).
    await page.getByTestId("start-session").focus();
    await expect(page.getByTestId("start-session")).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByText("Preview mode — no backend connected.")).toBeVisible();
  });

  test(`flow (a) — axe 0 on every step — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
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
          summary: "Your juggling set: Claude ×2 (Work — signed in; Side — needs sign-in).",
          start_here: "/Users/you/dev-api",
        };
        localStorage.setItem("aij.theme", t);
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: honestEmpty, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    // First-run wizard (the Accounts entry is flow (d)'s LOCKED
    // path since AJ-E306 — no Step-0 gate to scan there).
    await page.goto("/");
    await expect(page.getByTestId("wizard-step-install")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("install-continue").click();
    await expect(page.getByTestId("proposal-teaser")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("proposal-customise").click();
    await expect(page.getByTestId("proposal-block")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    // AJ-E309: the first-run policy question rides this Confirm.
    await answerPolicy(page, "strip");
    await page.getByTestId("accept-setup").click();
    await expect(page.getByTestId("signin-queue")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    for (let card = 0; card < 2; card += 1) {
      await page.getByTestId("signin-skip").click();
    }
    await expect(page.getByTestId("summary-exits")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/**
 * AJ-E306 a11y gates: the launch sheet traps focus and returns it on
 * close; both new surfaces scan axe-clean (0 critical/serious) —
 * confirmatory re-scan per AJ-E806 (D-16).
 */
for (const theme of ["dark", "light"] as const) {
  test(`launch sheet — focus trap, return focus, axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expect(page.getByRole("tab", { name: "Sessions" })).toBeVisible();
    // The summoning button is where focus must RETURN.
    const summon = page.getByRole("button", { name: /^New$/ }).first();
    await summon.click();
    await expect(page.getByTestId("new-session-sheet")).toBeVisible();
    const confirmed = await scanSeriousWithConfirm(page);
    expect(confirmed).toHaveLength(0);
    // The trap: many Tabs keep focus inside the dialog.
    for (let tab = 0; tab < 12; tab += 1) {
      await page.keyboard.press("Tab");
    }
    const focusedInside = await page.evaluate(() => {
      const sheet = document.querySelector("[data-testid='new-session-sheet']");
      return sheet !== null && sheet.contains(document.activeElement);
    });
    expect(focusedInside).toBe(true);
    // Esc cancels cleanly and focus returns to the summoner.
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("new-session-sheet")).toHaveCount(0);
    await expect(summon).toBeFocused();
  });

  test(`Add Provider sheet — axe 0, cards reachable by keyboard — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, installs, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_PROVIDER_INSTALLS__ = installs;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
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
            verified_against: ["2.1.x"],
            kind: "terminal",
            single_instance_note: null,
          },
          {
            provider: "codex",
            display_name: "Codex",
            install_url: "https://developers.openai.com/codex",
            status: "not_installed",
            version: null,
            binary: null,
            notes: [],
            verified_against: [],
            kind: "terminal",
            single_instance_note: null,
          },
        ],
      },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Accounts" }).click();
    await page.getByTestId("add-provider").click();
    await expect(page.getByTestId("provider-cards")).toBeVisible();
    const confirmed = await scanSeriousWithConfirm(page);
    expect(confirmed).toHaveLength(0);
    // Keyboard operable: focusing a card's action and pressing Enter
    // runs it (the wizard opens) — cards are real buttons, not
    // click-only divs.
    await page.locator("button[data-testid='provider-choose']").focus();
    await expect(page.locator("button[data-testid='provider-choose']")).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("onboarding-wizard")).toBeVisible();
  });
}

/**
 * AJ-E307 a11y gate: the Confirm-source panel — the breakdown is
 * real list markup, failure states announce via role="alert", and
 * the step scans axe-clean (0 critical/serious) in both themes.
 */
for (const theme of ["dark", "light"] as const) {
  test(`confirm-source — list semantics, alerts, axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
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
        w.__AIJ_SOURCE_INSPECT__ = {
          total_bytes: 46_000_000,
          file_count: 128,
          folders: [
            { name: "projects/", bytes: 40_000_000 },
            { name: "settings.json", bytes: 60 },
          ],
          truncated: false,
          ceiling_bytes: 2_147_483_648,
          is_config: true,
          already_managed: false,
          login: "logged_in",
          keychain_bound: true,
        };
        localStorage.setItem("aij.theme", t);
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: honestEmpty, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByTestId("install-continue").click();
    await page.getByTestId("import-as").first().click();
    const panel = page.getByTestId("confirm-source");
    await expect(panel).toBeVisible();
    // The breakdown is a real list with items a screen reader counts.
    const breakdown = page.getByTestId("source-breakdown");
    await expect(breakdown).toBeVisible();
    expect(await breakdown.locator("li").count()).toBeGreaterThan(0);
    // Sizes are text (never colour-only).
    await expect(panel).toContainText("projects/");
    const confirmed = await scanSeriousWithConfirm(page);
    expect(confirmed).toHaveLength(0);
  });
}

/**
 * AJ-E309 a11y gates (D-16, both themes): the policy question is a
 * labelled radio group (fieldset + legend, every option's consequence
 * wired by aria-describedby); the renewal date input is labelled; the
 * Expert hatch is a disclosure (aria-expanded/aria-controls); the
 * Summary restatement and Accounts → Pack secrets scan axe-clean.
 */
for (const theme of ["dark", "light"] as const) {
  test(`first-run policy, renewal, expert, summary — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
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
            { provider: "claude-code", label: "Work", action: "import", rationale: "Signed in — import keeps history", default_on: true, path: "/Users/you/.claude-work" },
          ],
          summary: "Your juggling set: Claude ×1 (Work — signed in).",
          start_here: null,
        };
        w.__AIJ_LOGIN_STATE__ = "signed_in";
        localStorage.setItem("aij.theme", t);
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: honestEmpty, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByTestId("install-continue").click();
    await page.getByTestId("use-recommended").click();

    // The radio group: a named group, four labelled radios, none checked.
    const group = page.getByRole("group", { name: "How should context packs handle secrets?" });
    await expect(group).toBeVisible();
    await expect(group.getByRole("radio")).toHaveCount(4);
    await expect(group.getByRole("radio", { checked: true })).toHaveCount(0);
    await expect(group.getByRole("radio", { name: "Strip — replace keys with •••" })).toHaveAccessibleDescription(
      /replaced with •••/,
    );
    await expect(page.getByTestId("accept-setup")).toHaveAccessibleDescription(/Decide later/);

    // The renewal date: a labelled native date input.
    await page.getByTestId("renewal-date-add").click();
    await expect(page.getByLabel("Renewal date for Work (optional)")).toHaveAttribute("type", "date");
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);

    // The Expert hatch: a disclosure whose panel scans clean.
    const hatch = page.getByTestId("expert-hatch");
    await hatch.click();
    await expect(hatch).toHaveAttribute("aria-expanded", "true");
    const controls = await hatch.getAttribute("aria-controls");
    expect(controls).toBeTruthy();
    await expect(page.locator(`[id="${controls}"]`)).toBeVisible();
    await expect(page.getByLabel("State folder (read-only)")).toHaveAttribute("readonly", "");
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);

    // Keyboard: the radios are reachable and choosable by keyboard.
    await group.getByRole("radio").first().focus();
    await page.keyboard.press("Space");
    await expect(group.getByRole("radio", { checked: true })).toHaveCount(1);
    await page.getByTestId("accept-setup").click();
    await expect(page.getByTestId("summary-exits")).toBeVisible();
    await expect(page.getByTestId("policy-restatement")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });

  test(`Accounts → Pack secrets + env overrides — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_RECIPE_ENV__ = { "p-1": { CLAUDE_CODE_MAX_OUTPUT_TOKENS: "8000" } };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Accounts" }).first().click();
    await expect(page.getByRole("group", { name: "Your choice (all Providers)" })).toBeVisible();
    await page.getByRole("button", { name: /Edit launch settings/ }).first().click();
    await page.getByTestId("recipe-env-toggle").first().click();
    await expect(page.getByLabel("Variable name 1")).toHaveValue("CLAUDE_CODE_MAX_OUTPUT_TOKENS");
    await page.getByTestId("expert-hatch").first().click();
    await expect(page.getByTestId("expert-panel").first()).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E702 a11y (both themes): the Fleet screen with peers, the
 * labelled, keyboard-focusable fingerprint, and the Unpair
 * confirmation (an alertdialog with Cancel focused) scan axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`fleet — identity + unpair confirm — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_FLEET_PEERS__ = [
          { machine_id: "m1", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA", permission: "view", permission_label: "View only", ask_each_time: false, paired_via: "code", paired_at_ms: 1, last_seen_ms: null },
        ];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Fleet" }).first().click();
    const fingerprint = page.getByLabel("Machine fingerprint");
    await expect(fingerprint).toBeVisible();
    await fingerprint.focus();
    await expect(fingerprint).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByRole("button", { name: "Unpair Mac mini" }).click();
    const sheet = page.getByRole("alertdialog", { name: "Unpair Mac mini?" });
    await expect(sheet).toBeVisible();
    await expect(sheet.getByRole("button", { name: "Cancel" })).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}


/** AJ-E704 a11y (both themes): the Link switch and every pairing step —
 * code, Verify, Permissions (radio group), a failure (alert) and the
 * joiner's labelled 6-digit code group — scan axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`fleet — pairing wizard — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_FLEET_PEERS__ = [];
        w.__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
        w.__AIJ_DISCOVERED__ = [{ instance_id: "a1b2c3d4", label: "Unnamed machine (a1b2c3d4)", version: "1.0" }];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Fleet" }).first().click();
    await expect(page.getByRole("switch", { name: /Link on/ })).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);

    await page.getByTestId("fleet-add").click();
    await expect(page.getByRole("dialog", { name: "Add a machine" })).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.evaluate(() => (window as unknown as { __AIJ_PAIR_SIMULATE__: (e: string) => void }).__AIJ_PAIR_SIMULATE__("joiner"));
    await expect(page.getByTestId("pairing-verify")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("pairing-fingerprints-match").click();
    await expect(page.getByRole("radiogroup")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.keyboard.press("Escape");

    await page.getByTestId("fleet-join").click();
    await expect(page.getByRole("group", { name: "Pairing code" })).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByRole("textbox", { name: "Digit 1 of 6" }).focus();
    await page.keyboard.type("000000");
    await page.getByTestId("pairing-connect").click();
    await expect(page.getByRole("alert").filter({ hasText: "The code didn't match" })).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1203 a11y (both themes): Settings in every Account & Cloud
 * state — the Appearance radio group, the labelled feature switches,
 * the consent screen and the Unlink alertdialog — scan axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`settings — appearance + account & cloud — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    // Wait for the shell (its key listener) before the shortcut.
    await expect(page.getByRole("button", { name: "Settings" }).first()).toBeVisible();
    await page.keyboard.press("ControlOrMeta+Comma");
    await expect(page.getByTestId("settings-screen")).toBeVisible();
    await expect(page.getByRole("radiogroup", { name: "Appearance" })).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("cloud-connect").click();
    await expect(page.getByTestId("cloud-linking")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.evaluate(() => {
      (window as unknown as Record<string, unknown>).__AIJ_FIGUREHEAD_APPROVE__ = "approved";
    });
    await expect(page.getByTestId("cloud-consent")).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole("switch", { name: "Cloud backup" })).toHaveAttribute("aria-checked", "true");
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("cloud-turn-on").click();
    await page.getByTestId("cloud-unlink").click();
    const sheet = page.getByRole("alertdialog", { name: "Unlink this Mac from Figurehead?" });
    await expect(sheet.getByRole("button", { name: "Cancel" })).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.keyboard.press("Escape");
    // Offline + a paid-locked feature (warning text, disabled switch, Upgrade).
    await page.evaluate(() => {
      const w = window as unknown as { __AIJ_FIGUREHEAD__: Record<string, unknown> & { features: unknown[] } };
      w.__AIJ_FIGUREHEAD__ = {
        ...w.__AIJ_FIGUREHEAD__,
        offline: true,
        features: [...w.__AIJ_FIGUREHEAD__.features, { key: "transcriptVault", access: "upgrade", tier: "paid", on: false }],
      };
    });
    await page.getByRole("button", { name: "Fleet" }).first().click();
    await page.getByRole("button", { name: "Settings" }).first().click();
    await expect(page.getByTestId("cloud-offline")).toBeVisible();
    await expect(page.getByTestId("cloud-upgrade")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E705 a11y (both themes): the remote inspector — breadcrumb,
 * polite freshness line, the six tabs and the disabled-with-reason
 * Stop — scans axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`fleet — remote inspector — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const now = Date.now();
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
        w.__AIJ_FLEET_PEERS__ = [{
          machine_id: "machine-mini", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
          permission: "view", permission_label: "View only", ask_each_time: false, paired_via: "code",
          paired_at_ms: now, last_seen_ms: now, connection: "connected", protocol: "v1.0", connection_detail: null,
        }];
        w.__AIJ_FLEET_REMOTE__ = [{
          machine_id: "machine-mini", label: "Mac mini", connection: "connected", freshness: "stale",
          last_heard_ms: now - 40_000, permission: "view",
          resources: { cpu_percent: 12, memory_used_bytes: 1e9, memory_total_bytes: 8e9, measured_at_ms: now },
          instances: [{
            id: "inst-1", provider: "claude-code", account_label: "Work Max", status: "running",
            model: "claude-opus-4", model_source: "transcript", project_name: "web-shop", started_at_ms: now - 60_000,
            last_activity_ms: now, cpu_percent: 3.1, rss_bytes: 3e8, process_count: 2, measured_at_ms: now,
          }],
        }];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Fleet" }).first().click();
    await page.getByTestId("fleet-remote-instance").click();
    await expect(page.getByRole("navigation", { name: "Breadcrumb" })).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByRole("tab", { name: "Actions" }).click();
    await expect(page.getByTestId("remote-stop")).toBeDisabled();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByRole("navigation", { name: "Breadcrumb" }).getByRole("button", { name: "Mac mini" }).click();
    await expect(page.getByTestId("remote-machine-overview")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E709 a11y (both themes): the board with a paired machine's rows,
 * the Access sheet (radio group) and the "Ask each time" alertdialog. */
for (const theme of ["dark", "light"] as const) {
  test(`fleet — board machines, access, prompt — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const now = Date.now();
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
        w.__AIJ_FLEET_PEERS__ = [{
          machine_id: "machine-mini", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
          permission: "control", permission_label: "View + control", ask_each_time: true, paired_via: "code",
          paired_at_ms: now, last_seen_ms: now, connection: "connected", protocol: "v1.0", connection_detail: null,
        }];
        w.__AIJ_FLEET_REMOTE__ = [{
          machine_id: "machine-mini", label: "Mac mini", connection: "connected", freshness: "live",
          last_heard_ms: now, permission: "view", resources: null, alerts: [],
          instances: [{
            id: "inst-1", provider: "claude-code", account_label: "Work Max", status: "running",
            model: "claude-opus-4", model_source: "transcript", project_name: "web-shop", started_at_ms: now,
            last_activity_ms: now, cpu_percent: 1, rss_bytes: 1e8, process_count: 1, measured_at_ms: now,
          }],
        }];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await expect(page.getByTestId("board-remote-row")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByRole("button", { name: "Fleet" }).first().click();
    await page.getByTestId("fleet-access").click();
    await expect(page.getByTestId("fleet-access-sheet")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.keyboard.press("Escape");
    await page.evaluate(() => {
      (window as unknown as Record<string, unknown>).__AIJ_FLEET_PROMPTS__ = [
        { id: 1, peer_label: "Mac mini", action: "stop Work Max (claude-code)", expires_at_ms: Date.now() + 20_000 },
      ];
    });
    await expect(page.getByRole("alertdialog", { name: "Mac mini asks to act on this Mac" })).toBeVisible({ timeout: 8000 });
    await expect(page.getByTestId("fleet-prompt-deny")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E711 a11y (both themes): the palette listing a paired machine's
 * desktop Instance and the remote Focus outcome scan axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`fleet — remote focus palette + outcome — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const now = Date.now();
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
        w.__AIJ_FLEET_PEERS__ = [{
          machine_id: "machine-mini", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
          permission: "view", permission_label: "View only", ask_each_time: false, paired_via: "code",
          paired_at_ms: now, last_seen_ms: now, connection: "connected", protocol: "v1.0", connection_detail: null,
        }];
        w.__AIJ_FLEET_REMOTE__ = [{
          machine_id: "machine-mini", label: "Mac mini", connection: "connected", freshness: "live",
          last_heard_ms: now, permission: "control", resources: null, alerts: [],
          instances: [{
            id: "inst-desk", provider: "claude-code", account_label: "Desk", status: "running",
            model: null, model_source: "unknown", project_name: "web-shop", started_at_ms: now,
            last_activity_ms: now, cpu_percent: null, rss_bytes: null, process_count: null,
            kind: "desktop", measured_at_ms: now,
          }],
        }];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
    await page.keyboard.press("Meta+k");
    await expect(page.getByTestId("goto-remote-machine").first()).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("goto-row-remote-inst-desk").click();
    await page.getByRole("tab", { name: "Actions" }).click();
    await expect(page.getByTestId("remote-outcome")).toContainText("Focused window");
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E707 a11y (both themes): paired machines in the Cleanup Centre and
 * the Clean confirmation (alertdialog, Cancel focused). */
for (const theme of ["dark", "light"] as const) {
  test(`fleet — remote cleanup — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const now = Date.now();
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
        w.__AIJ_FLEET_REMOTE__ = [{
          machine_id: "machine-mini", label: "Mac mini", connection: "connected", freshness: "live",
          last_heard_ms: now, permission: "control", resources: null, instances: [], alerts: [],
          processes: [
            { instance_id: "inst-1", provider: "claude-code", account_label: "Work Max", pid: 4242, role: "helper", state: "orphaned",
              state_label: "Still running after its Instance closed", verified: true, name: "node", age_ms: 60000,
              rss_bytes: null, cpu_percent: null, ports: "3000", measured_at_ms: now },
            { instance_id: "inst-1", provider: "claude-code", account_label: "Work Max", pid: 4250, role: "helper", state: "unverifiable",
              state_label: "Unverifiable — will not be touched", verified: false, name: "python3", age_ms: 60000,
              rss_bytes: null, cpu_percent: null, ports: "", measured_at_ms: now },
          ],
        }];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Cleanup" }).first().click();
    await page.getByLabel("Machine filter").selectOption("all");
    await expect(page.getByTestId("remote-cleanup")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("remote-cleanup-clean").click();
    await expect(page.getByRole("alertdialog", { name: "Clean node on Mac mini?" })).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E708 a11y (both themes): an away machine — the sync chip with its
 * queued count, the clock-skew note, last-known rows on Fleet and on the
 * board — scans axe-clean; state is in words, never colour alone. */
for (const theme of ["dark", "light"] as const) {
  test(`fleet — sync chip, skew, last known — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const now = Date.now();
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
        w.__AIJ_FLEET_PEERS__ = [{
          machine_id: "machine-mini", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
          permission: "view", permission_label: "View only", ask_each_time: false, paired_via: "code",
          paired_at_ms: now, last_seen_ms: now - 3_600_000, connection: "offline", protocol: null,
          connection_detail: null, queued_changes: 12,
        }];
        w.__AIJ_FLEET_REMOTE__ = [{
          machine_id: "machine-mini", label: "Mac mini", connection: "offline", freshness: "offline",
          last_heard_ms: now - 3_600_000, permission: "view", resources: null, alerts: [], processes: [],
          clock_skew_ms: 12 * 60_000, synced_at_ms: now - 3_600_000,
          instances: [{
            id: "inst-1", provider: "claude-code", account_label: "Work Max", status: "running",
            model: "claude-opus-4", model_source: "transcript", project_name: "web-shop", started_at_ms: now - 60_000,
            last_activity_ms: now - 3_600_000, cpu_percent: null, rss_bytes: null, process_count: null,
            kind: "terminal", measured_at_ms: now - 3_600_000,
          }],
        }];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Fleet" }).first().click();
    await expect(page.getByTestId("fleet-sync")).toHaveText("⇡ 12 changes queued");
    await expect(page.getByTestId("fleet-clock-skew")).toBeVisible();
    await expect(page.getByTestId("fleet-last-known")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.reload();
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByLabel("Filter by machine").selectOption("machine-mini");
    await expect(page.getByTestId("board-remote-row").first()).toContainText("was running");
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E706 / AJ-E708 a11y (both themes): cross-machine destinations, the
 * wizard (radio groups with descriptions, path mapping), and the conflict
 * sheet (radio group, "Review later" focused) — axe-clean, keyboard-only
 * completable. */
for (const theme of ["dark", "light"] as const) {
  test(`fleet — cross-machine switch and conflict sheet — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        const base = {
          machine_id: "machine-mini", machine_label: "Mac mini", recipe_id: "rec-mini", recipe_name: "Main",
          account_label: "Team", provider: "claude-code", mechanism: "transplant", readiness: "needs_mapping",
          blockers: ["ai-juggler isn't at the same path on Mac mini — pick its copy there (you confirm the mapping) or use T3."],
          fidelity: "F3", badge: "F3 · Unvalidated", validated: false, carried: ["chat transcript"], regenerated: [],
          dropped: ["todo list"], same_path_project: null,
          projects: [{ project_id: "proj-m1", name: "ai-juggler", same_path: false, same_name: true }],
          replay_available: true, version_here: "2.1.0", version_there: "2.1.0",
        };
        w.__AIJ_FLEET_SWITCH_TARGETS__ = [base];
        w.__AIJ_FLEET_SWITCH_BACK_PLAN__ = {
          original_machine_id: "machine-laptop", original_machine_label: "Laptop", original_account_label: "Work Max",
          away_account_label: "Desktop Max", new_turns: 2, delta_preview: "While you were away on Desktop Max — 2 new turns",
          blockers: [],
        };
        w.__AIJ_FLEET_SWITCH_BACK_OUTCOMES__ = [{
          accepted: false, reason_code: "diverged", message: "moved on", instance_id: null, edge_id: "e",
          divergence: { turns_since: 1, last_activity_ms: Date.now() - 60_000, summary: "1 new turns" },
        }];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await expect(page.getByTestId("remote-switch-target")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    // The wizard, keyboard-only: open, step, map, confirm.
    await page.getByTestId("remote-switch-target").focus();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("cross-switch-flow")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("cross-switch-next").click();
    await page.getByTestId("cross-switch-project-option").focus();
    await page.keyboard.press("Space");
    await page.getByTestId("cross-switch-confirm-mapping").focus();
    await page.keyboard.press("Space");
    await expect(page.getByTestId("cross-switch-next")).toBeEnabled();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.keyboard.press("Escape");
    // The conflict sheet: "Review later" holds focus; a choice is a radio.
    await page.getByTestId("cross-back-card").click();
    await page.getByTestId("cross-back-go").click();
    await expect(page.getByTestId("conflict-later")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("conflict-keep_mine").focus();
    await page.keyboard.press("Space");
    await expect(page.getByTestId("conflict-apply")).toBeEnabled();
  });
}

/** AJ-E1001 a11y (both themes): the rail's identity marker (focusable,
 * named in words) and the wrong-account pause (alertdialog, actions in
 * order, Esc cancels) scan axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`identity — rail marker and wrong-account pause — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        const base = fixture as { sessions: Array<Record<string, unknown>> };
        w.__AIJ_FIXTURE__ = {
          ...base,
          sessions: base.sessions.map((s, i) => ({
            ...s,
            identity_state: ["verified", "first_seen", "unverified", "signed_out"][i % 4],
            identity_label: "o…@work.example · Acme",
          })),
        };
        w.__AIJ_IDENTITY_CHECK__ = {
          "p-1": {
            state: "mismatch", provider: "claude-code", account_id: "claude-work", account_label: "Work Max",
            model: "opus", expected: "o…@work.example · Acme", detected: "o…@home.example", reason: null,
            alternatives: [{ recipe_id: "p-2", recipe_name: "Main", account_label: "Personal" }],
          },
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await expect(page.getByTestId("rail-identity").first()).toHaveAttribute("aria-label", /verified|first seen|unverified|Not signed in/);
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await page.getByRole("tab", { name: "Actions" }).click();
    await page.getByRole("button", { name: /Resume|Relaunch/ }).first().click();
    await expect(page.getByTestId("identity-mismatch")).toBeVisible();
    await expect(page.getByTestId("identity-switch")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("identity-recheck")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("identity-mismatch")).toHaveCount(0);
  });
}

/** AJ-E1002 a11y (both themes): the git badge in words, the worktree
 * offer (radio group, default focus on the safe choice) and the dirty
 * removal dialog (typed confirmation) scan axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`worktree — offer and removal — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        const base = fixture as { sessions: Array<Record<string, unknown>> };
        w.__AIJ_FIXTURE__ = {
          ...base,
          sessions: base.sessions.map((s) => ({ ...s, git_branch: "juggler/a1b2c3", git_dirty_count: 3, git_ahead: 1, git_worktree: true, cwd: "/Users/dev/ai-juggler" })),
        };
        w.__AIJ_GIT_STATUS__ = Object.fromEntries(
          base.sessions.map((s) => [s.id as string, {
            branch: "juggler/a1b2c3", changed: 2, untracked: 1, ahead: 1, behind: 0, worktree: true,
            created_by_juggler: true, path: "/Users/dev/ai-juggler.juggler/a1b2c3",
          }]),
        );
        w.__AIJ_ISOLATION_OFFER__ = {
          project_name: "ai-juggler", in_use_by: ["Personal"], available: true, unavailable_reason: null,
          proposed_path: "/Users/dev/ai-juggler.juggler/a1b2c3", proposed_branch: "juggler/a1b2c3", base_branch: "main",
        };
        w.__AIJ_WORKTREE_REMOVAL__ = {
          path: "/Users/dev/ai-juggler.juggler/a1b2c3", folder: "a1b2c3", branch: "juggler/a1b2c3",
          changed: 2, untracked: 1, needs_confirmation: true, in_use: false, summary: "Discards 2 changed and 1 untracked files.",
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await expect(page.getByTestId("git-badge").first()).toHaveAttribute("title", /3 uncommitted files/);
    await page.getByRole("button", { name: /Work Max/ }).first().click();
    await page.getByTestId("worktree-remove").click();
    await expect(page.getByTestId("worktree-remove-confirm")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.keyboard.press("Escape");
    await page.getByRole("tab", { name: "Actions" }).click();
    await page.getByRole("button", { name: /Resume|Relaunch/ }).first().click();
    await expect(page.getByTestId("isolation-worktree")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1003 a11y (both themes): the parked row (countdown as text) and
 * the reset offer (Resume and a distinct snooze, both keyboard
 * reachable) scan axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`parked — row and reset offer — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        const base = fixture as { sessions: Array<Record<string, unknown>> };
        w.__AIJ_FIXTURE__ = {
          ...base,
          sessions: base.sessions.map((s) =>
            s.status === "quota_blocked" ? { ...s, parked: "estimated", parked_resets_at_ms: Date.now() + 3_600_000 } : s,
          ),
        };
        w.__AIJ_PARKED_OFFERS__ = [
          { session_id: "x", account_label: "Personal", text: "Personal has reset. Resume the work you parked.", last: false },
        ];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("tab", { name: "Sessions" }).click();
    await expect(page.getByTestId("rail-parked").first()).toContainText("Parked (est.)");
    await page.getByTestId("parked-offer-resume").focus();
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("parked-offer-snooze")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1004 a11y (both themes): the morning summary — a labelled
 * region, one button per row, severity in words — scans axe-clean and
 * every row is reachable by keyboard. */
for (const theme of ["dark", "light"] as const) {
  test(`morning summary — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        const tr = [{ from: "running", to: "failed", at_ms: Date.now() - 3_600_000 }];
        w.__AIJ_SUMMARY__ = {
          since_ms: Date.now() - 9 * 3_600_000,
          generated_ms: Date.now(),
          rows: [
            { session_id: "sess-8", account_label: "Work Max", provider: "claude-code", project: "ai-juggler", group: "crashed", status: "failed", words: "Crashed", transitions: tr, tokens: null, leftovers: 1 },
            { session_id: "sess-3", account_label: "Personal", provider: "claude-code", project: null, group: "needs_you", status: "waiting_user", words: "Needs you — waiting for an answer", transitions: tr, tokens: 1200, leftovers: 0 },
          ],
          counts: { crashed: 1, needs_you: 1 },
          leftovers: 1,
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expect(page.getByRole("region", { name: "Since you were away" })).toBeVisible();
    await page.getByTestId("summary-seen").focus();
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("summary-row").first()).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("summary-row").nth(1)).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1005 a11y (both themes): the triage view — labelled groups, the
 * action named on each row with its shortcut — scans axe-clean and
 * needs no mouse: j moves, A runs the action. */
for (const theme of ["dark", "light"] as const) {
  test(`triage — axe 0 and keyboard only — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Triage" }).click();
    await expect(page.getByRole("region", { name: /Needs you/ })).toBeVisible();
    await page.getByTestId("triage-row").first().locator("button").first().focus();
    await page.keyboard.press("j");
    await expect(page.getByTestId("triage-row").nth(1).locator("button").first()).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1007 a11y (both themes): History (search, tabs, row actions) and
 * the delete dialog scan axe-clean; the delete button cannot be reached
 * armed without typing the word. */
for (const theme of ["dark", "light"] as const) {
  test(`history and delete dialog — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "History" }).click();
    await expect(page.getByTestId("history-search")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("history-delete").first().click();
    await expect(page.getByTestId("delete-confirm")).toBeFocused();
    await expect(page.getByTestId("delete-go")).toBeDisabled();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1009 a11y (both themes): the Backup & restore card and the
 * restore plan dialog scan axe-clean; the plan opens with Restore
 * focused and Escape closes it without restoring. */
for (const theme of ["dark", "light"] as const) {
  test(`backup and restore plan — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_BACKUP_FILE__ = "/Users/dev/b.ajbackup";
        w.__AIJ_RESTORE_PLAN__ = {
          path: "", other_machine: true, token: "t", notes: ["Sign in to each Account again afterwards."],
          manifest: {
            format: "ai-juggler-backup", format_version: 1, schema_version: 27, app_version: "0.0.1", created_at_ms: Date.now(),
            origin: { fingerprint: "x", label: "Studio Mac" }, counts: { instances: 3 }, files: [], withheld: [], excluded: [],
          },
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Settings" }).click();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("backup-restore").click();
    await expect(page.getByTestId("restore-go")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("restore-plan")).toHaveCount(0);
  });
}

/** AJ-E1010 a11y (both themes): About & updates with an update
 * available scans axe-clean; Install and Check now are reachable by
 * keyboard; no modal traps focus. */
for (const theme of ["dark", "light"] as const) {
  test(`updates card — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_UPDATE_STATUS__ = {
          current_version: "0.1.0", channel: "beta", configured: true, check_on_launch: true, state: "available",
          available: { version: "0.2.0", notes: "## New\n- History", date_ms: null },
          progress: null, error: null, backup_path: null, checked_at_ms: null,
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Settings" }).click();
    await page.getByTestId("update-install").focus();
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("update-check")).toBeFocused();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1204 a11y (both themes): the recovery-code sheet scans
 * axe-clean; the code is focusable selectable text, Copy and Save are
 * reachable by keyboard, and "Start backups" stays disabled until the
 * confirmation is ticked. */
for (const theme of ["dark", "light"] as const) {
  test(`recovery code sheet — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_FIGUREHEAD__ = {
          state: "linked", email: "you@example.com", plan: "Free", device_name: "Studio Mac",
          features: [{ key: "cloudBackup", access: "included", tier: "free", on: true }],
          link: null, refreshed_at_ms: Date.now(), offline: false, notice: null, consent_version: 1, consent_current: 1,
        };
        w.__AIJ_VAULT_STATUS__ = { has_key: false, code_saved: false };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Settings" }).click();
    await expect(page.getByTestId("recovery-code")).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("recovery-copy")).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(page.getByTestId("recovery-save")).toBeFocused();
    await expect(page.getByTestId("recovery-done")).toBeDisabled();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1205 a11y (both themes): the Cloud backup panel and its backup
 * list (with the recovery-code field) scan axe-clean. */
for (const theme of ["dark", "light"] as const) {
  test(`cloud backup panel and list — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_FIGUREHEAD__ = {
          state: "linked", email: "you@example.com", plan: "Free", device_name: "Studio Mac",
          features: [{ key: "cloudBackup", access: "included", tier: "free", on: true }],
          link: null, refreshed_at_ms: Date.now(), offline: false, notice: null, consent_version: 1, consent_current: 1,
        };
        w.__AIJ_VAULT_STATUS__ = { has_key: false, code_saved: true };
        w.__AIJ_CLOUD_BACKUPS__ = [
          { id: "bak_1", label: "AI Juggler · Studio Mac", size_bytes: 400000, created_at: "2026-10-18T09:00:00Z", this_mac: false },
        ];
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Settings" }).click();
    await page.getByTestId("cloud-backup-restore").click();
    await expect(page.getByTestId("cloud-backup-code")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}

/** AJ-E1206 a11y (both themes): Your Macs on Figurehead and the
 * fingerprint comparison scan axe-clean; "Pair" stays disabled until the
 * match is confirmed; Escape closes without pairing. */
for (const theme of ["dark", "light"] as const) {
  test(`account macs and compare — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_FIGUREHEAD__ = {
          state: "linked", email: "you@example.com", plan: "Free", device_name: "Studio",
          features: [{ key: "fleetDiscovery", access: "included", tier: "free", on: true }],
          link: null, refreshed_at_ms: Date.now(), offline: false, notice: null, consent_version: 1, consent_current: 1,
        };
        w.__AIJ_ACCOUNT_MACS__ = {
          this_fingerprint: "7Q3K-M9XA-R2PD-W8HN", read_at_ms: Date.now(),
          macs: [{ device_id: "dev_mini", name: "Mac mini", platform: "macos", fingerprint: "C4TJ-V6LE-Y1QZ-B5SF", state: "not_paired", last_seen_at: null }],
        };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: "Fleet" }).click();
    await expect(page.getByTestId("account-mac")).toHaveCount(1);
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.getByTestId("account-mac-compare").click();
    await expect(page.getByTestId("compare-pair")).toBeDisabled();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("account-compare")).toHaveCount(0);
  });
}

// AJ-E1105: Settings › Privacy with everything listed and a report open.
for (const theme of ["dark", "light"] as const) {
  test(`settings — privacy, populated, report open — axe 0 — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_USAGE_STATS__ = {
          info: {
            included: true,
            chosen: "share",
            effective: "share",
            can_upload: true,
            reason: null,
            decided_at_ms: 1,
            consent_version: 1,
            current_consent_version: 1,
          },
          week: "2026-W42",
          rows: [
            { category: "environment", metric: "projects.count", dims: "", value: "4-5" },
            { category: "features", metric: "app.launch", dims: "", value: "6" },
            { category: "reliability", metric: "perf.cold_start_ms", dims: "bucket: 750-1500", value: "6" },
          ],
          reports: [
            { week: "2026-W41", status: "pending", reason: null, upload_at_ms: 2, next_try_ms: 3, attempts: 1, bytes: 1204, synced: false },
            { week: "2026-W40", status: "skipped", reason: "Sharing didn't cover the whole week — never sent", upload_at_ms: null, next_try_ms: null, attempts: 0, bytes: 988, synced: true },
          ],
          next_upload_ms: 2,
          last_error: "Couldn't reach Figurehead.",
          docs_url: "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/usage-stats.md",
          sync: { offered: true, on: true, email: "you@example.com", notice: null },
        };
        w.__AIJ_USAGE_STATS_REPORTS__ = { "pending/2026-W41": '{"schema":1,"week":"2026-W41","entries":[]}' };
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expect(page.getByRole("button", { name: "Settings" }).first()).toBeVisible();
    await page.keyboard.press("ControlOrMeta+Comma");
    await expect(page.getByTestId("settings-privacy")).toBeVisible();
    await page.getByTestId("privacy-report-view").first().click();
    await expect(page.getByTestId("privacy-report-json")).toBeVisible();
    expect(await scanSeriousWithConfirm(page)).toHaveLength(0);
  });
}
