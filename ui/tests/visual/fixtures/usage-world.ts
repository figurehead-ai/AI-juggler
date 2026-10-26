/**
 * 0.1.5 — the usage gallery's world: seven profiles (six Claude profiles
 * and Codex) on four accounts, seven projects, sessions open in the
 * apps, and limit readings in every state the runway shows (fresh, hit
 * its limit, reset since the last reading, measuring, never measured).
 * The preview's synthetic history (src/preview/usage-history.ts) draws
 * 120 days of usage with a weekly rhythm on these profiles.
 */
export const T0 = Date.UTC(2026, 9, 23, 14, 40); // Fri 23 Oct 2026, 14:40 UTC

const account = (id: string, label: string, provider: string, plan: string | null) => ({
  id,
  provider,
  label,
  plan,
  login_state: "logged_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: true,
});

export const ACCOUNTS = [
  account("claude-work", "Work", "claude-code", "max-20x"),
  account("codex-main", "Codex Main", "codex", "pro"),
  account("claude-client-a", "Client A", "claude-code", "team"),
  account("claude-lab", "Lab", "claude-code", "max-20x"),
  account("claude-personal", "Personal", "claude-code", "pro"),
  account("claude-work-2", "Work-2", "claude-code", "max-20x"),
  account("claude-client-b", "Client B", "claude-code", "team"),
];

const profile = (id: string, folder: string) => ({ id, folder, plan: null, renewal_date: null, fee_usd: null });
export const SUBSCRIPTIONS = [
  { key: "max", tool: "claude-code", state: "signed_in", email: "o…@work.com", organisation: null, reason: null, plan: "Max", renewal_date: "2026-11-02", fee_usd: 200, differs: [], profiles: [profile("claude-work", "~/Library/Application Support/ClaudeCodeProfiles/work"), profile("claude-work-2", "~/Library/Application Support/ClaudeCodeProfiles/work-2"), profile("claude-lab", "~/Library/Application Support/ClaudeCodeProfiles/lab")] },
  { key: "gpt", tool: "codex", state: "signed_in", email: "o…@work.com", organisation: null, reason: null, plan: "Pro", renewal_date: null, fee_usd: 200, differs: [], profiles: [profile("codex-main", "~/.codex")] },
  { key: "acme", tool: "claude-code", state: "signed_in", email: "o…@acme.com", organisation: "Acme team", reason: null, plan: "Team", renewal_date: null, fee_usd: null, differs: [], profiles: [profile("claude-client-a", "~/Library/Application Support/ClaudeCodeProfiles/client-a"), profile("claude-client-b", "~/Library/Application Support/ClaudeCodeProfiles/client-b")] },
  { key: "pro", tool: "claude-code", state: "signed_in", email: "o…@home.com", organisation: null, reason: null, plan: "Pro", renewal_date: null, fee_usd: 20, differs: [], profiles: [profile("claude-personal", "~/Library/Application Support/ClaudeCodeProfiles/bookauth-olivia")] },
];

const project = (id: string, name: string, swatch: number, used: string[]) => ({
  id,
  root: `/Users/you/code/${name}`,
  name,
  last_activity_ms: T0 - 12 * 60_000,
  active_instance: null,
  accounts_used: used,
  instance_count: 0,
  history_sessions: 40,
  history_last_used_ms: T0 - 12 * 60_000,
  repo: null,
  missing: false,
  mark: { letters: name.slice(0, 2).toUpperCase(), swatch, shows: "letters" },
});

export const PROJECTS = [
  project("proj-web", "web-app", 0, ["Work", "Codex Main"]),
  project("proj-api", "api", 3, ["Codex Main", "Work"]),
  project("proj-acme", "acme-portal", 2, ["Client A", "Client B"]),
  project("proj-aij", "ai-juggler", 4, ["Work", "Lab"]),
  project("proj-site", "site", 7, ["Personal"]),
  project("proj-docs", "docs", 5, ["Work-2"]),
  project("proj-data", "data-pipeline", 1, ["Lab", "Work"]),
];

const conversation = (id: string, accountId: string, label: string, tool: string, projectId: string, title: string, live: boolean, state: string | null, app: string | null) => ({
  id,
  tool,
  account_id: accountId,
  account_label: label,
  project_id: projectId,
  cwd: `/Users/you/code/${PROJECTS.find((p) => p.id === projectId)!.name}`,
  title,
  git_branch: "main",
  started_ms: T0 - 3 * 3_600_000,
  last_activity_ms: T0 - 2 * 60_000,
  tokens: { input: 4_200_000, output: 900_000, cache_read: 52_000_000, cache_write: 400_000, reasoning: 0 },
  live,
  session_id: null,
  places: live && app ? [{ app, app_id: null, profile: label, instance: null, pid: 4242, kind: "desktop", app_pid: 4200, label: `${app} · ${label}` }] : [],
  pid: live ? 4242 : null,
  state,
  probably: false,
  viewed_ms: live ? T0 - 2 * 60_000 : null,
  background: false,
  inactive: false,
});

export const CONVERSATIONS = [
  conversation("c-checkout", "claude-work-2", "Work-2", "claude-code", "proj-web", "Fix the flaky checkout test", true, "asks_you", "Claude"),
  conversation("c-billing", "claude-work", "Work", "claude-code", "proj-web", "Refactor the billing module", true, "working", "Terminal"),
  conversation("c-oauth", "codex-main", "Codex Main", "codex", "proj-api", "Move sign-in to OAuth", true, "working", "Codex"),
  conversation("c-acme", "claude-client-a", "Client A", "claude-code", "proj-acme", "Invoice export for Acme", true, "idle", "Claude"),
  conversation("c-rail", "claude-lab", "Lab", "claude-code", "proj-aij", "Rail grouping by app", false, null, null),
  conversation("c-blog", "claude-personal", "Personal", "claude-code", "proj-site", "Blog redesign", true, "working", "Claude"),
  conversation("c-notes", "claude-work-2", "Work-2", "claude-code", "proj-docs", "Release notes draft", false, null, null),
];

const window = (state: string, percentage: number | null, resetsInMin: number | null, readMinAgo: number | null, burned = 0) => ({
  state,
  percentage,
  badge: "measured",
  stale_note_ms: null,
  resets_at_ms: resetsInMin === null ? null : T0 + resetsInMin * 60_000,
  reset_basis: resetsInMin === null ? null : "measured",
  measured_at_ms: readMinAgo === null ? null : T0 - readMinAgo * 60_000,
  burned_tokens: burned,
  source: readMinAgo === null ? null : "desktop_app",
  source_words: readMinAgo === null ? null : "the Claude app",
  burned_credits: null,
  headroom_tokens: null,
});
const display = (id: string, five: unknown, weekly: unknown, extra: Record<string, unknown> = {}) => ({
  account: id,
  five_hour: five,
  five_hour_omitted_reason: null,
  weekly,
  weekly_omitted_reason: null,
  model_weeks: [],
  ...extra,
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: { state: "wont-exhaust", tte_min: null, range_min: null, range_max: null, badge: "measured" },
  unavailable_reason: null,
  credits: null,
});

/** A week on one model only (Claude's "Fable only"), read by Claude
 * Code's usage check. */
const fable = (percentage: number, resetsInMin: number) => ({
  model: "Fable",
  window: { ...window("tracking", percentage, resetsInMin, 12), source: "usage_cache", source_words: "Claude Code's last usage check" },
});

/** Every runway state: Max fresh, with Fable's own week beside its week;
 * Codex fresh; Acme (a team seat) hit its 5-hour limit, and its only week
 * is Fable's; Pro's last reading is from before a reset. Lab is never
 * measured. */
export const QUOTA_DISPLAY = [
  display("claude-work", window("tracking", 38, 40, 12), window("tracking", 41, 3 * 24 * 60, 12), { model_weeks: [fable(22, 3 * 24 * 60)] }),
  display("codex-main", { ...window("tracking", 22, 180, 5), source: "rollout", source_words: "Codex" }, { ...window("tracking", 47, 6 * 24 * 60, 5), source: "rollout", source_words: "Codex" }),
  display("claude-client-a", window("exhausted", 100, 4 * 60 + 20, 38), null, {
    weekly_omitted_reason: "Claude Code reports a weekly limit only for Fable",
    model_weeks: [fable(36, 2 * 24 * 60)],
  }),
  display("claude-personal", window("reset", 40, null, 13 * 60), window("tracking", 12, 5 * 24 * 60, 13 * 60)),
  display("claude-lab", window("unmeasured", null, null, null, 2_400_000), window("unmeasured", null, null, null, 9_100_000)),
];

const recipe = (id: string, account: (typeof ACCOUNTS)[number], name: string) => ({ id, name, account_id: account.id, account_label: account.label, provider: account.provider, model: null, default_cwd: null, restart_on_crash: null });
export const RECIPES = ACCOUNTS.map((account, i) => recipe(`r-${i}`, account, "Default"));

/** web-app's checkout: a lot not committed, ahead of its upstream. */
export const PROJECT_GIT = {
  "proj-web": { branch: "main", changed: 146, untracked: 35, ahead: 2, behind: 0, worktree: false, created_by_juggler: false, path: "/Users/you/code/web-app" },
};

export const USAGE_WORLD = {
  accounts: ACCOUNTS,
  subscriptions: SUBSCRIPTIONS,
  projects: PROJECTS,
  conversations: CONVERSATIONS,
  sessions: [],
  profiles: RECIPES,
  stats: { accounts: 7, profiles: 0, active_sessions: 0, ledger_degraded: false },
};
