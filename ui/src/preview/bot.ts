/**
 * D-136 — Bot's preview data: each state of the approved mock-up
 * (`~/Dev/AIJuggler-design-refs/chat-panel/`, `?s=<state>`) as the
 * window's seams (`__AIJ_BOT__`, `__AIJ_AI_JOBS__`, `__AIJ_WATCHES__`).
 * Generic names only (the sessions "api" and "web", the Apps "Agent app
 * one", "Agent app two" and "Agent CLI", the project "shop"); the Apps'
 * providers are unknown ids, so they draw as letter tiles, never a
 * vendor's mark. Used by the component tests and the gallery.
 */
import type { AiJobInfo } from "../lib/ipc/ai";
import type { BotSeam } from "../lib/ipc/bot";
import type { WatchesSeam } from "../lib/ipc/watches";
import type { BotMemoryInfo } from "../types/generated/BotMemoryInfo";
import type { BotRowInfo } from "../types/generated/BotRowInfo";
import type { BotTurnInfo } from "../types/generated/BotTurnInfo";
import { FORGET_AND_BACKUPS } from "../lib/bot-backup-words";
import { STATES as MODEL_STATES, jobs as modelJobs } from "./ai-models";

const MIN = 60_000;
const DAY = 86_400_000;
/** 19 Nov 2026, 11:30 local: "now" in every state. */
export const BOT_NOW = new Date(2026, 10, 19, 11, 30).getTime();
const at = (h: number, m: number, daysAgo = 0) => new Date(2026, 10, 19 - daysAgo, h, m).getTime();

function row(over: Partial<BotRowInfo> & Pick<BotRowInfo, "kind" | "id" | "title">): BotRowInfo {
  return {
    name_hidden: false, state: null, at_ms: null, started_ms: null, provider: null, app_id: null, app_label: null, desktop: false, project: null, project_id: null,
    pool: null, pool_id: null, holder: null, files_changed: null, branch: null, used_percent: null, window: null, resets_at_ms: null, blocked_by: null, quote: null,
    then_state: null, gone: false, ...over,
  };
}

const session = (id: string, title: string, app: string, project: string, over: Partial<BotRowInfo> = {}) =>
  row({ kind: "session", id, title, provider: app.toLowerCase().replaceAll(" ", "-"), desktop: true, app_label: app, project, project_id: `p-${project}`, ...over });

export const R = {
  web: session("s-web", "web", "Agent app one", "shop", { state: "asks_you", at_ms: BOT_NOW - 12 * MIN, quote: "Allow edits to 3 files in auth/?" }),
  login: session("s-login", "Fix the login redirect", "Agent app one", "shop", { state: "stuck", at_ms: BOT_NOW - 18 * MIN }),
  task: row({ kind: "task", id: "t-export", title: "Wire the export button", state: "blocked", at_ms: BOT_NOW - 31 * MIN, pool: "Release pool", pool_id: "pool-release", holder: "builder-2", blocked_by: 1, project: "shop", project_id: "p-shop" }),
  apiDone: session("s-api", "api", "Agent CLI", "shop", { state: "finished", at_ms: at(14, 32, 1), files_changed: 3 }),
  apiRun: session("s-api", "api", "Agent CLI", "shop", { state: "working", at_ms: BOT_NOW - 34 * MIN, branch: "feat/limits" }),
  webRun: session("s-web", "web", "Agent app one", "shop", { state: "working", at_ms: BOT_NOW - 8 * MIN }),
  loginDone: session("s-login", "Fix the login redirect", "Agent app one", "shop", { state: "finished", at_ms: at(16, 20, 1), files_changed: 3 }),
  taskDone: row({ kind: "task", id: "t-export", title: "Wire the export button", state: "done", at_ms: at(17, 5, 1), pool: "Release pool", pool_id: "pool-release", holder: "builder-2", project: "shop", project_id: "p-shop" }),
  docs: session("s-docs", "Draft the settings page", "Agent app two", "docs-site", { state: "finished", at_ms: at(11, 5) }),
  work: row({ kind: "account", id: "a-work", title: "Work", used_percent: 54, window: "weekly", resets_at_ms: at(9, 0, -4) }),
  personal: row({ kind: "account", id: "a-personal", title: "Personal", used_percent: 36, window: "five_hour", resets_at_ms: at(20, 13) }),
};

function turn(id: string, question: string, when: number, over: Partial<BotTurnInfo> = {}): BotTurnInfo {
  return {
    id, at_ms: when, question, lang: "en", outcome: "rows", scope: null, rows: [], total: over.rows?.length ?? 0, unknown: null, shortcut: null,
    looked: { reach: "everything", scope: null, memories: 0 }, written: null, memory: null, used: [], live: true, redacted: false, flagged: false,
    ...over,
  };
}

const BLOCKED = turn("t1", "What is blocked?", at(10, 44), { rows: [R.web, R.login, R.task] });
/** Asked a minute ago: "Looked at · just now", as the mock frames it. */
const BLOCKED_NOW = { ...BLOCKED, at_ms: BOT_NOW - 40_000 };
const SHOP_YESTERDAY = turn("t2", "What changed in shop yesterday?", at(10, 52), { rows: [R.loginDone, R.taskDone], looked: { reach: "subject", scope: { kind: "project", id: "p-shop", name: "shop", sessions: [] }, memories: 0 } });

const MEMORY: BotMemoryInfo = { key: "alias:session:s-web", slot: "alias", text: "the api agent", about: "web", about_kind: "session", at_ms: at(11, 2), used: 4, versions: 2 };
const MEMORIES: BotMemoryInfo[] = [
  { ...MEMORY, at_ms: BOT_NOW - 12 * DAY },
  { key: "alias:pool:pool-release", slot: "alias", text: "the release", about: "Release pool", about_kind: "pool", at_ms: BOT_NOW - 3 * DAY, used: 1, versions: 0 },
  { key: "focus:global", slot: "focus", text: "shop", about: "shop", about_kind: "project", at_ms: BOT_NOW - 60 * DAY, used: 0, versions: 0 },
  { key: "note:a1", slot: "note", text: "I review pull requests after lunch", about: null, about_kind: null, at_ms: BOT_NOW - 3 * DAY, used: 0, versions: 0 },
  { key: "note:a2", slot: "note", text: "Leave docs-site alone this week", about: null, about_kind: null, at_ms: BOT_NOW - 6 * DAY, used: 0, versions: 0 },
];

/** Bot's answer: Beta off where Apple's model passes (the default). */
const BETA_OFF = modelJobs({ chat: { state: "beta_off", model_id: "apple-on-device", model_name: "Apple’s built-in AI" } });
const BETA_ON = modelJobs({ chat: { state: "ready", beta: true, beta_on: true, model_id: "apple-on-device", model_name: "Apple’s built-in AI", why: "recommended" } });
const NO_ENGINE = modelJobs({ chat: { state: "no_engine", why: "not_this_computer" } });

export interface BotPreviewState {
  /** The mock's state name. */
  name: string;
  bot: BotSeam;
  jobs: AiJobInfo[];
  watches?: WatchesSeam;
  /** The models catalog (the no-model offer). */
  catalog?: unknown;
  /** Where Bot is: the panel, the page, or closed (the door's page). */
  open: "panel" | "page" | "closed";
  /** The window's width. */
  width: 1280 | 1024;
  /** The Bot page's tab. */
  tab?: "memory";
}

const memories = { items: MEMORIES, forget_and_backups: FORGET_AND_BACKUPS, watches: 5, questions: 38 };
const COUNT = { questions: 312, since_ms: at(9, 0, 17), memories: 4, watches: 5 };
const WATCHES: WatchesSeam = {
  names: { "s-web": "web", "s-api": "api" },
  watches: [
    { id: "w1", words: "Tell you when “web” finishes. Once." },
    { id: "w2", words: "Tell you when “api” finishes, or looks stuck for 15 minutes." },
  ],
};

const newest = (...turns: BotTurnInfo[]) => [...turns].reverse();

export const STATES: Record<string, BotPreviewState> = {
  icon: { name: "icon", bot: { turns: [] }, jobs: BETA_OFF, open: "closed", width: 1280 },
  narrow: { name: "narrow", bot: { turns: newest(BLOCKED_NOW), count: COUNT }, jobs: BETA_OFF, open: "panel", width: 1024 },
  empty: { name: "empty", bot: { turns: [] }, jobs: BETA_OFF, open: "panel", width: 1280 },
  lookup: { name: "lookup", bot: { turns: newest(BLOCKED_NOW), count: COUNT }, jobs: BETA_OFF, open: "panel", width: 1280 },
  scope: {
    name: "scope",
    bot: {
      scope: { kind: "session", id: "s-api", name: "api", sessions: [] },
      turns: newest(turn("t3", "What did it change today?", at(11, 22), { scope: { kind: "session", id: "s-api", name: "api", sessions: [] }, rows: [R.apiRun], looked: { reach: "scope", scope: { kind: "session", id: "s-api", name: "api", sessions: [] }, memories: 0 } })),
      count: COUNT,
    },
    jobs: BETA_OFF,
    open: "panel",
    width: 1280,
  },
  doors: { name: "doors", bot: { turns: newest(BLOCKED_NOW), count: COUNT }, jobs: BETA_OFF, open: "closed", width: 1280 },
  dontknow: {
    name: "dontknow",
    bot: {
      turns: newest(
        turn("t4", "How long did the tests take in api?", at(10, 47), { outcome: "dont_know", unknown: "test_durations", shortcut: "finished_today", rows: [R.apiDone], looked: { reach: "subject", scope: { kind: "session", id: "s-api", name: "api", sessions: [] }, memories: 0 } }),
        turn("t5", "Did the deploy to staging work?", at(10, 49), { outcome: "dont_know", unknown: "nothing_found", shortcut: "finished_today", looked: { reach: "searched", scope: null, memories: 0 } }),
      ),
      count: COUNT,
    },
    jobs: BETA_OFF,
    open: "panel",
    width: 1280,
  },
  beta: {
    name: "beta",
    bot: {
      turns: newest(
        turn("t6", "What changed in shop yesterday?", at(10, 52), {
          outcome: "answered",
          rows: [R.loginDone, R.taskDone],
          looked: { reach: "subject", scope: { kind: "project", id: "p-shop", name: "shop", sessions: [] }, memories: 0 },
          written: { parts: [{ text: "“Fix the login redirect” finished yesterday with 3 files changed.", cites: [1] }, { text: "In Release pool, “Wire the export button” is done.", cites: [2] }], model_name: "Apple’s built-in AI", on_this_mac: true },
        }),
      ),
      count: COUNT,
    },
    jobs: BETA_ON,
    open: "panel",
    width: 1280,
  },
  nomodel: { name: "nomodel", bot: { turns: [] }, jobs: NO_ENGINE, catalog: MODEL_STATES.noapple?.catalog, open: "panel", width: 1280 },
  remember: {
    name: "remember",
    bot: {
      turns: newest(
        turn("t7", "Remember: call web the api agent", at(11, 2), { outcome: "remembered", memory: MEMORY }),
        turn("t8", "Is the api agent done?", at(11, 3), { rows: [R.webRun], used: [MEMORY], looked: { reach: "subject", scope: { kind: "session", id: "s-web", name: "web", sessions: [] }, memories: 1 } }),
      ),
      count: COUNT,
      memories,
    },
    jobs: BETA_OFF,
    open: "panel",
    width: 1280,
  },
  watch: { name: "watch", bot: { turns: newest(turn("t9", "What’s running in shop?", at(11, 10), { rows: [R.apiRun, R.webRun], looked: { reach: "subject", scope: { kind: "project", id: "p-shop", name: "shop", sessions: [] }, memories: 0 } })), count: COUNT }, jobs: BETA_OFF, watches: WATCHES, open: "panel", width: 1280 },
  newtopic: {
    name: "newtopic",
    bot: { turns: newest(BLOCKED, turn("t10", "How much of Work did I use this week?", at(11, 21), { rows: [R.work], looked: { reach: "accounts", scope: null, memories: 0 } })), topics: [{ id: "topic-2", started_ms: at(11, 20), first: false }], count: COUNT },
    jobs: BETA_OFF,
    open: "panel",
    width: 1280,
  },
  search: {
    name: "search",
    bot: {
      turns: newest(
        turn("t11", "Is Fix the login redirect stuck?", at(16, 2, 1), { rows: [{ ...R.loginDone, at_ms: at(18, 40, 1), then_state: "stuck" }] }),
        turn("t12", "What did builder-3 do?", at(11, 20, 1), { rows: [row({ kind: "task", id: "t-gone", title: "builder-3", gone: true, pool: "Release pool" })] }),
        { ...SHOP_YESTERDAY, rows: [R.loginDone], total: 1 },
      ),
      count: COUNT,
    },
    jobs: BETA_OFF,
    open: "panel",
    width: 1280,
  },
  long: {
    name: "long",
    bot: {
      turns: newest(
        ...Array.from({ length: 20 }, (_, i) => turn(`old-${i}`, "What needs me?", at(9, 2, 17 - i), { rows: [R.web] })),
        turn("t13", "Is Fix the login redirect stuck?", at(16, 2, 1), { rows: [{ ...R.loginDone, at_ms: at(18, 40, 1), then_state: "stuck" }] }),
        turn("t14", "What did builder-3 do?", at(17, 20, 1), { rows: [row({ kind: "task", id: "t-gone", title: "builder-3", gone: true, pool: "Release pool" })] }),
        turn("t15", "Welche Sitzungen sind heute fertig?", at(8, 30), { lang: "de", rows: [{ ...R.apiDone, at_ms: at(8, 20) }, R.docs] }),
        turn("t16", "Which account has the most room?", at(11, 14), { rows: [R.personal, R.work], looked: { reach: "accounts", scope: null, memories: 0 } }),
      ),
      topics: [{ id: "topic-3", started_ms: at(11, 12), first: false }],
      count: COUNT,
    },
    jobs: BETA_OFF,
    open: "panel",
    width: 1280,
  },
  settings: { name: "settings", bot: { turns: newest(BLOCKED_NOW), count: COUNT }, jobs: BETA_OFF, open: "panel", width: 1280 },
  clear: { name: "clear", bot: { turns: newest(BLOCKED_NOW), count: COUNT }, jobs: BETA_OFF, open: "panel", width: 1280 },
  menu: { name: "menu", bot: { turns: newest(BLOCKED_NOW), count: COUNT }, jobs: BETA_OFF, open: "panel", width: 1280 },
  full: { name: "full", bot: { turns: newest(BLOCKED, SHOP_YESTERDAY), count: COUNT, memories }, jobs: BETA_OFF, watches: WATCHES, open: "page", width: 1280 },
  memories: { name: "memories", bot: { turns: newest(BLOCKED), count: COUNT, memories }, jobs: BETA_OFF, open: "page", tab: "memory", width: 1280 },
  forget: { name: "forget", bot: { turns: newest(BLOCKED), count: COUNT, memories }, jobs: BETA_OFF, open: "page", tab: "memory", width: 1280 },
};
