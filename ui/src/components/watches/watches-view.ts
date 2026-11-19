/**
 * AJ-E3816 — what the Watches view works out, kept apart from how it
 * draws: the targets the picker offers (suggested first, then all: Open
 * now, Recent, then All by kind), each target in plain words, a row's
 * state, the conditions every chosen target can report (D-127), the
 * suggested lifetime (ADR-0036 §2.4), a watch as a draft to edit, and the
 * list's search, filters and sort.
 */
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { CatalogueEntry } from "../../types/generated/CatalogueEntry";
import type { LifetimeChoice } from "../../types/generated/LifetimeChoice";
import type { Watch } from "../../types/generated/Watch";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";
import type { WatchInfo } from "../../types/generated/WatchInfo";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import { shortName } from "../../lib/session-titles";
import { clock, pastDay, pastWhen } from "../../lib/time-words";
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

/** Sessions that count as open now. */
const OPEN = new Set(["running", "waiting_user", "stuck", "starting", "quota_blocked", "idle"]);

export interface PoolLike {
  id: string;
  name: string;
  kind: string;
  project_id: string | null;
  members: number;
}

/** What the picker and the rows know about this Mac's targets. */
export interface TargetSources {
  sessions: readonly InstanceInfo[];
  apps: readonly LaunchRecipeInfo[];
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
}

export function targetKey(target: WatchTarget): string {
  return target.kind === "any" ? "any" : `${target.kind}:${target.id}`;
}

export function sameTarget(a: WatchTarget, b: WatchTarget): boolean {
  return targetKey(a) === targetKey(b);
}

const STATUS_WORDS: Record<string, string> = { running: "Working", waiting_user: "Asks you", stuck: "Looks stuck", starting: "Starting", quota_blocked: "At its limit", idle: "Idle" };

function projectName(sources: TargetSources, id: string | null): string | null {
  return id === null ? null : (sources.projects.find((p) => p.id === id)?.name ?? null);
}

/** Every target this Mac has, by key. */
export function targetBook(sources: TargetSources): Map<string, TargetOption> {
  const book = new Map<string, TargetOption>();
  const add = (option: TargetOption) => book.set(option.key, option);
  const appName = (id: string) => sources.apps.find((app) => app.id === id)?.name ?? null;
  for (const session of sources.sessions) {
    const target: WatchTarget = { kind: "session", id: session.id };
    const open = OPEN.has(session.status);
    const parts = [appName(session.launch_recipe_id) ?? session.account_label, projectName(sources, session.project_id), open ? (STATUS_WORDS[session.status] ?? null) : "Ended"].filter(Boolean);
    add({ target, key: targetKey(target), kind: "session", name: shortName(session, sources.now), line: parts.join(" · "), tool: session.provider, open, at: session.last_activity_ms ?? session.started_at_ms });
  }
  for (const app of sources.apps) {
    const target: WatchTarget = { kind: "app", id: app.id };
    const last = Math.max(0, ...sources.sessions.filter((s) => s.launch_recipe_id === app.id).map((s) => s.last_activity_ms ?? s.started_at_ms));
    add({ target, key: targetKey(target), kind: "app", name: app.name, line: `${KIND.app} · ${KIND.everySessionInIt}`, tool: app.provider, open: false, at: last });
  }
  for (const project of sources.projects) {
    const target: WatchTarget = { kind: "project", id: project.id };
    const open = sources.sessions.filter((s) => s.project_id === project.id && OPEN.has(s.status)).length;
    add({ target, key: targetKey(target), kind: "project", name: project.name, line: open > 0 ? `${KIND.project} · ${KIND.openSessions(open)}` : `${KIND.project} · ${KIND.everySessionInIt}`, tool: null, open: false, at: project.last_activity_ms ?? 0 });
  }
  for (const pool of sources.pools) {
    const target: WatchTarget = { kind: "pool", id: pool.id };
    const name = pool.kind === "default" ? (projectName(sources, pool.project_id) ?? pool.name) : pool.name;
    const parts = [KIND.pool, KIND.members(pool.members), projectName(sources, pool.project_id)].filter(Boolean);
    add({ target, key: targetKey(target), kind: "pool", name, line: parts.join(" · "), tool: null, open: false, at: 0 });
  }
  for (const account of sources.accounts) {
    const target: WatchTarget = { kind: "account", id: account.id };
    const signIns = sources.apps.filter((app) => app.account_id === account.id).length;
    const last = Math.max(0, ...sources.sessions.filter((s) => s.account_id === account.id).map((s) => s.last_activity_ms ?? s.started_at_ms));
    add({ target, key: targetKey(target), kind: "account", name: account.label, line: signIns > 0 ? `${KIND.account} · ${KIND.appsSignIn(signIns)}` : `${KIND.account} · ${KIND.roomAndLimits}`, tool: null, open: false, at: last });
  }
  add({ target: { kind: "any" }, key: "any", kind: "any", name: "Any session", line: KIND.any, tool: null, open: false, at: 0 });
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
export function unknownTarget(target: WatchTarget): TargetOption {
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
export function lastWhen(at: number, now: number): string {
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
export const TOP_CODES = ["finishes", "asks_you", "stuck", "fails"] as const;

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
  const at = (hour: number) => {
    const d = new Date(2026, 0, 1, hour, 0, 0, 0);
    return clock(d.getTime());
  };
  return { from: at(prefs.quiet_start), to: at(prefs.quiet_end) };
}
