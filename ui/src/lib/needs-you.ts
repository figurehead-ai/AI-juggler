/**
 * AJ-E2502, D-49 — Needs you, as the backend works it out
 * (features/notifications/needs.rs, on its own tick): one list and one
 * count for the rail, Home, the bell, the app badge and the menu-bar item.
 * The window works nothing out: it reads the list (what counts: a snoozed
 * need waits until its snooze ends) and the summary's count, and marks
 * its rows from it. The rail's snoozes are the backend's too.
 */
import { useCallback, useEffect, useMemo, useRef } from "react";
import type { ThisMacInfo } from "./machine";
import type { HomeNeed, HomeNeedKind } from "./home-needs";
import type { Need, RailItem } from "./rail-items";
import { readSnooze, forgetSnooze, type SnoozeState } from "./railview";
import type { AccountInfo, InstanceInfo } from "./bridge";
import { notificationsPage, notificationsSnooze, notificationsSnoozes, standInNeedsPage, type NeedSnoozeInfo, type NoticeInfo } from "./ipc/notices";
import { acted, useNoticeSummary } from "./notices-store";
import { usePagedList, MAX_PAGE } from "./paged";
import { quotaDetail } from "./notice-words";

/** The kinds that are a session's (the rail's Needs you shows them). */
const SESSION_KINDS = new Set<string>(["asks", "stuck", "limit", "failed", "review"]);

/** Home's order: what blocks work first (lib/home-needs.ts). */
const HOME_KIND: Record<string, HomeNeedKind> = { asks: "session", stuck: "session", review: "session", limit: "limit", failed: "failed", quota: "quota", signin: "signin", room: "mac", heavy: "heavy", leftovers: "leftovers" };
const HOME_ORDER: HomeNeedKind[] = ["session", "limit", "failed", "quota", "signin", "mac", "heavy", "leftovers"];
const NEED_ORDER = ["asks", "limit", "failed", "stuck", "review"];

export interface BackendNeeds {
  /** What counts now, oldest first (snoozed ones left out). */
  rows: NoticeInfo[];
  /** The one count (the bell's, the badge's, the menu-bar item's). */
  count: number;
  /** A session's or another app's session's need, by its rail key. */
  needOf: ReadonlyMap<string, Need>;
  /** The first read hasn't answered yet. */
  loading: boolean;
}

/** The backend's Needs you, kept current by its events. */
export function useBackendNeeds(): BackendNeeds {
  const list = usePagedList<NoticeInfo>("notices:needs", (request) => notificationsPage("needs", request), { pageSize: MAX_PAGE, keyOf: (row) => String(row.id), seed: standInNeedsPage });
  const summary = useNoticeSummary();
  return useMemo(() => {
    const rows = list.items.filter((row) => !row.snoozed);
    const needOf = new Map<string, Need>();
    for (const row of rows) if (SESSION_KINDS.has(row.kind)) needOf.set(row.subject, row.kind as Need);
    return { rows, count: summary?.count ?? rows.length, needOf, loading: list.loading };
  }, [list.items, list.loading, summary?.count]);
}

/** The busiest live session on an account (a hand-over starts there). */
export function liveOn(sessions: InstanceInfo[], accountId: string | null | undefined): InstanceInfo | undefined {
  return sessions.find((s) => s.account_id === accountId && (s.status === "running" || s.status === "waiting_user" || s.status === "stuck"));
}

/** Home's rows for the backend's list: a session's need shows its rail
 * item (its place, its words); the rest say what the backend said. Caps,
 * agents' messages and tasks have their own rows (their own controls). */
export function homeNeedsOf(rows: NoticeInfo[], items: RailItem[], accounts: AccountInfo[], sessions: InstanceInfo[], thisMac: ThisMacInfo | null, now: number): HomeNeed[] {
  const byKey = new Map(items.map((item) => [item.key, item]));
  const out: HomeNeed[] = [];
  for (const row of rows) {
    const kind = HOME_KIND[row.kind];
    if (!kind) continue;
    const item = SESSION_KINDS.has(row.kind) ? byKey.get(row.subject) : undefined;
    if (item) {
      out.push({ key: item.key, kind, title: item.title, detail: [item.project, item.place.app, item.place.profile, item.accountLabel !== item.place.profile ? item.accountLabel : null].filter(Boolean).join(" · "), severe: kind !== "session", item });
      continue;
    }
    if (SESSION_KINDS.has(row.kind)) continue; // its row isn't loaded here yet
    const account = row.target.account_id ? accounts.find((entry) => entry.id === row.target.account_id) : undefined;
    out.push({
      key: row.subject,
      kind,
      title: row.title,
      detail: row.kind === "quota" ? quotaDetail(row, now) : row.detail,
      severe: kind === "mac" && thisMac?.machine.summary.level === "pressure",
      account,
      sessionId: row.kind === "quota" ? liveOn(sessions, row.target.account_id)?.id : (row.target.session_id ?? undefined),
    });
  }
  const rank = (need: HomeNeed) => (need.item?.need ? NEED_ORDER.indexOf(need.item.need) : 0);
  return out.sort((a, b) => HOME_ORDER.indexOf(a.kind) - HOME_ORDER.indexOf(b.kind) || rank(a) - rank(b) || (a.item?.lastMs ?? 0) - (b.item?.lastMs ?? 0));
}

/** The kinds counted elsewhere on Home (their own rows). */
export function isOwnRowKind(kind: string): boolean {
  return kind === "cap" || kind === "message" || kind === "task";
}

function stateOf(rows: NeedSnoozeInfo[]): SnoozeState {
  const out: SnoozeState = {};
  for (const row of rows) if (row.subject.startsWith("s:")) out[row.subject.slice(2)] = { until: row.until_ms, muted: row.until_ms === null };
  return out;
}

/** The rail's snoozes, the backend's (D-49): out of Needs you until they
 * end. An updater form too, so Undo applies to the state as it is then.
 * Snoozes this window kept itself before 0.2 move over once. */
export function useRailSnoozes(): [SnoozeState, (next: SnoozeState | ((current: SnoozeState) => SnoozeState)) => void] {
  const list = usePagedList<NeedSnoozeInfo>("notices:snoozes", notificationsSnoozes, { pageSize: MAX_PAGE, keyOf: (row) => row.subject });
  const snooze = useMemo(() => stateOf(list.items), [list.items]);
  const current = useRef(snooze);
  current.current = snooze;
  useEffect(() => {
    const old = readSnooze();
    const ids = Object.keys(old);
    if (ids.length === 0) return;
    forgetSnooze();
    for (const id of ids) {
      const entry = old[id]!;
      if (entry.muted) void acted(() => notificationsSnooze(`s:${id}`, null, true));
      else if (entry.until !== null && entry.until > Date.now()) void acted(() => notificationsSnooze(`s:${id}`, entry.until));
    }
  }, []);
  const update = useCallback((next: SnoozeState | ((state: SnoozeState) => SnoozeState)) => {
    const before = current.current;
    const after = typeof next === "function" ? next(before) : next;
    current.current = after;
    for (const id of new Set([...Object.keys(before), ...Object.keys(after)])) {
      const was = before[id];
      const now = after[id];
      if (was?.until === now?.until && was?.muted === now?.muted) continue;
      void acted(() => notificationsSnooze(`s:${id}`, now ? now.until : null, now ? now.muted || now.until === null : false), "Couldn't snooze it");
    }
  }, []);
  return [snooze, update];
}
