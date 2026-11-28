/**
 * D-118 (AJ-E3701/E3702) — Home's feed in the preview: items seeded as
 * `window.__AIJ_FEED__` (the preview reads them as the backend's pages)
 * and the "last look" seeded as `__AIJ_SUMMARY__`. Shared by the Home
 * feed gallery, the paging spec and the capture families.
 */
import { feedItem } from "../../../src/preview/feed";
import type { FeedItemInfo } from "../../../src/types/generated/FeedItemInfo";
import type { ConversationInfo } from "../../../src/types/generated/ConversationInfo";
import type { KnownPlaceInfo } from "../../../src/types/generated/KnownPlaceInfo";
import type { MessageNeedInfo } from "../../../src/types/generated/MessageNeedInfo";
import type { FixtureSet } from "../../../src/preview/fixtures";
import type { WatchInfo } from "../../../src/types/generated/WatchInfo";


type Over = Omit<Parameters<typeof feedItem>[0], "id" | "at_ms" | "kind" | "category">;

const HOUR = 3_600_000;
const MIN = 60_000;
const WORK = { kind: "app", name: null, provider: "claude-code", folder: "claude-work", folder_label: "Work Max", desktop: false } as const;
const PERSONAL = { kind: "app", name: null, provider: "claude-code", folder: "claude-personal", folder_label: "Personal", desktop: true } as const;
const CODEX = { kind: "app", name: null, provider: "codex", folder: "codex", folder_label: "Codex", desktop: false } as const;

/** The last look, `hours` ago (never the default window), as the seam
 * reads it; the return (the read as Home opened, `generated_ms`) `back`
 * hours ago: what came after it is above the away divider, what came
 * between the two is the digest's only (R5). */
export function away(now: number, hours: number, back = 0) {
  return { since_ms: now - hours * HOUR, generated_ms: now - back * HOUR, default_window: false, rows: [], counts: {}, leftovers: 0 };
}

/** Busy's away: gone from 9 h ago until 4½ h ago, so every card of the
 * busy day is since the return and the night's two are the digest's. */
export const BUSY_AWAY = (now: number) => away(now, 9, 4.5);

/** A few endings newer than the last look (9 h) and one older: the digest's input. */
export function digestFeed(now: number): FeedItemInfo[] {
  const ended = (id: string, ago: number, kind: string, subject: string, level: number) =>
    feedItem({
      id, at_ms: now - ago, kind, category: kind === "failed" ? "needs" : "finished", level, coalesce: id, subject, author: WORK,
      facts: { project: "ai-juggler", cwd: "/Users/you/Dev/ai-juggler", duration_ms: 25 * MIN },
      target: { session_id: id.slice(2) },
    });
  return [
    ended("l:s1", 1 * HOUR, "finished", "Refactor the router", 0),
    ended("l:s2", 2 * HOUR, "failed", "Migrate the schema", 3),
    ended("l:s3", 3 * HOUR, "finished", "Write the docs", 0),
    ended("l:s4", 20 * HOUR, "finished", "An older session", 0),
  ];
}

/** AJ-E3713 R6a: a turn's diff stat, as the backend reads it from the
 * turn's edit and write calls (`null` lines: a tool whose calls don't
 * count them, files only). */
function changed(turn: string, files: number, added: number | null, removed: number | null): FeedItemInfo["facts"]["changes"] {
  const names = ["src/auth/session.ts", "src/auth/login.ts", "src/auth/refresh.ts", "src/api/client.ts", "src/api/routes.ts", "src/ui/LoginForm.tsx", "src/ui/Header.tsx", "tests/login.test.ts", "tests/refresh.test.ts", "tests/routes.test.ts", "docs/auth.md", "src/auth/cookies.ts", "src/api/errors.ts", "src/ui/Banner.tsx"];
  return { turn, files, names: names.slice(0, files), added, removed };
}

/** Every kind the feed draws, newest first, a rollup of four finished
 * sessions, and two from the night before (the away digest's). */
export function busyFeed(now: number): FeedItemInfo[] {
  const item = (id: string, ago: number, kind: string, category: string, over: Over) =>
    feedItem({ id, at_ms: now - ago, kind, category, coalesce: id, subject: "Refactor the router", author: WORK, ...over });
  const rolled = [0, 1, 2, 3].map((n) =>
    feedItem({
      id: `l:r${n}`, at_ms: now - (3 * HOUR + n * 20 * MIN), kind: "finished", category: "finished", coalesce: `s:r${n}`, rollup: "v:finished:claude-work:proj-ai-juggler:L0",
      rollup_window_ms: 2 * HOUR, scope: "f:claude-work", subject: `Fix flaky test ${n + 1}`, author: WORK,
      facts: { project: "ai-juggler", project_id: "proj-ai-juggler", duration_ms: 12 * MIN, changes: changed(`r${n}`, n + 1, 12 + n * 7, n * 3) }, target: { session_id: `r${n}` },
    }),
  );
  return [
    item("l:1", 2 * MIN, "finished", "finished", { facts: { project: "ai-juggler", duration_ms: 18 * MIN, changes: changed("l1", 3, 58, 12) } }),
    item("l:2", 9 * MIN, "failed", "needs", { level: 3, subject: "Migrate the schema", facts: { project: "payments-api", detail: "exit code 1", duration_ms: 4 * MIN } }),
    // A crash and the restart after it: one session, one card.
    item("n:4", 13 * MIN, "restarted", "needs", { level: 3, coalesce: "s:idx", subject: "Index the docs", author: { ...PERSONAL, kind: "juggler" }, facts: { attempt: 1, tries: 3 } }),
    item("l:3", 14 * MIN, "crashed", "needs", { level: 3, coalesce: "s:idx", subject: "Index the docs", author: PERSONAL, facts: { detail: "signal 9" } }),
    item("l:5", 30 * MIN, "answered", "needs", { subject: "Write the docs", facts: { need: "asks" } }),
    item("l:6", 40 * MIN, "switch", "switches", { level: 1, subject: "Answer the release question", author: { ...WORK, kind: "juggler" }, facts: { from_label: "Work Max", from_provider: "claude-code", to_label: "Codex", to_provider: "codex", mechanism: "copy", fidelity: "full" } }),
    item("l:7", 50 * MIN, "switch_failed", "switches", { level: 2, subject: "Plan the trip notes", author: { ...WORK, kind: "juggler" }, facts: { from_label: "Work Max", to_label: "Personal", detail: "Personal is signed out" } }),
    item("l:8", 1 * HOUR, "quota", "switches", { level: 2, subject: "Work Max", author: { ...WORK, kind: "account", name: "Work Max" }, facts: { window: "5-hour", meter: 8, resets_at_ms: now + 2 * HOUR, sessions: 2 } }),
    item("l:9", 70 * MIN, "reset", "switches", { subject: "Codex", author: { ...CODEX, kind: "account", name: "Codex" }, facts: { window: "weekly", meter: 100 } }),
    item("l:10", 80 * MIN, "task_done", "pools", { subject: "Add the paging test", author: { kind: "agent", name: "Tess", provider: "claude-code", folder: null, folder_label: null, desktop: false }, facts: { pool: "Release crew", pool_id: "pool-1", change: "In progress → Done" } }),
    item("l:11", 90 * MIN, "task_review", "pools", { level: 1, subject: "Review the diff", author: { kind: "agent", name: "Rae", provider: "codex", folder: null, folder_label: null, desktop: false }, facts: { pool: "Release crew", pool_id: "pool-1", change: "In progress → In review" } }),
    item("l:12", 100 * MIN, "message", "pools", { subject: "Which branch?", author: { kind: "agent", name: "Tess", provider: "claude-code", folder: null, folder_label: null, desktop: false }, facts: { pool: "Release crew", pool_id: "pool-1", thread: "Which branch?", thread_count: 3, quote: "I'd cut it from release/0.5 so the paging test lands with the feed." } }),
    item("c:13", 110 * MIN, "worked", "other", { subject: "Plan the trip notes app", author: CODEX, facts: { project: "trip-notes", changes: changed("c13", 2, null, null) } }),
    ...rolled,
    // The night (BUSY_AWAY's digest): a failure and a finish while away.
    item("l:14", 6 * HOUR, "failed", "needs", { level: 3, subject: "Rebuild the search index", facts: { project: "ai-juggler", detail: "exit code 2", duration_ms: 40 * MIN } }),
    item("l:15", 7 * HOUR, "finished", "finished", { subject: "Bump the dependencies", facts: { project: "ai-juggler", duration_ms: 22 * MIN } }),
  ];
}

/** `count` single cards (no rollup), newest first: the paging test's flood. */
export function manyFeed(now: number, count: number): FeedItemInfo[] {
  return Array.from({ length: count }, (_, n) =>
    feedItem({
      id: `l:p${n}`, at_ms: now - n * 7 * MIN, kind: "finished", category: "finished", coalesce: `s:p${n}`, subject: `Session ${n + 1}`, author: WORK,
      facts: { project: "ai-juggler", duration_ms: 10 * MIN }, target: { session_id: `p${n}` },
    }),
  );
}

/**
 * The approved D-118 mock-up's data (design refs: calm-ui/home-feed,
 * mock-home-feed.html), for like-for-like side-by-sides: a busy morning,
 * 10:13 AM UTC (read the words in UTC and en-US: the spec's `test.use`).
 * Two needs, five apps, seven open sessions, runway Work Max 12 % / Personal
 * 64 % / Team Plus not measured, and the mock's feed and away digest.
 * Not reproducible yet (no type or feature): a finished item's agent
 * excerpt and git line (AJ-E3708), PR items (AJ-E3706), a pool need's
 * quote line and the "allow edits to 3 files" ask (E3703), This Mac
 * pressure lines in the digest, and the mock's "1 to review" (no session
 * here has finished and waits on a look: the feed's Mark reviewed is
 * E3703's). "N finished today" is the notification model's count, seeded
 * as `notices` from the feed's finished items of today (11, the mock's
 * own number is 6).
 */
export const MOCK_T0 = Date.UTC(2024, 9, 13, 10, 13, 0);

const mockPlace = (app: string, shown: string, appId: string, profile: string, kind: "desktop" | "terminal", appPid: number, instance: string | null = null) => ({
  app, shown, app_id: appId, profile, instance, pid: appPid + 1, kind, app_pid: appPid, label: instance ? `${shown} · ${instance}` : shown,
});

export function mockHome(now: number) {
  const CLAUDE_APP = mockPlace("Claude", "Claude app", "claude-app", "Work Max", "desktop", 4100, "Work");
  const CLAUDE_CODE = mockPlace("Terminal", "Terminal", "terminal", "Work Max", "terminal", 5100);
  const CODEX_CLI = mockPlace("Terminal", "Terminal", "terminal", "Personal", "terminal", 5100);
  const CHATGPT = mockPlace("ChatGPT", "ChatGPT", "codex-app", "Personal", "desktop", 6100);
  let pid = 7000;
  const open = (id: string, place: ReturnType<typeof mockPlace>, account: [string, string, string], over: Partial<ConversationInfo>): ConversationInfo => {
    pid += 1;
    return {
      id, tool: account[0], account_id: account[1], account_label: account[2], project_id: null, cwd: "/Users/you/Dev/web-app", title: id, git_branch: "main",
      started_ms: now - 3_600_000, last_activity_ms: now - 60_000, tokens: null, live: true, session_id: null, places: [{ ...place, pid }], pid, state: "working",
      probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over,
    };
  };
  const ACC_WORK: [string, string, string] = ["claude-code", "claude-work", "Work Max"];
  const ACC_PERSONAL: [string, string, string] = ["codex", "codex-personal", "Personal"];
  const conversations = [
    open("m-1", CLAUDE_APP, ACC_WORK, { title: "Migrate the billing webhooks", project_id: "proj-payments", cwd: "/Users/you/Dev/payments-api", state: "asks_you", last_activity_ms: now - 12 * 60_000 }),
    open("m-2", CLAUDE_APP, ACC_WORK, { title: "Draft the onboarding copy", project_id: "proj-web", state: "working" }),
    open("m-3", CLAUDE_CODE, ACC_WORK, { title: "Port the settings parser", project_id: "proj-web", state: "working", last_activity_ms: now - 2 * 60_000 }),
    open("m-4", CLAUDE_CODE, ACC_WORK, { title: "feat/read-model", project_id: "proj-web", state: "working", last_activity_ms: now - 3 * 60_000 }),
    open("m-5", CLAUDE_CODE, ACC_WORK, { title: "Nightly tests", project_id: "proj-web", state: "idle", background: true, last_activity_ms: now - 50 * 60_000 }),
    open("m-6", CODEX_CLI, ACC_PERSONAL, { title: "Answer the release question", project_id: "proj-web", state: "working" }),
    open("m-7", CODEX_CLI, ACC_PERSONAL, { title: "Plan the trip notes app", project_id: null, cwd: "/Users/you/Dev/trip-notes", state: "idle", last_activity_ms: now - 40 * 60_000 }),
    open("m-8", CHATGPT, ACC_PERSONAL, { title: "Outline the launch post", project_id: null, cwd: null, state: "idle", last_activity_ms: now - 30 * 60_000 }),
  ];
  const account = (id: string, provider: string, label: string, plan: string | null) => ({ id, provider, label, plan, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
  const project = (id: string, name: string) => ({ id, root: `/Users/you/Dev/${name}`, name, last_activity_ms: now, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false });
  const sub = (key: string, name: string, ids: string[], tool: string, plan: string | null) => ({
    key, tool, state: "signed_in", email: `${key}@example.com`, organisation: null, reason: null, plan, renewal_date: null, fee_usd: null, differs: [], choices: [], name,
    profiles: ids.map((id) => ({ id, label: name, folder: `~/.${id}`, plan, renewal_date: null, fee_usd: null })),
  });
  const fixture = {
    accounts: [account("claude-work", "claude-code", "Work Max", "max-20x"), account("codex-personal", "codex", "Personal", null), account("claude-team", "claude-code", "Team Plus", null)],
    projects: [project("proj-payments", "payments-api"), project("proj-web", "web-app")],
    sessions: [],
    conversations,
    subscriptions: [sub("work", "Work Max", ["claude-work"], "claude-code", "max-20x"), sub("me", "Personal", ["codex-personal"], "codex", null), sub("team", "Team Plus", ["claude-team"], "claude-code", null)],
  } as unknown as FixtureSet;
  const window5 = (used: number, resets: number) => ({ state: "tracking", percentage: used, projected_pct: null, badge: "measured", stale_note_ms: null, resets_at_ms: resets, reset_basis: "measured", measured_at_ms: now - 5 * 60_000, burned_tokens: 0 });
  const quota = [
    { account: "claude-work", five_hour: window5(88, now + 5 * HOUR), weekly: null },
    { account: "codex-personal", five_hour: window5(36, now + 10 * HOUR), weekly: null },
  ];
  const place = (id: string, app: string, label: string, over: Partial<KnownPlaceInfo>): KnownPlaceInfo => ({
    id, app_id: app, app: app === "codex-app" ? "ChatGPT" : "Claude", instance: null, profile: null, label, app_pid: null, lost: false, keep_alive: false, paused: null, account_id: null,
    seen_ms: now - 3_600_000, cannot_start: null, recent: [], found: false, ...over,
  });
  const places = {
    places: [
      place("app:claude-app:/Users/you/Profiles/Work", "claude-app", "Claude · Work Max", { instance: "Work", profile: "Work Max", app_pid: CLAUDE_APP.app_pid, account_id: "claude-work" }),
      place("app:codex-app:", "codex-app", "ChatGPT", { profile: "Personal", app_pid: CHATGPT.app_pid, account_id: "codex-personal" }),
      place("app:claude-app:/Users/you/Profiles/Lab", "claude-app", "Claude · Lab", { instance: "Lab", profile: "Lab", seen_ms: now - 2 * 24 * HOUR }),
    ],
  };
  const messages = {
    threads: [], messages: {}, paused: [],
    needs: [{
      key: "question:builder-1", kind: "question", pool_id: "pool-release", pool_name: "Release pool", thread_id: "thr-export", message_id: "m-q1", session_id: null, ask_id: null, offer_always: false,
      title: "builder-1 asks you", detail: "Export flow", preview: "Use the v2 export API, or keep v1 for this release?", sender: "builder-1", thread: "Export flow", at_ms: now - 5 * MIN,
    } satisfies MessageNeedInfo],
  };

  const pool = (name: string) => ({ kind: "agent", name, provider: "claude-code", folder: null, folder_label: null, desktop: false }) as const;
  const item = (id: string, ago: number, kind: string, category: string, over: Over) =>
    feedItem({ id, at_ms: now - ago, kind, category, coalesce: id, subject: "", author: WORK, ...over });
  const trip = ["Sketch the notes data model", "Add the offline cache", "Fix the date picker on Safari", "Name the sync rules"].map((title, n) =>
    feedItem({
      id: `l:t${n}`, at_ms: now - (40 * MIN + n * 8 * MIN), kind: "finished", category: "finished", coalesce: `s:t${n}`, rollup: "v:finished:codex-personal:proj-trip:L0", rollup_window_ms: 2 * HOUR,
      scope: "f:codex-personal", subject: title, author: { ...CODEX, folder: "codex-personal", folder_label: "Personal" }, facts: { project: "trip-notes", duration_ms: [18, 31, 12, 9][n]! * MIN }, target: { session_id: `t${n}` },
    }),
  );
  const night = (id: string, hour: number, minute: number, subject: string) =>
    item(id, now - Date.UTC(2024, 9, 13, hour, minute), "finished", "finished", { subject, facts: { project: "web-app", duration_ms: 20 * MIN } });
  const feed: FeedItemInfo[] = [
    item("l:1", 0, "finished", "finished", { subject: "Fix the flaky login test", facts: { project: "web-app", duration_ms: 42 * MIN, quote: "Fixed the race in the login test by awaiting the session cookie before the redirect check. All 212 tests pass; no other files touched." }, target: { session_id: "m-3" } }),
    item("l:2", 14 * MIN, "quota", "switches", { level: 2, subject: "Work Max", author: { ...WORK, kind: "account", name: "Work Max" }, facts: { window: "5-hour", meter: 12, resets_at_ms: now + 5 * HOUR, sessions: 2 } }),
    item("l:3", 21 * MIN, "message", "pools", { subject: "builder-1", author: pool("reviewer"), facts: { pool: "Release pool", pool_id: "pool-release", thread: "Export flow", thread_count: 4, quote: "The migration misses its down step. Adding it before merge, then this is ready for review." } }),
    item("l:4", 34 * MIN, "task_done", "pools", { subject: "Wire the export button", author: pool("builder-2"), facts: { pool: "Release pool", pool_id: "pool-release", change: "In progress → Done", quote: "CSV and JSON both covered; the PDF export is a follow-up task." } }),
    item("l:5", 0, "switch", "switches", { level: 1, at_ms: Date.UTC(2024, 9, 13, 9, 58), subject: "Answer the release question", author: { ...WORK, kind: "juggler" }, facts: { from_label: "Work Max", from_provider: "claude-code", to_label: "Personal", to_provider: "codex", mechanism: "copy", fidelity: "full" } }),
    ...trip,
    // The night (the away digest): a stop and restart, the window reset, six finishes.
    item("n:6", 0, "restarted", "needs", { level: 3, at_ms: Date.UTC(2024, 9, 13, 2, 4), coalesce: "s:port", subject: "Port the settings parser", author: { ...WORK, kind: "juggler" }, facts: { attempt: 1, tries: 3, detail: "exit code 1" } }),
    item("l:6", 0, "crashed", "needs", { level: 3, at_ms: Date.UTC(2024, 9, 13, 2, 3), coalesce: "s:port", subject: "Port the settings parser", facts: { detail: "exit code 1" } }),
    item("l:7", 0, "reset", "switches", { at_ms: Date.UTC(2024, 9, 13, 3, 13), subject: "Personal", author: { ...CODEX, kind: "account", name: "Personal" }, facts: { window: "5-hour", meter: 100 } }),
    ...[[0, 20, "Bump the lint rules"], [0, 50, "Update the changelog"], [1, 10, "Prune the old fixtures"], [1, 40, "Rename the export module"], [3, 30, "Tidy the CI cache"], [4, 40, "Move the docs to the new site"]].map(([h, m, subject], n) => night(`l:n${n}`, h as number, m as number, subject as string)),
  ].sort((a, b) => b.at_ms - a.at_ms);
  // What the notification model holds of today (the backend's digest,
  // "N finished today" beside Home's title): one read finished notice per
  // finished item of today in the feed, as the backend writes them.
  const today = Date.UTC(2024, 9, 13);
  const target = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
  const facts = { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null };
  const notices = {
    earlier: feed
      .filter((one) => one.kind === "finished" && one.at_ms >= today)
      .map((one, n) => ({ id: 500 + n, kind: "finished", needs: false, title: `${one.subject} finished`, detail: "", project_id: null, project: null, target, facts, action: null, meter: null, created_ms: one.at_ms, unread: false, snoozed_until_ms: null, muted: false, dismissed: false })),
  };
  return { fixture, quota, places, messages, feed, notices, summary: away(now, 13, 2.5) };
}

/**
 * AJ-E3713 — the approved story-feed mock's data
 * (`AIJuggler-design-refs/story-feed/mock-story-feed.html`), at 3 PM UTC:
 * one ask pinned (Cart endpoints), then the posts it draws: a Claude app
 * post with its thread (Login refactor answered, then finished), AI
 * Juggler's switch with its before → after, Codex's finish quoting its
 * last reply, a pool's story (followed), and this morning's usage. The
 * first post's diff stat is its turn's edit and write calls (AJ-E3713
 * R6a: 14 files +412 −96, the mock's). Not drawn, for want of the facts
 * (no stand-in): the three-session summary, and "2 keys stayed hidden" (a
 * switch's redaction count isn't kept).
 */
export const STORY_T0 = Date.UTC(2024, 9, 13, 15, 0, 0);

export function storyHome(now: number) {
  const base = mockHome(now);
  const CLAUDE_APP = { kind: "app", name: null, provider: "claude-code", folder: "claude-work", folder_label: "work-max", desktop: true } as const;
  const CODEX_PERSONAL = { kind: "app", name: null, provider: "codex", folder: "codex-personal", folder_label: "Personal", desktop: false } as const;
  const conversations = (base.fixture as unknown as { conversations: ConversationInfo[] }).conversations.map((one) =>
    one.id === "m-1" ? { ...one, title: "Cart endpoints", last_activity_ms: now - 4 * MIN } : one.state === "asks_you" ? { ...one, state: "working" as const } : one,
  );
  const fixture = { ...base.fixture, conversations } as unknown as FixtureSet;
  const at = (minutes: number) => now - minutes * MIN;
  const feed: FeedItemInfo[] = [
    feedItem({ id: "n:fin", at_ms: at(7), kind: "finished", category: "finished", level: 1, coalesce: "s:login", subject: "Login refactor", author: CLAUDE_APP, facts: { project: "web-app", project_id: "proj-web", duration_ms: 18 * MIN, changes: changed("login", 14, 412, 96) }, target: { session_id: "login", project_id: "proj-web" } }),
    feedItem({ id: "n:ans", at_ms: at(9), kind: "answered", category: "needs", level: 0, coalesce: "s:login", subject: "Login refactor", author: CLAUDE_APP, facts: { project: "web-app", project_id: "proj-web", need: "asks" }, target: { session_id: "login", project_id: "proj-web" } }),
    feedItem({ id: "w:port", at_ms: at(18), kind: "switch", category: "switches", level: 1, coalesce: "w:port", subject: "Port the parser", facts: { project: "web-app", project_id: "proj-web", from_label: "Work Max", from_provider: "claude-code", to_label: "Personal", to_provider: "codex", mechanism: "copy", fidelity: "full" }, target: { project_id: "proj-web" } }),
    feedItem({ id: "l:flaky", at_ms: at(25), kind: "finished", category: "finished", level: 1, coalesce: "s:flaky", subject: "Fix the flaky login test", author: CODEX_PERSONAL, facts: { project: "web-app", project_id: "proj-web", duration_ms: 42 * MIN, quote: "The race was in the session cookie's refresh. I now wait for the refresh before the second request, and the test passed 20 runs in a row." }, target: { session_id: "flaky", project_id: "proj-web" } }),
    feedItem({ id: "m:cents", at_ms: at(41), kind: "message", category: "pools", level: 1, coalesce: "m:cents", subject: "Totals in cents", author: { kind: "agent", name: "Builder", provider: "claude-code", folder: null, folder_label: null, desktop: false }, facts: { pool: "Checkout redesign", pool_id: "pool-checkout", thread: "Totals in cents", change: "question" }, target: { pool_id: "pool-checkout" } }),
    feedItem({ id: "n:quota", at_ms: Date.UTC(2024, 9, 13, 9, 40), kind: "quota", category: "switches", level: 1, coalesce: "q:work", subject: "Work Max", author: { kind: "account", name: "Work Max", provider: null, folder: "claude-work", folder_label: null, desktop: false }, facts: { window: "5-hour", meter: 39, resets_at_ms: Date.UTC(2024, 9, 13, 15, 0) + 0, sessions: 2 }, target: { account_id: "claude-work" } }),
  ];
  // The pool post is followed (a live watch on the pool, the Watches stand-in's).
  const watchView = {
    watches: [{
      watch: { id: "w-10000", targets: [{ kind: "pool", id: "pool-checkout" }], matching: "each", conditions: [{ code: "pool_message" }], lifetime: { kind: "always" }, paused: false, evenInQuietHours: false, createdMs: now - 3 * HOUR },
      words: "Tell me when Checkout redesign gets a message.", echo: { head: [], lifetime: "Always.", quiet: null }, state: "watching",
      ended: null, endedWords: null, endedMs: null, told: 0, lastToldMs: null, lastToldCode: null, lastToldWords: null, version: 1, changedMs: now - 3 * HOUR,
    } satisfies WatchInfo],
    calls: [],
  };
  return { ...base, fixture, feed, summary: away(now, 1, 0.9), messages: { ...base.messages, needs: [] }, notices: { earlier: [] }, watchView };
}

/** 0.5.2 (the RW1 walk's G21, G22, G24): a digest beside the finished
 * posts it tells (they stay posts), its lines on the fixture's sessions
 * (`allStatuses`: sess-6 "Add retries to the sync job" finished, sess-2
 * "Fix the flaky login test" working) and one the rail no longer lists;
 * two Codex threads of one App in two projects (a lean on one leaves the
 * other alone). */
export function releaseWalkFeed(now: number): FeedItemInfo[] {
  const CODEX = { kind: "app", name: null, provider: "codex", folder: "codex-default", folder_label: "ChatGPT", desktop: false } as const;
  const line = (session_id: string, name: string, state: string, at: number) => ({
    session_id, name, state, at_ms: now - at,
    author: { provider: "claude-code", folder: "claude-work", folder_label: "Work Max", desktop: false },
    level: 1, notable: state === "finished", stuck_min: null, left_processes: null, runway_left: null, restarted: false, task: null,
    turns: null, files: null, tests: null, passed: null, failed: null, reverted: false, caveat: null, quote: null, quote_kind: null, picked: null,
  });
  const worked = (id: string, ago: number, subject: string, project: string, projectId: string, names: string[], added: number | null, removed: number | null) =>
    feedItem({
      id: `c:codex-default:${id}`, at_ms: now - ago, kind: "worked", category: "other", level: 0, coalesce: `c:codex-default:${id}`, subject, author: CODEX,
      facts: { project, project_id: projectId, changes: { turn: `${id}:0`, files: names.length, names, added, removed } },
      target: { conversation_id: id, account_id: "codex-default", project_id: projectId },
    });
  return [
    feedItem({
      id: "d:1", at_ms: now - MIN, kind: "agent_digest", category: "digest", level: 1,
      digest: { start_ms: now - 30 * MIN, end_ms: now - MIN, shown: 3, quiet: 0, ai: false, lines: [line("sess-6", "api", "finished", 3 * MIN), line("sess-2", "web", "working", 5 * MIN), line("sess-gone", "docs", "finished", 9 * MIN)] },
    }),
    feedItem({
      id: "l:sess-6", at_ms: now - 3 * MIN, kind: "finished", category: "finished", level: 1, coalesce: "s:sess-6", subject: "Add retries to the sync job", author: WORK,
      facts: { project: "api", project_id: "proj-2", duration_ms: 4 * MIN, changes: { turn: "sess-6:0", files: 3, names: ["src/sync/retry.ts", "src/sync/client.ts", "tests/sync/retry.test.ts"], added: 31, removed: 4 } },
      target: { session_id: "sess-6" },
    }),
    worked("cx-patch", 20 * MIN, "Add a retry helper to the API client", "web", "proj-1", ["src/api/client.py", "src/api/retry.py"], 11, 1),
    worked("cx-delete", 40 * MIN, "Remove the old handlers module", "api", "proj-2", ["src/old_handlers.py", "src/routes.py"], null, null),
  ];
}
