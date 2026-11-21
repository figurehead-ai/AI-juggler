/**
 * AJ-E3816 — the Watches view's preview data, generic as the approved
 * mock-up's: sessions "api", "web", "Fix the login redirect" and "Draft
 * the settings page"; Apps "Agent CLI", "Agent app one", "Agent app two";
 * projects "shop" and "docs-site"; the account "Work"; "Release pool".
 * Each state of the mock (`list`, `create`, `edit`, `empty`, `ended`,
 * `many`) as the stand-in's seam. Every watch's words come from the
 * stand-in echo, held to the backend's own cases. Used by the component
 * tests and the gallery; nothing here is fetched or written.
 */
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { CatalogueEntry } from "../types/generated/CatalogueEntry";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { Lifetime } from "../types/generated/Lifetime";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { Watch } from "../types/generated/Watch";
import type { WatchDraftInfo } from "../types/generated/WatchDraftInfo";
import type { WatchInfo } from "../types/generated/WatchInfo";
import type { WatchTarget } from "../types/generated/WatchTarget";
import type { ToolSpecInfo } from "../types/generated/ToolSpecInfo";
import { PREVIEW_TOOLS } from "./tools";
import type { DraftWords, WatchViewSeam } from "../lib/ipc/watch-view";
import type { AppLike } from "../components/watches/watches-view";
import { echoParts, joined, nextMidnight, type EchoClock, type EchoNames } from "./watch-echo";

/** 19 Nov 2026, 15:00 local. */
export const WATCHES_NOW = new Date(2026, 10, 19, 15, 0, 0, 0).getTime();
const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

const ACCOUNTS: AccountInfo[] = [
  { id: "acct-work", provider: "agent-cli", label: "Work", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "acct-studio", provider: "agent-one", label: "Studio", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "acct-home", provider: "agent-two", label: "Home", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];

const APPS: LaunchRecipeInfo[] = [
  { id: "p-cli", name: "Agent CLI", account_id: "acct-work", account_label: "Work", provider: "agent-cli", model: null, default_cwd: null, restart_on_crash: null },
  { id: "p-one", name: "Agent app one", account_id: "acct-studio", account_label: "Studio", provider: "agent-one", model: null, default_cwd: null, restart_on_crash: null },
  { id: "p-two", name: "Agent app two", account_id: "acct-home", account_label: "Home", provider: "agent-two", model: null, default_cwd: null, restart_on_crash: null },
];

function projectInfo(id: string, name: string, at: number): ProjectInfo {
  return { id, root: `/Users/me/${name}`, name, last_activity_ms: at, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false };
}

const PROJECTS: ProjectInfo[] = [
  projectInfo("proj-shop", "shop", WATCHES_NOW - 5 * MIN),
  projectInfo("proj-docs", "docs-site", WATCHES_NOW - 3 * HOUR),
];

function session(id: string, label: string, app: string, project: string, status: string, ago: number): InstanceInfo {
  const recipe = APPS.find((a) => a.id === app)!;
  return {
    id,
    label,
    launch_recipe_id: app,
    provider: recipe.provider,
    account_id: recipe.account_id,
    account_label: recipe.account_label,
    model: null,
    launch_model: null,
    running_model: null,
    model_source: "unknown",
    status,
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: WATCHES_NOW - ago - HOUR,
    last_activity_ms: WATCHES_NOW - ago,
    machine_id: "this-mac",
    cwd: `/Users/me/${project === "proj-shop" ? "shop" : "docs-site"}`,
    kind: recipe.provider === "agent-cli" ? "terminal" : "desktop",
    adopted: false,
    project_id: project,
    conversation_id: null,
    isolation_label: null,
  };
}

const SESSIONS: InstanceInfo[] = [
  session("s-api", "api", "p-cli", "proj-shop", "running", 1 * MIN),
  session("s-web", "web", "p-one", "proj-shop", "running", 2 * MIN),
  session("s-login", "Fix the login redirect", "p-one", "proj-shop", "idle", 20 * MIN),
  session("s-draft", "Draft the settings page", "p-two", "proj-docs", "idle", 40 * MIN),
];

/** The Apps, by App id (each folder's CLI App, named as the mock's). */
export const WATCHES_APPS: AppLike[] = [
  { id: "cli:acct-work", kind: "cli", name: "Agent CLI", tool: "agent-cli", folder: { id: "acct-work" } },
  { id: "cli:acct-studio", kind: "cli", name: "Agent app one", tool: "agent-one", folder: { id: "acct-studio" } },
  { id: "cli:acct-home", kind: "cli", name: "Agent app two", tool: "agent-two", folder: { id: "acct-home" } },
];

/** Their stored rows, which name them (`__AIJ_STORED_APPS__`). */
export const WATCHES_STORED_APPS = WATCHES_APPS.map((app) => ({ id: app.id, kind: "cli", tool: app.tool ?? "", host_app_id: null, label: app.name, folder_id: app.folder?.id ?? null, account_id: null, last_seen_ms: 0 }));

/** The shell's fixture (`__AIJ_FIXTURE__`) the gallery opens with. */
export const WATCHES_FIXTURE = { accounts: ACCOUNTS, profiles: APPS, sessions: SESSIONS, projects: PROJECTS };

/** "Release pool", with three members (`__AIJ_POOLS__`). */
export const WATCHES_POOLS = {
  pools: [{ id: "pool-rel", name: "Release pool", project_id: "proj-shop" }],
  members: [
    { session_id: "s-api", pool_id: "pool-rel", role: "lead" },
    { session_id: "s-web", pool_id: "pool-rel", role: "member" },
    { session_id: "s-login", pool_id: "pool-rel", role: "member" },
  ],
};

const NAMES: Record<string, string> = {
  "session:s-api": "api",
  "session:s-web": "web",
  "session:s-login": "Fix the login redirect",
  "session:s-draft": "Draft the settings page",
  "app:cli:acct-work": "Agent CLI",
  "app:cli:acct-studio": "Agent app one",
  "app:cli:acct-home": "Agent app two",
  "project:proj-shop": "shop",
  "account:acct-work": "Work",
  "pool:pool-rel": "Release pool",
};

/** The names the stand-in echo quotes: these, and `more` (a seam's, by
 * `kind:id` or by id). */
function namesWith(more: Record<string, string> = {}): EchoNames {
  return {
    target: (t: WatchTarget) => (t.kind === "any" ? null : (more[`${t.kind}:${t.id}`] ?? more[t.id] ?? NAMES[`${t.kind}:${t.id}`] ?? null)),
    agent: (id: string) => (id === "agent-reviewer" ? "reviewer" : null),
  };
}

function clockNow(): EchoClock {
  const now = Date.now();
  return { nowMs: now, offsetHours: -new Date(now).getTimezoneOffset() / 60 };
}

// ---------------------------------------------------------------------------
// The catalogue: what each target can report (D-127)
// ---------------------------------------------------------------------------

type P = CatalogueEntry["param"];
const minutes = (name: string, min: number, max: number, required: boolean): P => ({ name, kind: "minutes", min, max, required });
const E = (code: string, label: string, told: string, need: boolean, param: P = null): CatalogueEntry => ({ code, label, told, need, param });

const C = {
  finishes: E("finishes", "Finishes", "finished", false),
  asks_you: E("asks_you", "Asks you", "asks you", true, minutes("after_min", 0, 120, false)),
  stuck: E("stuck", "Looks stuck", "looks stuck", true, minutes("for_min", 0, 240, false)),
  fails: E("fails", "Fails", "failed", true),
  ends: E("ends", "Ends", "ended", false),
  idle_for: E("idle_for", "Is idle", "has been idle", false, minutes("min", 5, 480, true)),
  hits_limit: E("hits_limit", "Hits a usage limit", "hit a usage limit", true),
  leaves_processes: E("leaves_processes", "Leaves processes running", "left processes running", false),
  runway_below: E("runway_below", "Runs low", "is running low on room", false, { name: "percent", kind: "percent", min: 5, max: 95, required: true }),
  runway_resets: E("runway_resets", "Has room again", "has room again", false),
  restarted: E("restarted", "Restarts after a crash", "was restarted after a crash", false),
  pool_message: E("pool_message", "Gets a message", "got a message", false, { name: "from", kind: "agent", min: null, max: null, required: false }),
  task_changes: E("task_changes", "A task moves", "had a task moved", false, { name: "to", kind: "task_state", min: null, max: null, required: true }),
  starts: E("starts", "Starts a session", "started a new session", false),
};

const SESSION = [C.finishes, C.ends, C.asks_you, C.fails, C.stuck, C.idle_for, C.hits_limit, C.leaves_processes];

export const WATCHES_CATALOGUE: Record<string, CatalogueEntry[]> = {
  session: SESSION,
  app: [C.finishes, C.ends, C.asks_you, C.fails, C.stuck, C.idle_for, C.hits_limit, C.leaves_processes, C.restarted, C.starts],
  project: [C.finishes, C.ends, C.asks_you, C.fails, C.stuck, C.idle_for, C.leaves_processes, C.task_changes, C.starts],
  pool: [C.finishes, C.asks_you, C.fails, C.pool_message, C.task_changes],
  account: [C.hits_limit, C.runway_below, C.runway_resets],
  any: [C.finishes, C.ends, C.asks_you, C.fails, C.stuck, C.idle_for, C.hits_limit, C.runway_below, C.runway_resets, C.leaves_processes, C.restarted, C.starts],
  // Agent app two can't tell when its sessions look stuck or sit idle:
  // those cards are hidden for it (D-127).
  "session:s-draft": [C.finishes, C.ends, C.asks_you, C.fails, C.hits_limit, C.leaves_processes],
  "app:cli:acct-home": [C.finishes, C.ends, C.asks_you, C.fails, C.hits_limit, C.leaves_processes, C.restarted, C.starts],
};

// ---------------------------------------------------------------------------
// Watches
// ---------------------------------------------------------------------------

function fixLifetime(draft: WatchDraftInfo, clock: EchoClock): Lifetime {
  const life = draft.lifetime;
  switch (life.kind) {
    case "today":
      return { kind: "until", atMs: nextMidnight(clock) };
    case "unsaid": {
      const sessions = draft.targets.length > 0 && draft.targets.every((t) => t.kind === "session");
      const ending = draft.conditions.length > 0 && draft.conditions.every((c) => c.code === "finishes" || c.code === "ends");
      return sessions && ending ? { kind: "once" } : { kind: "until", atMs: nextMidnight(clock) };
    }
    case "until":
      return { kind: "until", atMs: life.atMs };
    default:
      return { kind: life.kind };
  }
}

function watchOf(draft: WatchDraftInfo, id: string, createdMs: number): Watch {
  return { id, targets: draft.targets, matching: draft.matching, conditions: draft.conditions, lifetime: fixLifetime(draft, clockNow()), paused: false, evenInQuietHours: draft.evenInQuietHours, createdMs };
}

/** A draft's words, as the backend gives them (the stand-in echo). */
export function draftWords(draft: WatchDraftInfo, more?: Record<string, string>): DraftWords {
  const watch = watchOf(draft, "w-0", Date.now());
  const echo = echoParts(watch, namesWith(more), clockNow());
  const bad = watch.lifetime.kind === "until" && watch.lifetime.atMs <= Date.now();
  return { echo, words: joined(echo), problem: bad ? "until_past" : null };
}

interface Extra {
  state?: WatchInfo["state"];
  told?: number;
  lastAgo?: number | null;
  lastCode?: string | null;
  ended?: WatchInfo["ended"];
  endedAgo?: number;
  createdAgo?: number;
  changedAgo?: number;
  quiet?: boolean;
  lifetime?: Lifetime;
}

const TOLD_WORDS: Record<string, string> = Object.fromEntries(Object.values(C).map((e) => [e.code, e.told]));
const ENDED_WORDS: Record<string, string> = {
  first_fire: "Ended: it told you once, as asked.",
  targets_ended: "Ended: what it watched has ended.",
  time_reached: "Ended: its time was up.",
  target_gone: "Ended: what it watched was removed.",
  no_longer_offered: "Ended: AI Juggler can no longer watch for this.",
};

/** A watch as the list reads it. */
export function made(draft: WatchDraftInfo, id: string, x: Extra = {}, more?: Record<string, string>): WatchInfo {
  const created = WATCHES_NOW - (x.createdAgo ?? 2 * HOUR);
  const watch: Watch = { ...watchOf(draft, id, created), ...(x.lifetime ? { lifetime: x.lifetime } : {}), paused: x.state === "paused", evenInQuietHours: x.quiet ?? draft.evenInQuietHours };
  const echo = echoParts(watch, namesWith(more), clockNow());
  const lastCode = x.lastCode ?? (x.told ? draft.conditions[0]!.code : null);
  return {
    watch,
    words: joined(echo),
    echo,
    state: x.state ?? "active",
    ended: x.ended ?? null,
    endedWords: x.ended ? ENDED_WORDS[x.ended]! : null,
    endedMs: x.ended ? WATCHES_NOW - (x.endedAgo ?? HOUR) : null,
    told: x.told ?? 0,
    lastToldMs: x.told ? WATCHES_NOW - (x.lastAgo ?? HOUR) : null,
    lastToldCode: x.told ? lastCode : null,
    lastToldWords: x.told && lastCode ? (TOLD_WORDS[lastCode] ?? null) : null,
    version: 1,
    changedMs: WATCHES_NOW - (x.changedAgo ?? x.createdAgo ?? 2 * HOUR),
  };
}

const S = (id: string): WatchTarget => ({ kind: "session", id });
const d = (targets: WatchTarget[], conditions: WatchDraftInfo["conditions"], lifetime: WatchDraftInfo["lifetime"], extra: Partial<WatchDraftInfo> = {}): WatchDraftInfo => ({ targets, matching: "each", conditions, lifetime, evenInQuietHours: false, ...extra });
const stuck15 = { code: "stuck", param: { kind: "minutes" as const, value: 15 } };

/** The list frame: five watching, one paused (the mock's six). */
export function listWatches(): WatchInfo[] {
  return [
    made(d([S("s-api")], [{ code: "finishes" }, stuck15], { kind: "until_ends" }), "w-a1", { told: 1, lastAgo: 28 * MIN, lastCode: "finishes" }),
    made(d([S("s-api"), S("s-web")], [{ code: "finishes" }], { kind: "once" }, { matching: "all" }), "w-a2", {}),
    made(d([{ kind: "app", id: "cli:acct-home" }], [{ code: "asks_you", param: { kind: "minutes", value: 10 } }], { kind: "always" }, { evenInQuietHours: true }), "w-a3", { told: 3, lastAgo: 16 * HOUR - 19 * MIN, createdAgo: 3 * DAY }),
    made(d([{ kind: "account", id: "acct-work" }], [{ code: "runway_below", param: { kind: "percent", value: 20 } }], { kind: "today" }), "w-a4", {}),
    made(d([{ kind: "any" }], [{ code: "hits_limit" }], { kind: "always" }), "w-a5", { told: 2, lastAgo: 2 * DAY + 4 * HOUR, createdAgo: 5 * DAY }),
    made(d([{ kind: "pool", id: "pool-rel" }], [{ code: "pool_message", param: { kind: "agent", value: "agent-reviewer" } }], { kind: "always" }), "w-a6", { state: "paused", told: 4, lastAgo: DAY, createdAgo: 4 * DAY, changedAgo: 4 * HOUR + 48 * MIN }),
  ];
}

/** The ended frame's five: one for each reason. */
export function endedWatches(): WatchInfo[] {
  return [
    made(d([S("s-api")], [{ code: "finishes" }, stuck15], { kind: "until_ends" }), "w-e1", { state: "ended", ended: "targets_ended", endedAgo: 20 * MIN, told: 2 }),
    made(d([S("s-login")], [{ code: "finishes" }], { kind: "once" }), "w-e2", { state: "ended", ended: "first_fire", endedAgo: 2 * HOUR, told: 1 }),
    made(d([{ kind: "account", id: "acct-work" }], [{ code: "runway_below", param: { kind: "percent", value: 20 } }], { kind: "today" }), "w-e3", { state: "ended", ended: "time_reached", endedAgo: 15 * HOUR, lifetime: { kind: "until", atMs: new Date(2026, 10, 19, 0, 0).getTime() } }),
    made(d([{ kind: "project", id: "proj-gone" }], [{ code: "task_changes", param: { kind: "task_state", value: "done" } }], { kind: "always" }), "w-e4", { state: "ended", ended: "target_gone", endedAgo: DAY + 3 * HOUR, told: 6 }),
    made(d([{ kind: "app", id: "cli:acct-home" }], [{ code: "leaves_processes" }], { kind: "always" }), "w-e5", { state: "ended", ended: "no_longer_offered", endedAgo: 2 * DAY + 5 * HOUR, told: 1 }),
  ];
}

/** The many frame: 47 watching and 3 paused, at the limit of 50. */
export function manyWatches(): WatchInfo[] {
  const base = listWatches();
  const out: WatchInfo[] = [...base];
  const targets: WatchTarget[] = [S("s-web"), { kind: "app", id: "cli:acct-work" }, { kind: "project", id: "proj-shop" }, S("s-draft"), S("s-login"), { kind: "app", id: "cli:acct-studio" }];
  const conds: WatchDraftInfo["conditions"][] = [[{ code: "fails" }], [{ code: "leaves_processes" }], [{ code: "task_changes", param: { kind: "task_state", value: "in_review" } }], [{ code: "idle_for", param: { kind: "minutes", value: 30 } }], [{ code: "asks_you" }], [{ code: "ends" }]];
  for (let i = 0; out.length < 50; i += 1) {
    const at = i % targets.length;
    const told = i % 4 === 0 ? 0 : (i % 5) + 1;
    out.push(made(d([targets[at]!], conds[at]!, i % 3 === 0 ? { kind: "today" } : { kind: "always" }), `w-m${i.toString(16).padStart(2, "0")}`, { told, lastAgo: (i + 1) * 47 * MIN, createdAgo: (i + 3) * HOUR, state: i === 7 || i === 19 ? "paused" : "active" }));
  }
  return out;
}

export type WatchesPreviewState = { name: string; watches: WatchInfo[]; quiet: boolean };

export const STATES: Record<string, WatchesPreviewState> = {
  list: { name: "list", watches: [...listWatches(), ...endedWatches().slice(0, 3)], quiet: true },
  create: { name: "create", watches: listWatches(), quiet: true },
  edit: { name: "edit", watches: [made(d([S("s-api")], [{ code: "finishes" }, stuck15], { kind: "until_ends" }), "w-a1", { told: 1, lastAgo: 28 * MIN, createdAgo: 6 * HOUR }), ...listWatches().slice(1)], quiet: true },
  empty: { name: "empty", watches: [], quiet: true },
  ended: { name: "ended", watches: [...listWatches().slice(2, 5), made(d([S("s-draft")], [{ code: "finishes" }], { kind: "once" }), "w-x1", {}), ...endedWatches()], quiet: true },
  many: { name: "many", watches: [...manyWatches(), ...Array.from({ length: 31 }, (_, i) => made(d([S("s-login")], [{ code: "finishes" }], { kind: "once" }), `w-z${i.toString(16).padStart(2, "0")}`, { state: "ended", ended: "first_fire", endedAgo: (i + 1) * DAY / 2, told: 1 }))], quiet: true },
};

/** The stand-in's seam for a state. */
export function seamFor(state: WatchesPreviewState): WatchViewSeam {
  return { watches: state.watches.map((w) => structuredClone(w)), catalogue: WATCHES_CATALOGUE, draft: draftWords, made: (draft, id) => made(draft, id, { createdAgo: 0 }), calls: [] };
}

/** Bot has a route (Ask Bot shows), for `__AIJ_AI_JOBS__`. */
export const WATCHES_JOBS = [{ job: "chat", state: "ready", beta: false, beta_on: false, model_id: "built-in", model_name: "Built-in model", why: "recommended", pick: null }];

/** Generic tools for the captures (no vendor's name or mark), with the
 * preview's own (`__AIJ_TOOLS__`). */
export function watchesTools(): ToolSpecInfo[] {
  const base = PREVIEW_TOOLS[0]!;
  const generic = (id: string, name: string, desktop: boolean): ToolSpecInfo => ({ ...base, id, display_name: name, short_name: name, vendor: name, mark: null, vendor_mark: null, desktop_app: desktop ? name : null, host_app: null, kinds: desktop ? ["desktop"] : ["cli"] });
  return [...PREVIEW_TOOLS, generic("agent-cli", "Agent", false), generic("agent-one", "Agent app one", true), generic("agent-two", "Agent app two", true)];
}
