/**
 * AJ-E3816 — what the Watches view works out, kept apart from how it
 * draws: the targets the picker offers (suggested first, then all: Open
 * now, Recent, then All by kind), each target in plain words, a row's
 * state, the conditions every chosen target can report (D-127), the
 * suggested lifetime (ADR-0036 §2.4), a watch as a draft to edit, and the
 * list's search, filters and sort.
 */
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ConversationInfo } from "../../types/generated/ConversationInfo";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { CatalogueEntry } from "../../types/generated/CatalogueEntry";
import type { LifetimeChoice } from "../../types/generated/LifetimeChoice";
import type { Watch } from "../../types/generated/Watch";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";
import type { WatchInfo } from "../../types/generated/WatchInfo";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import { shortName } from "../../lib/session-titles";
import { conversationItem, needWord, openNow as railOpenNow, sessionState } from "../../lib/rail-items";
import { conversationTitle } from "../../lib/conversations";
import { switchSourceOf } from "../../lib/ipc/conversations";
import { clock, hourWords, pastDay, pastWhen } from "../../lib/time-words";
import { ENDED, KIND, ROW } from "./watches-words";

/** The most targets a watch has (the backend's MAX_TARGETS). */
export const MAX_TARGETS = 10;
/** The most conditions a watch has. */
export const MAX_CONDITIONS = 4;
/** Rows past which the list gets its toolbar and goes compact. */
export const MANY_AT = 8;
/** Rows a compact list shows at first, then 20 more at a time. */
export const FIRST_ROWS = 10;
export const MORE_ROWS = 20;

/** 0.5.2 G34 (RW1: the picker listed idle sessions as "Working"): a
 * session's state and its word are the rail's, by one rule
 * (`sessionState`: the live state its hooks report, its need by D-117),
 * never its stored status alone. Open: any state but ended. */
function liveState(session: InstanceInfo): { open: boolean; word: string } {
  const now = sessionState(session);
  return { open: now.state !== "ended", word: now.word };
}

/** 0.5.2 G45: a session AI Juggler didn't start (one in the user's own
 * terminal, another app's), by the same rule and words as its rail row
 * (`conversationItem`): its word, and whether Open now lists it
 * (`openNow`). */
function conversationState(c: ConversationInfo): { open: boolean; word: string } {
  const item = conversationItem(c, () => null, null);
  return { open: railOpenNow(item), word: needWord(item) };
}

/** An App as the Watches view needs it (an `AppModel`): its App id
 * (`cli:<folder>`, `app:<host>:<instance>`), the one every surface uses. */
export interface AppLike {
  id: string;
  kind: "cli" | "desktop";
  name: string;
  tool: string | null;
  folder: { id: string } | null;
}

/** The App a session is in, by App id: the backend's rule
 * (`sessions().pool_facts_of`): the one desktop App on its folder for a
 * desktop session, else its folder's CLI App. */
export function appOfSession(session: Pick<InstanceInfo, "account_id" | "kind">, apps: readonly AppLike[]): AppLike | null {
  const folder = session.account_id;
  if (folder === null) return null;
  if (session.kind === "desktop") {
    const desktop = apps.filter((app) => app.kind === "desktop" && app.folder?.id === folder);
    if (desktop.length === 1) return desktop[0]!;
  }
  return apps.find((app) => app.id === `cli:${folder}`) ?? null;
}

interface PoolLike {
  id: string;
  name: string;
  kind: string;
  project_id: string | null;
  members: number;
}

/** What the picker and the rows know about this Mac's targets. */
export interface TargetSources {
  sessions: readonly InstanceInfo[];
  /** 0.5.2 G45: the sessions AI Juggler didn't start, as the rail has
   * them (`useConversations`); none when left out. */
  conversations?: readonly ConversationInfo[];
  apps: readonly AppLike[];
  projects: readonly { id: string; name: string; last_activity_ms?: number | null }[];
  pools: readonly PoolLike[];
  accounts: readonly AccountInfo[];
  now: number;
}

/** One target as the picker and the rows show it. */
export interface TargetOption {
  target: WatchTarget;
  key: string;
  kind: WatchTarget["kind"];
  name: string;
  /** Its second line ("Agent CLI · shop · Working"). */
  line: string;
  /** The tool whose icon marks it (a session's or App's), else null. */
  tool: string | null;
  /** Open now (a session that runs). */
  open: boolean;
  /** When it was last active, for Recent. */
  at: number;
  /** 0.5.2 G32: its sessions open now (an App, a project, any session). */
  openSessions?: number;
}

export function targetKey(target: WatchTarget): string {
  return target.kind === "any" ? "any" : `${target.kind}:${target.id}`;
}

export function sameTarget(a: WatchTarget, b: WatchTarget): boolean {
  return targetKey(a) === targetKey(b);
}

function projectName(sources: TargetSources, id: string | null): string | null {
  return id === null ? null : (sources.projects.find((p) => p.id === id)?.name ?? null);
}

/** One session as the Watches view reads it, wherever it was started. */
export interface SeenSession {
  /** Its id where a session id goes: AI Juggler's own, or (another app's,
   * one in the user's own terminal) its key, `conversation:<account>:<id>`. */
  id: string;
  name: string;
  /** Its state in the rail's words. */
  word: string;
  /** Open now, as Open now lists it. */
  open: boolean;
  app: AppLike | null;
  appName: string;
  projectId: string | null;
  accountId: string | null;
  tool: string;
  at: number;
}

/** 0.5.2 G45 (RW1 W1): every session this Mac sees, in one read: AI
 * Juggler's own and the ones it didn't start, as the rail lists them
 * (`itemsOf`: no AI Juggler session on it, not put away), each in its
 * rail state. The picker, a target's open sessions and its counts read
 * only this. */
export function seenSessions(sources: TargetSources): SeenSession[] {
  const own = sources.sessions.map((session): SeenSession => {
    const { open, word } = liveState(session);
    const app = appOfSession(session, sources.apps);
    return { id: session.id, name: shortName(session, sources.now), word, open, app, appName: app?.name ?? session.account_label, projectId: session.project_id, accountId: session.account_id, tool: session.provider, at: session.last_activity_ms ?? session.started_at_ms };
  });
  const outside = (sources.conversations ?? [])
    .filter((c) => c.session_id === null && c.app_archived !== true && c.archived_here !== true && c.app_unlisted !== true)
    .map((c): SeenSession => {
      const { open, word } = conversationState(c);
      const app = sources.apps.find((a) => a.id === c.app) ?? null;
      return { id: switchSourceOf(c), name: conversationTitle(c, sources.now), word, open, app, appName: app?.name ?? c.account_label, projectId: c.project_id, accountId: c.account_id, tool: c.tool, at: c.last_activity_ms ?? c.started_ms ?? 0 };
    });
  return [...own, ...outside];
}

/** Every target this Mac has, by key. */
export function targetBook(sources: TargetSources): Map<string, TargetOption> {
  const book = new Map<string, TargetOption>();
  const add = (option: TargetOption) => book.set(option.key, option);
  const seen = seenSessions(sources);

  for (const session of seen) {
    const target: WatchTarget = { kind: "session", id: session.id };
    const parts = [session.appName, projectName(sources, session.projectId), session.word].filter(Boolean);
    add({ target, key: targetKey(target), kind: "session", name: session.name, line: parts.join(" · "), tool: session.tool, open: session.open, at: session.at });
  }
  for (const app of sources.apps) {
    const target: WatchTarget = { kind: "app", id: app.id };
    const its = seen.filter((s) => s.app?.id === app.id);
    const last = Math.max(0, ...its.map((s) => s.at));
    add({ target, key: targetKey(target), kind: "app", name: app.name, line: `${KIND.app} · ${KIND.everySessionInIt}`, tool: app.tool, open: false, at: last, openSessions: its.filter((s) => s.open).length });
  }
  for (const project of sources.projects) {
    const target: WatchTarget = { kind: "project", id: project.id };
    const open = seen.filter((s) => s.projectId === project.id && s.open).length;
    add({ target, key: targetKey(target), kind: "project", name: project.name, line: open > 0 ? `${KIND.project} · ${KIND.openSessions(open)}` : `${KIND.project} · ${KIND.everySessionInIt}`, tool: null, open: false, at: project.last_activity_ms ?? 0, openSessions: open });
  }
  for (const pool of sources.pools) {
    const target: WatchTarget = { kind: "pool", id: pool.id };
    const name = pool.kind === "default" ? (projectName(sources, pool.project_id) ?? pool.name) : pool.name;
    const parts = [KIND.pool, KIND.members(pool.members), projectName(sources, pool.project_id)].filter(Boolean);
    add({ target, key: targetKey(target), kind: "pool", name, line: parts.join(" · "), tool: null, open: false, at: 0 });
  }
  for (const account of sources.accounts) {
    const target: WatchTarget = { kind: "account", id: account.id };
    const signIns = sources.apps.filter((app) => app.folder?.id === account.id).length;
    const last = Math.max(0, ...seen.filter((s) => s.accountId === account.id).map((s) => s.at));
    add({ target, key: targetKey(target), kind: "account", name: account.label, line: signIns > 0 ? `${KIND.account} · ${KIND.appsSignIn(signIns)}` : `${KIND.account} · ${KIND.roomAndLimits}`, tool: null, open: false, at: last });
  }
  add({ target: { kind: "any" }, key: "any", kind: "any", name: "Any session", line: KIND.any, tool: null, open: false, at: 0, openSessions: seen.filter((s) => s.open).length });
  return book;
}

export const KIND_ORDER: readonly Exclude<WatchTarget["kind"], "any">[] = ["session", "app", "project", "pool", "account"];

/** The picker's lists: Open now (open sessions, newest first), Recent
 * (the current project and the account in use), and All
 * by kind with counts. With `query`, one list of what matches. */
export function pickerLists(book: Map<string, TargetOption>, query: string): { openNow: TargetOption[]; recent: TargetOption[]; counts: Record<WatchTarget["kind"], number>; matches: TargetOption[] | null } {
  const all = [...book.values()];
  const counts = { session: 0, app: 0, project: 0, pool: 0, account: 0, any: 1 } as Record<WatchTarget["kind"], number>;
  for (const option of all) if (option.kind !== "any") counts[option.kind] += 1;
  const newest = (a: TargetOption, b: TargetOption) => b.at - a.at || a.name.localeCompare(b.name);
  const openNow = all.filter((o) => o.kind === "session" && o.open).sort(newest);
  // The current project and the account in use: the newest of each.
  const newestOf = (kind: WatchTarget["kind"]) => all.filter((o) => o.kind === kind && o.at > 0).sort(newest)[0];
  const recent = [newestOf("project"), newestOf("account")].filter((o): o is TargetOption => o !== undefined);
  const words = query.trim().toLowerCase();
  const matches = words === "" ? null : all.filter((o) => `${o.name} ${o.line}`.toLowerCase().includes(words)).sort((a, b) => Number(b.open) - Number(a.open) || newest(a, b));
  return { openNow, recent, counts, matches };
}

/** One kind's list, in All: newest first, open sessions on top. */
export function kindList(book: Map<string, TargetOption>, kind: WatchTarget["kind"]): TargetOption[] {
  return [...book.values()].filter((o) => o.kind === kind).sort((a, b) => Number(b.open) - Number(a.open) || b.at - a.at || a.name.localeCompare(b.name));
}

/** A target the book doesn't know (gone, or not read yet). */
function unknownTarget(target: WatchTarget): TargetOption {
  const kind = target.kind;
  // Not on this Mac's lists right now (an ended session, one not read
  // yet): said by its kind, never called removed (only its watch's ending
  // says that).
  return { target, key: targetKey(target), kind, name: kind === "any" ? "Any session" : KIND[kind], line: kind === "any" ? KIND.any : KIND[kind], tool: null, open: false, at: 0 };
}

export function optionOf(book: Map<string, TargetOption>, target: WatchTarget): TargetOption {
  return book.get(targetKey(target)) ?? unknownTarget(target);
}

/** A row's second line: the target in plain words. */
export function targetLine(watch: Watch, book: Map<string, TargetOption>): string {
  const options = watch.targets.map((target) => optionOf(book, target));
  if (options.length === 1) {
    const only = options[0]!;
    if (only.kind === "any") return KIND.any;
    if (only.kind === "session" && only.line === KIND.session) return KIND.session;
    if (only.kind === "session") return [KIND.session, only.line.split(" · ").slice(0, 2).join(" · ")].filter(Boolean).join(" · ");
    return only.line;
  }
  const kinds = new Set(options.map((o) => o.kind));
  if (kinds.size === 1 && kinds.has("session")) {
    const apps = [...new Set(options.map((o) => o.line.split(" · ")[0]).filter(Boolean))];
    return [KIND.sessions(options.length), apps.join(", ")].filter(Boolean).join(" · ");
  }
  return KIND.targets(options.length);
}

/** The chip for a watch's lifetime: the echo's own sentence. */
export function lifetimeChip(info: WatchInfo): string {
  return info.echo.lifetime.replace(/\.$/, "");
}

export type StateTone = "told" | "waiting" | "paused";

/** A row's state column: its top line, its line under, its dot. */
export function rowState(info: WatchInfo, now: number): { tone: StateTone; top: string; sub: string } {
  if (info.state === "paused") {
    // Paused since its last change (a pause is a new version).
    return { tone: "paused", top: ROW.paused, sub: ROW.pausedSince(pastWhen(info.changedMs, now), info.told) };
  }
  if (info.told > 0 && info.lastToldMs !== null) {
    // What it told is said for today's (the mock's "last at 14:32 · finished"); an older one is its day alone.
    const today = info.lastToldMs >= nextMidnight(now) - 86_400_000;
    return { tone: "told", top: ROW.toldTimes(info.told), sub: ROW.last(lastWhen(info.lastToldMs, now), today ? info.lastToldWords : null) };
  }
  return { tone: "waiting", top: ROW.waiting, sub: ROW.neverTold };
}

/** "at 14:32" today, "yesterday 22:41", "Tue 11:05", "Nov 17 11:05". */
function lastWhen(at: number, now: number): string {
  const words = pastWhen(at, now);
  return words === clock(at) ? `at ${words}` : words;
}

/** An ended row's time: "Ended 16:40", "Ended at midnight", "Ended Nov 18". */
export function endedWhen(info: WatchInfo, now: number): string {
  const at = info.endedMs;
  if (at === null) return ENDED.endedAt("");
  const local = new Date(at);
  if (info.ended === "time_reached" && local.getHours() === 0 && local.getMinutes() === 0) return ENDED.endedAtMidnight;
  // Today by its clock, else by its day ("Ended Tue"), so the column holds one line.
  return ENDED.endedAt(pastDay(at, now));
}

/** The conditions every target can report: the catalogues' intersection,
 * in the first one's order (D-127: what one can't report isn't shown). */
export function intersect(catalogues: readonly CatalogueEntry[][]): CatalogueEntry[] {
  if (catalogues.length === 0) return [];
  const [first, ...rest] = catalogues;
  return first!.filter((entry) => rest.every((list) => list.some((other) => other.code === entry.code)));
}

/** The four most used first; the rest fold under "N more". */
const TOP_CODES = ["finishes", "asks_you", "stuck", "fails"] as const;

export function splitConditions(entries: readonly CatalogueEntry[]): { top: CatalogueEntry[]; more: CatalogueEntry[] } {
  const top = TOP_CODES.map((code) => entries.find((e) => e.code === code)).filter((e): e is CatalogueEntry => e !== undefined);
  const more = entries.filter((e) => !top.includes(e));
  // With few of the common ones on offer, the others move up.
  while (top.length < 2 && more.length > 0) top.push(more.shift()!);
  return { top, more };
}

/** The lifetime ADR-0036 §2.4 suggests: Once for finishing or ending on
 * sessions, else Today. */
export function suggestedLifetime(targets: readonly WatchTarget[], codes: readonly string[]): "once" | "today" {
  const sessions = targets.length > 0 && targets.every((t) => t.kind === "session");
  const ending = codes.length > 0 && codes.every((c) => c === "finishes" || c === "ends");
  return sessions && ending ? "once" : "today";
}

/** Whether "Until it ends" can be offered: every target is a session. */
export function untilEndsOffered(targets: readonly WatchTarget[]): boolean {
  return targets.length > 0 && targets.every((t) => t.kind === "session");
}

/** A watch as a draft to edit (its lifetime as it stands). */
export function draftOf(watch: Watch, nextMidnightMs: number): WatchDraftInfo {
  const life = watch.lifetime;
  const lifetime: LifetimeChoice =
    life.kind === "once" ? { kind: "once" } : life.kind === "until_ends" ? { kind: "until_ends" } : life.kind === "always" ? { kind: "always" } : life.atMs === nextMidnightMs ? { kind: "today" } : { kind: "until", atMs: life.atMs };
  return { targets: watch.targets, matching: watch.matching, conditions: watch.conditions, lifetime, evenInQuietHours: watch.evenInQuietHours };
}

/** The local midnight that ends "today". */
export function nextMidnight(now: number): number {
  const day = new Date(now);
  day.setHours(24, 0, 0, 0);
  return day.getTime();
}

export type ListFilter = "all" | "told_today" | "never" | "paused";
export type ListSort = "listed" | "last" | "newest" | "target";

/** The list after search, filter and sort (at most 50 rows: one read). */
export function shownWatches(list: readonly WatchInfo[], opts: { query: string; filter: ListFilter; sort: ListSort; now: number; nameOf: (info: WatchInfo) => string }): WatchInfo[] {
  const midnight = nextMidnight(opts.now) - 86_400_000;
  const words = opts.query.trim().toLowerCase();
  const kept = list.filter((info) => {
    if (words && !`${info.words} ${opts.nameOf(info)}`.toLowerCase().includes(words)) return false;
    if (opts.filter === "told_today") return info.lastToldMs !== null && info.lastToldMs >= midnight;
    if (opts.filter === "never") return info.told === 0;
    if (opts.filter === "paused") return info.state === "paused";
    return true;
  });
  const by: Record<ListSort, (a: WatchInfo, b: WatchInfo) => number> = {
    listed: () => 0,
    last: (a, b) => (b.lastToldMs ?? -1) - (a.lastToldMs ?? -1) || b.watch.createdMs - a.watch.createdMs,
    newest: (a, b) => b.watch.createdMs - a.watch.createdMs,
    target: (a, b) => opts.nameOf(a).localeCompare(opts.nameOf(b)),
  };
  // Paused ones sit at the foot, as the mock lists them.
  return [...kept].sort((a, b) => Number(a.state === "paused") - Number(b.state === "paused") || by[opts.sort](a, b));
}

/** Quiet hours as the user set them; `null` when they have none. */
export function quietHours(prefs: { quiet_start: number; quiet_end: number } | null): { from: string; to: string } | null {
  if (!prefs || prefs.quiet_start === prefs.quiet_end) return null;
  return { from: hourWords(prefs.quiet_start), to: hourWords(prefs.quiet_end) };
}
