import type { FixtureSet, InstanceInfo } from "../../src/lib/bridge";
import { expect } from "@playwright/test";

/**
 * Deterministic fixture sets for the visual harness (AJ-E802 Req.1):
 * every SessionStatus value, quota honesty tiers where the current
 * read-model carries them, and honest-empty variants.
 */
const baseAccounts = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in" },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in" },
  { id: "fake-desk", provider: "fake-desktop", label: "Fake Desk", plan: null, login_state: "logged_in", preview_opt_in: true },
];

const baseProfiles = [
  { id: "p-1", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: "opus", default_cwd: null },
  { id: "p-2", name: "Main", account_id: "claude-personal", account_label: "Personal", provider: "claude-code", model: null, default_cwd: null },
  { id: "p-3", name: "App", account_id: "fake-desk", account_label: "Fake Desk", provider: "fake-desktop", model: null, default_cwd: null },
];

// One session per SessionStatus (model.rs:188 + Stopped). Rows carry the
// full AJ-E401 read model: nulls where a real row would have no source
// (never zeros), quota/token/resource data only where measured.
const T0 = 1_728_800_000_000; // pinned clock base (deterministic)
const mk = (n: number, status: InstanceInfo["status"], label: string): InstanceInfo => {
  const work = label === "Work Max";
  const active = !["finished", "stopped", "failed", "handed_over"].includes(status);
  const attention: InstanceInfo["needs_attention"] =
    status === "waiting_user"
      ? "waiting_user"
      : status === "stuck"
        ? "stuck"
        : status === "quota_blocked"
          ? "quota_blocked"
          : status === "failed"
            ? "failed"
            : null;
  const quota: InstanceInfo["quota"] =
    status === "running"
      ? { used_percentage: 42.5, resets_at_ms: T0 + 3_600_000, confidence: "computed" }
      : status === "quota_blocked"
        ? { used_percentage: 100, resets_at_ms: T0 + 7_200_000, confidence: "measured" }
        : null;
  return {
    id: `sess-${n}`,
    launch_recipe_id: work ? "p-1" : "p-2",
    provider: "claude-code",
    // AJ-E510: false by default; the recovery captures flip it per row.
    adopted: false,
    account_id: work ? "claude-work" : "claude-personal",
    account_label: label,
    // AJ-E412: sess-8 carries the drift fixture (launched opus, observed
    // running sonnet-5 via an in-session /model change); other Work rows
    // are launch-only (not verified live); Personal rows have neither.
    model: n === 8 ? "sonnet-5" : work ? "opus" : null,
    launch_model: work ? "opus" : null,
    running_model: n === 8 ? "sonnet-5" : null,
    model_source: n === 8 ? "in-session" : work ? "launch" : "unknown",
    status,
    quota,
    tokens:
      status === "running"
        ? { input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 }
        : null,
    cpu_pct: status === "running" ? 3.5 : null,
    // A Claude Code session as measured on a real Mac (126–208 MB, AJ-E1535).
    rss_bytes: status === "running" ? 180_000_000 : null,
    child_count: active ? (status === "running" ? 2 : null) : 0,
    subtask_count: null,
    git_branch: work ? "feat/read-model" : null,
    git_dirty_count: null,
    needs_attention: attention,
    attention:
      attention === null
        ? null
        : {
            rule: attention === "waiting_user" ? "waiting_prompt" : attention === "stuck" ? "stuck_no_output" : "quota_exit",
            inputs:
              attention === "waiting_user"
                ? ["prompt alive", `no output ${n}m`]
                : attention === "stuck"
                  ? [`no output ${n}m`, "threshold 5m", "cpu 0.3%", `transcript stale ${n}m`]
                  : ["exit code 1", "rate-limit signature in output"],
            since_ms: T0 - n * 3_500_000,
          },
    started_at_ms: T0 - n * 3_600_000,
    last_activity_ms: T0 - n * 3_500_000,
    machine_id: "this-mac",
    kind: "terminal",
    isolation_label: null,
    project_id: work ? "proj-1" : "proj-2",
    // Every session past starting is on a conversation (it can be carried
    // to another account); the running one keeps the real-looking id.
    conversation_id: status === "running" ? "822e4125-e07d-4866-9954-ec9295efb079" : status === "starting" ? null : `conv-${n}`,
  };
};

export const allStatuses: FixtureSet = {
  accounts: baseAccounts,
  profiles: baseProfiles,
  sessions: [
    mk(1, "starting", "Work Max"),
    mk(2, "running", "Work Max"),
    mk(3, "waiting_user", "Personal"),
    mk(4, "stuck", "Work Max"),
    mk(5, "quota_blocked", "Personal"),
    mk(6, "finished", "Work Max"),
    mk(7, "stopped", "Personal"),
    mk(8, "failed", "Work Max"),
    mk(9, "handed_over", "Personal"),
    // AJ-E413: desktop-kind rows under the fake-desktop provider —
    // running, exited, crashed. Desktop apps in this fixture have no
    // transcript, quota feed, conversation, or model: those fields are
    // null (D-06), resources only where measured.
    {
      ...mk(10, "running", "Personal"),
      id: "sess-desktop-1",
      launch_recipe_id: "p-3",
      account_id: "fake-desk",
      account_label: "Fake Desk",
      provider: "fake-desktop",
      kind: "desktop",
      isolation_label: "Single-account app — Juggler tracks, cannot isolate",
      model: null,
      launch_model: null,
      running_model: null,
      model_source: "unknown",
      quota: null,
      tokens: null,
      cpu_pct: 2.4,
      rss_bytes: 88_000_000,
      child_count: 1,
      needs_attention: null,
      attention: null,
      git_branch: null,
      project_id: "proj-3",
      conversation_id: null,
    },
    {
      ...mk(11, "finished", "Personal"),
      id: "sess-desktop-2",
      launch_recipe_id: "p-3",
      account_id: "fake-desk",
      account_label: "Fake Desk",
      provider: "fake-desktop",
      kind: "desktop",
      isolation_label: "Single-account app — Juggler tracks, cannot isolate",
      model: null,
      launch_model: null,
      running_model: null,
      model_source: "unknown",
      quota: null,
      tokens: null,
      cpu_pct: null,
      rss_bytes: null,
      child_count: 0,
      needs_attention: null,
      attention: null,
      git_branch: null,
      project_id: "proj-3",
      conversation_id: null,
    },
    {
      ...mk(12, "failed", "Personal"),
      id: "sess-desktop-3",
      launch_recipe_id: "p-3",
      account_id: "fake-desk",
      account_label: "Fake Desk",
      provider: "fake-desktop",
      kind: "desktop",
      isolation_label: "Single-account app — Juggler tracks, cannot isolate",
      model: null,
      launch_model: null,
      running_model: null,
      model_source: "unknown",
      quota: null,
      tokens: null,
      cpu_pct: null,
      rss_bytes: null,
      child_count: 0,
      // Crashed (exit code 7): the failed badge comes from the status;
      // no heuristic rule was recorded — attention stays null.
      attention: null,
      git_branch: null,
      project_id: "proj-3",
      conversation_id: null,
    },
  ],
  projects: [
    { id: "proj-1", root: "/Users/dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0 - 60_000, active_instance: null, accounts_used: ["Work Max"], instance_count: 5, history_sessions: 12, history_last_used_ms: null, repo: null },
    { id: "proj-2", root: "/Users/dev/storefront", name: "storefront", last_activity_ms: T0 - 3_600_000, active_instance: null, accounts_used: ["Personal"], instance_count: 4, history_sessions: 7, history_last_used_ms: null, repo: null },
    { id: "proj-3", root: "/Users/dev/sacrificial", name: "sacrificial", last_activity_ms: T0 - 600_000, active_instance: null, accounts_used: ["Fake Desk"], instance_count: 3, history_sessions: 0, history_last_used_ms: null, repo: null },
  ],
  stats: { accounts: 3, profiles: 3, active_sessions: 3, ledger_degraded: false },
};

export const honestEmpty: FixtureSet = {
  accounts: [],
  profiles: [],
  sessions: [],
  projects: [],
  stats: { accounts: 0, profiles: 0, active_sessions: 0, ledger_degraded: false },
};

export const readyHome: FixtureSet = {
  accounts: [
    { id: "a1", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_out" },
    { id: "a2", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in" },
  ],
  profiles: [],
  sessions: [],
  projects: [],
  stats: { accounts: 2, profiles: 0, active_sessions: 0, ledger_degraded: false },
};

/**
 * AJ-E805: the product's promised scale — 10 Accounts over three
 * Providers, 50 Instances in every status, 20 Projects.
 */
export const atScale: FixtureSet = (() => {
  const providers = ["claude-code", "codex", "zcode"] as const;
  const accounts = Array.from({ length: 10 }, (_, i) => ({
    id: `acct-${i}`,
    provider: providers[i % 3],
    label: `Account ${String(i).padStart(2, "0")}`,
    plan: null,
    login_state: "logged_in",
  }));
  const profiles = accounts.map((account, i) => ({
    id: `p-${i}`,
    name: "Main",
    account_id: account.id,
    account_label: account.label,
    provider: account.provider,
    model: null,
    default_cwd: null,
  }));
  const statuses: InstanceInfo["status"][] = [
    "running", "running", "running", "waiting_user", "stuck", "quota_blocked", "starting", "finished", "stopped", "failed",
  ];
  const sessions = Array.from({ length: 50 }, (_, n) => {
    const account = accounts[n % 10]!;
    const base = mk(n + 1, statuses[n % statuses.length]!, account.label);
    return {
      ...base,
      id: `scale-${n}`,
      launch_recipe_id: `p-${n % 10}`,
      provider: account.provider,
      account_id: account.id,
      account_label: account.label,
      project_id: `proj-${n % 20}`,
      git_branch: `feat/task-${n}`,
    };
  });
  const projects = Array.from({ length: 20 }, (_, i) => ({
    id: `proj-${i}`,
    root: `/Users/dev/project-${String(i).padStart(2, "0")}`,
    name: `project-${String(i).padStart(2, "0")}`,
    last_activity_ms: T0 - i * 600_000,
    active_instance: null,
    accounts_used: [accounts[i % 10]!.label],
    instance_count: sessions.filter((s) => s.project_id === `proj-${i}`).length,
    history_sessions: 10 + i, history_last_used_ms: null, repo: null,
  }));
  return {
    accounts,
    profiles,
    sessions,
    projects,
    stats: { accounts: 10, profiles: 10, active_sessions: sessions.filter((s) => ["running", "starting", "waiting_user", "stuck", "quota_blocked"].includes(s.status)).length, ledger_degraded: false },
  };
})();

const CLEANUP_ROW = (pid: number, state: string) => ({
  pid,
  pgid: 8100,
  role: "helper",
  state,
  state_label:
    state === "orphaned"
      ? "Still running after its session closed"
      : state === "adopted"
        ? "Running — picked up after restart"
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

/** The Cleanup centre inventory for a gallery state (AJ-E503/E804). */
export function cleanupFixture(state: string) {
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

/** Switch preflights for the wizard and 1-click doors (AJ-E206/E804). */
export const DOOR_WIZARD = {
  mechanism: "transplant",
  fidelity: "F2",
  fidelity_words: "Same conversation, new account (compacted)",
  score: 0.78,
  warnings: ["This conversation was compacted. Early detail carries as text, not as live memory — it may ask about early decisions."],
  zombies: [],
  diff_snapshotted: true,
  source_live: true,
  experimental_copy: "Beta, not proven yet: the whole session moves, with its subagents and file history. The source stays resumable.",
  door: "wizard" as const,
  wizard_steps: ["Target", "What's carried", "Confirm"],
  wizard_reasons: ["This isn't the usual account for this project."],
  usual_account: false,
  carry: { carried: ["chat transcript", "rules & guidelines", "uncommitted git diff"], regenerated: [], dropped: ["todo list"] },
};
export const DOOR_ONECLICK = {
  ...DOOR_WIZARD,
  fidelity: "F4",
  fidelity_words: "Continues exactly",
  warnings: [],
  door: "one-click" as const,
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: { carried: ["chat transcript", "rules & guidelines", "uncommitted git diff", "MCP config", "skills & commands", "model settings"], regenerated: [], dropped: [] },
};

/**
 * D-25: opens a top-level screen the way a user does — Home, Accounts,
 * History and Settings sit in the header (Fleet too, once a Mac is
 * paired); Triage, Overview, Usage, Fleet and Cleanup wait under More.
 */
export async function goTo(page: import("@playwright/test").Page, name: string): Promise<void> {
  const main = page.getByRole("navigation", { name: "Main" });
  const direct = main.getByRole("button", { name, exact: true });
  if ((await direct.count()) > 0) {
    await direct.first().click();
    return;
  }
  await main.getByRole("button", { name: "More", exact: true }).click();
  await page.locator("#nav-more").getByRole("button", { name: new RegExp(`^${name}\\b`) }).click();
}

/** AJ-E2110: opens one of Settings' panes from its sidebar. */
export async function settingsPane(page: import("@playwright/test").Page, name: string): Promise<void> {
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name, exact: true }).click();
}

/**
 * AJ-E2104: the rail's View (Group by, Show, Only) sits behind its View
 * button (⇧V); open it (once) before using it.
 */
export async function openFilters(page: import("@playwright/test").Page): Promise<void> {
  const panel = page.getByTestId("rail-view");
  if (await panel.isVisible()) return;
  await page.getByTestId("rail-view-button").click();
  await panel.waitFor();
}

/** AJ-E2104: the rail's View as kept, then the page again. */
export async function railViewAs(page: import("@playwright/test").Page, view: Record<string, unknown>): Promise<void> {
  await page.evaluate((value) => localStorage.setItem("aij.rail.view.v1", JSON.stringify(value)), view);
  await page.reload();
  await expect(page.getByTestId("rail")).toBeVisible();
}

/** AJ-E2104: the Project lens (projects' groups and their headers). */
export async function projectLens(page: import("@playwright/test").Page): Promise<void> {
  await railViewAs(page, { group: "project" });
}

/** AJ-E2104: Only › Mac in the View ("all" puts it back). */
export async function onlyMac(page: import("@playwright/test").Page, value: string): Promise<void> {
  await openFilters(page);
  await page.getByLabel("Only this mac").selectOption(value === "all" ? "" : value);
}

/** AJ-E2104: a session started outside AI Juggler, by its page (its
 * row's menu: Open its page). Quiet places shown, so its row is drawn. */
export async function openOutsidePage(page: import("@playwright/test").Page): Promise<void> {
  await railViewAs(page, { group: "place", show: { quiet: true } });
  await page.locator("[data-testid='rail'] [data-conversation-id]").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: "Open its page" }).click();
}

/**
 * D-25: a session's ranked switch list sits behind "Switch account…"
 * (open by itself only when the session hit its limit). Opens it once.
 */
export async function openSwitch(page: import("@playwright/test").Page): Promise<void> {
  const section = page.getByTestId("handover-section");
  if (await section.evaluate((node) => (node as HTMLDetailsElement).open).catch(() => false)) return;
  await page.getByTestId("header-switch").click();
  await expect(section).toHaveJSProperty("open", true);
}

/**
 * D-25 (AJ-E1020): at 900 px and wider the rail shows exactly the stored
 * density (Settings › Appearance); narrower, a non-Minimal rail folds to
 * its icon column by itself. Waits for the rail and, where it is the
 * list, for the list.
 */
export async function expectRail(page: import("@playwright/test").Page): Promise<void> {
  const rail = page.getByTestId("rail");
  await expect(rail).toBeVisible();
  const stored = (await page.evaluate(() => localStorage.getItem("aij.density"))) ?? "comfortable";
  const wide = (page.viewportSize()?.width ?? 1280) >= 900;
  await expect(rail).toHaveAttribute("data-density", wide || stored === "minimal" ? stored : "minimal");
  if (wide && stored !== "minimal") await expect(page.getByTestId("instance-board")).toBeVisible();
}

/**
 * D-25 (AJ-E1020): a narrow window folds the rail to session icons; a
 * project row needs the list, which ⌘B opens for now. No-op when the
 * list already shows.
 */
export async function openRailList(page: import("@playwright/test").Page): Promise<void> {
  const rail = page.getByTestId("rail");
  await expect(rail).toBeVisible();
  if ((await rail.getAttribute("data-density")) === "minimal") {
    await page.keyboard.press("ControlOrMeta+b");
    await expect(page.getByTestId("instance-board")).toBeVisible();
  }
}
