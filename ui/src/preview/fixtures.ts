/**
 * The design preview's base fixture set (plain browser + visual harness).
 * The harness (AJ-E802) seeds `window.__AIJ_FIXTURE__` before this module
 * loads; mock calls then serve the seeded set instead of the defaults.
 */
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { QualitySignalsInfo } from "../types/generated/QualitySignalsInfo";
import type { StatsInfo } from "../types/generated/StatsInfo";

// --- design-preview mocks (plain-browser + visual harness) ----------------
// The harness (AJ-E802) seeds `window.__AIJ_FIXTURE__` before this module
// loads; mock calls then serve the seeded fixture set (deterministic
// captures) instead of the defaults below.

export interface FixtureSet {
  accounts: AccountInfo[];
  profiles: LaunchRecipeInfo[];
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  stats: StatsInfo;
}

declare global {
  interface Window {
    __AIJ_FIXTURE__?: Partial<FixtureSet>;
  }
}

export const seeded = (typeof window !== "undefined" && window.__AIJ_FIXTURE__) || {};

/** A `YYYY-MM-DD` string N days from today (fixture dates move with the clock). */
function renewalFixtureInDays(days: number): string {
  const target = new Date(Date.now() + days * 86_400_000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${target.getFullYear()}-${pad(target.getMonth() + 1)}-${pad(target.getDate())}`;
}

export const MOCK_ACCOUNTS: AccountInfo[] = [
  // AJ-E3206: each folder's CLI App and its Account wear the slots the
  // backend would give them (the first App, the first Account: 0).
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: Some("max-20x"), login_state: "logged_in", cleanup_opt_in: false, renewal_date: Some(renewalFixtureInDays(6)), preview_opt_in: false, in_place: false, app_colour_slot: 0, subscription_id: "acct:preview:k-claude-work", account_colour_slot: 0 },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: None(), login_state: "logged_out", cleanup_opt_in: false, renewal_date: None(), preview_opt_in: false, in_place: false, app_colour_slot: 1, subscription_id: null, account_colour_slot: null },
];

export const MOCK_PROFILES: LaunchRecipeInfo[] = [
  { id: "p-1", name: "Deep work", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: Some("opus"), default_cwd: None(), restart_on_crash: null },
  { id: "p-2", name: "Quick fixes", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: Some("sonnet"), default_cwd: None(), restart_on_crash: null },
  { id: "p-3", name: "Weekend build", account_id: "claude-personal", account_label: "Personal", provider: "claude-code", model: None(), default_cwd: None(), restart_on_crash: null },
];

export const MOCK_QUALITY: Record<string, QualitySignalsInfo> = {
  // sess-8 sorts first (needs-me) and is what the captures click; sess-2
  // is the running row.
  "sess-8": {
    tokens_bars: [300, 410, 380, 500, 440],
    avg_tokens_per_min: 406,
    error_label: "1 error / 12 turns",
    errors_not_measurable: false,
    context_used: null,
    context_window: null,
    cumulative: { input: 4_200, output: 1_800, cache_read: 20, cache_write: 40, reasoning: 8 },
    session_cost_usd: 0.19,
    priced_model: "opus",
    quota_forecast: "Not using its limit right now",
    stuck: { rule: "stuck_no_output", inputs: ["no output 8m", "threshold 5m", "cpu 0.3%", "transcript stale 8m"], since_ms: Date.now() - 8 * 60_000 }, stuck_episodes: 1, end_context: null,
  },
  "sess-2": {
    tokens_bars: [420, 510, 380, 600, 540],
    avg_tokens_per_min: 490,
    error_label: "2 errors / 41 turns",
    errors_not_measurable: false,
    context_used: null,
    context_window: null,
    cumulative: { input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 },
    session_cost_usd: 0.69,
    priced_model: "opus",
    quota_forecast: "Not using its limit right now",
    stuck: null, stuck_episodes: 0, end_context: null,
  },
  "sess-live-1": {
    tokens_bars: [420, 510, 380, 600, 540],
    avg_tokens_per_min: 490,
    error_label: "2 errors / 41 turns",
    errors_not_measurable: false,
    context_used: null,
    context_window: null,
    cumulative: { input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 },
    session_cost_usd: 0.69,
    priced_model: "opus",
    quota_forecast: "Not using its limit right now",
    stuck: null, stuck_episodes: 0, end_context: null,
  },
};

export const MOCK_STATS: StatsInfo =
  seeded.stats ?? {
    accounts: 2,
    profiles: 3,
    active_sessions: 0,
    ledger_degraded: false,
  };

// Mock rows mirror the AJ-E401 read-model fixtures: one full row, one
// quota-less row, one model-less/terminal row — nulls where the backend
// reports absence, never zeros or guessed names.
export const MOCK_SESSIONS: InstanceInfo[] = seeded.sessions ?? [
  {
    id: "sess-live-1",
    adopted: false,
    launch_recipe_id: "p-1",
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: Some("opus"),
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: Some({
      used_percentage: 42.5,
      resets_at_ms: Date.now() + 3_600_000,
      confidence: "computed",
    }),
    tokens: Some({ input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 }),
    cpu_pct: Some(3.5),
    rss_bytes: Some(640_000_000),
    child_count: Some(2),
    subtask_count: null,
    git_branch: Some("feat/read-model"),
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: Date.now() - 1_200_000,
    last_activity_ms: Date.now() - 30_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-ai-juggler",
    conversation_id: "822e4125-e07d-4866-9954-ec9295efb079",
  },
  {
    id: "sess-done-1",
    adopted: false,
    launch_recipe_id: "p-3",
    provider: "claude-code",
    account_id: "claude-personal",
    account_label: "Personal",
    model: null,
    launch_model: null,
    running_model: null,
    model_source: "unknown",
    status: "finished",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: Some(0),
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: Date.now() - 86_400_000,
    last_activity_ms: Date.now() - 80_000_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/payments-api",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-payments",
    conversation_id: null,
  },
  {
    id: "sess-done-2",
    adopted: false,
    launch_recipe_id: "p-2",
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: Some("sonnet"),
    launch_model: "sonnet",
    running_model: null,
    model_source: "launch",
    status: "finished",
    quota: Some({ used_percentage: 100, resets_at_ms: Date.now() + 7_200_000, confidence: "measured" }),
    tokens: Some({ input: 98_000, output: 51_000, cache_read: 9_000, cache_write: 4_000, reasoning: 800 }),
    cpu_pct: null,
    rss_bytes: null,
    child_count: Some(0),
    subtask_count: null,
    git_branch: Some("main"),
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: Date.now() - 43_200_000,
    last_activity_ms: Date.now() - 600_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/payments-api",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-payments",
    conversation_id: "c3f10b52-9a41-4f0e-8f2a-7717db9c04aa",
  },
];

export const MOCK_PROJECTS: ProjectInfo[] = seeded.projects ?? [
  {
    id: "proj-ai-juggler",
    root: "/Users/you/Dev/ai-juggler",
    name: "ai-juggler",
    last_activity_ms: Date.now() - 60_000,
    active_instance: MOCK_SESSIONS[0] ?? null,
    accounts_used: ["Work Max"],
    instance_count: 1,
    history_sessions: 0, history_last_used_ms: null, repo: null, mark: { letters: "AJ", swatch: 0, shows: "letters" },
    missing: false,
  },
  {
    id: "proj-payments",
    root: "/Users/you/Dev/payments-api",
    name: "payments-api",
    last_activity_ms: Date.now() - 3_600_000,
    active_instance: null,
    accounts_used: ["Personal", "Work Max"],
    instance_count: 2,
    history_sessions: 14, history_last_used_ms: null, repo: null, missing: false, mark: { letters: "PA", swatch: 1, shows: "letters" },
  },
];

function Some<T>(value: T): T | null {
  return value;
}

function None<T>(): T | null {
  return null;
}

/** The preview's one tool, for mock answers that name a tool. */
export const PREVIEW_TOOL = { id: "claude-code", name: "Claude Code" };
