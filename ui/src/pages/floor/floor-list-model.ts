/**
 * AJ-E3512 — the Floor list's model: the Apps by floor in the Floor's
 * order, each App's sessions waiting-first, a floor's columns (only those
 * with a value on it), its Accounts in one line, and the rows a search or
 * the "Waiting on you" chip leaves, flat, for the shared paged list.
 */
import type { FloorApp, FloorSession, FloorSnapshot, FloorState } from "../../lib/ipc/floor";
import type { FloorPlaceInfo } from "../../lib/ipc/floor-controls";
import { agoWords } from "../../lib/time-words";

/** Each state's glyph (decoration: the word beside it says the same). */
export const GLYPH: Record<FloorState, string> = { asks: "!", working: "▸", stuck: "?", limit: "⏸", crashed: "✕", done: "✓", idle: "·", ended: "–" };
const WAITING: readonly FloorState[] = ["asks", "stuck", "limit", "crashed", "done"];

export function isWaiting(session: FloorSession): boolean {
  return WAITING.includes(session.state);
}

export interface FloorGroup {
  floor: number;
  apps: FloorApp[];
}

/** The apps by floor, in the Floor's order; an App the plan doesn't place
 * (left off the Floor) isn't listed. With no plan yet, one floor. */
export function groups(snapshot: FloorSnapshot, places: FloorPlaceInfo[]): FloorGroup[] {
  const at = new Map(places.map((place) => [place.app_id, place]));
  const placed = places.length > 0 ? snapshot.apps.filter((app) => at.has(app.id)) : [...snapshot.apps];
  placed.sort((a, b) => (at.get(a.id)?.order ?? 0) - (at.get(b.id)?.order ?? 0));
  const out = new Map<number, FloorApp[]>();
  for (const app of placed) {
    const floor = at.get(app.id)?.floor ?? 0;
    out.set(floor, [...(out.get(floor) ?? []), app]);
  }
  return [...out.entries()].sort((a, b) => a[0] - b[0]).map(([floor, apps]) => ({ floor, apps }));
}

/** An App's sessions, those waiting on you first, each group in the
 * Floor's own order (principle 11: what needs you leads). */
export function waitingFirst(sessions: FloorSession[]): FloorSession[] {
  return [...sessions.filter(isWaiting), ...sessions.filter((s) => !isWaiting(s))];
}

/** How many sessions wait on you, over every floor. */
export function waitingCount(list: FloorGroup[]): number {
  return list.reduce((n, group) => n + group.apps.reduce((m, app) => m + app.sessions.filter(isWaiting).length, 0), 0);
}

/** "2 floors · 7 Apps · 4 need you · 3 sessions waiting": Needs you's one
 * count (`needs`, the bell's and Home's number, AJ-E2502), then the
 * sessions waiting in the office; each left out at zero. */
export function summary(list: FloorGroup[], needs: number): string {
  const apps = list.reduce((n, group) => n + group.apps.length, 0);
  const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
  const waiting = waitingCount(list);
  return [
    plural(list.length, "floor", "floors"),
    plural(apps, "App", "Apps"),
    needs > 0 ? `${needs} ${needs === 1 ? "needs" : "need"} you` : null,
    waiting > 0 ? `${plural(waiting, "session", "sessions")} waiting` : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

/** The columns past State and Session a floor has a value in (the 0.4
 * review: columns of "—" are left out). */
export interface FloorColumns {
  project: boolean;
  subagents: boolean;
  tasks: boolean;
}

export function floorColumns(group: FloorGroup): FloorColumns {
  const sessions = group.apps.flatMap((app) => app.sessions);
  return { project: sessions.some((s) => !!s.project), subagents: sessions.some((s) => s.subagents > 0), tasks: sessions.some((s) => s.tasks > 0) };
}

/** The grid every row of a floor shares, so its columns line up under
 * its one header. */
export function columnTemplate(columns: FloorColumns): string {
  return ["minmax(8rem, 11rem)", "minmax(0, 1fr)", columns.project ? "minmax(0, 12rem)" : null, columns.subagents ? "6rem" : null, columns.tasks ? "4.5rem" : null].filter(Boolean).join(" ");
}

/** A floor's Accounts, each once, with its room ("Work Max · 62% left"). */
export function floorAccounts(group: FloorGroup, snapshot: FloorSnapshot): string[] {
  const ids = [...new Set(group.apps.flatMap((app) => (app.account_id ? [app.account_id] : [])))];
  return ids.flatMap((id) => {
    const account = snapshot.accounts.find((entry) => entry.id === id);
    if (!account) return [];
    return [account.out ? `${account.label} · out of room` : account.left_pct != null ? `${account.label} · ${Math.round(account.left_pct)}% left` : `${account.label} · not measured yet`];
  });
}

/** What an App's header says beside its name: a busy processor, unused,
 * or that it has no sessions. */
export function appNote(app: FloorApp, now: number): string {
  const since = (ms: number) => agoWords(ms, now).replace(" ago", "");
  const hot = app.hot_since_ms != null ? `Busy processor · ${since(app.hot_since_ms)}` : null;
  const unused = !app.open && app.sessions.length === 0 && app.last_active_ms != null ? `Unused · ${since(app.last_active_ms)}` : null;
  const none = app.sessions.length === 0 ? `No sessions${app.last_active_ms != null && !unused ? ` · last active ${agoWords(app.last_active_ms, now)}` : ""}` : null;
  return [hot, unused ?? none].filter(Boolean).join(" · ");
}

/** A row of a floor's list: an App's header, or one of its sessions. */
export type FloorRow = { kind: "app"; key: string; app: FloorApp } | { kind: "session"; key: string; app: FloorApp; session: FloorSession };

/** A floor's rows, as a search and the waiting chip leave them. An App
 * whose name matches keeps all its sessions; otherwise it shows the ones
 * that match, and goes when none do. */
export function floorRows(group: FloorGroup, query: string, waitingOnly: boolean): FloorRow[] {
  const words = query.trim().toLowerCase();
  const has = (text: string | null | undefined) => !!text && text.toLowerCase().includes(words);
  const rows: FloorRow[] = [];
  for (const app of group.apps) {
    const named = words === "" || has(app.name);
    const sessions = waitingFirst(app.sessions).filter((s) => (!waitingOnly || isWaiting(s)) && (named || has(s.title) || has(s.project) || has(s.word)));
    // Narrowed, an App is listed for what it holds; unfiltered, every App is.
    if (sessions.length === 0 && (waitingOnly || !named)) continue;
    rows.push({ kind: "app", key: `app:${app.id}`, app });
    for (const session of sessions) rows.push({ kind: "session", key: `s:${app.id}:${session.key}`, app, session });
  }
  return rows;
}

/** An App's own name, split as the rail shows it: its app ("Claude
 * Code") and its profile ("Work Max"), when it has one. */
export function appParts(app: FloorApp): { app: string; profile: string | null } {
  const at = app.name.indexOf(" · ");
  return at < 0 ? { app: app.name, profile: null } : { app: app.name.slice(0, at), profile: app.name.slice(at + 3) };
}
