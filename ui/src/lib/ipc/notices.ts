/**
 * AJ-E2501 — the one notification model over IPC (ENGINEERING-GUIDE U3:
 * one thin file per feature): where it stands, its lists a page at a
 * time (Needs you among them: the backend works it out, D-49), and the
 * user's marks. In the preview a small stand-in keeps the same rules (the
 * seam `__AIJ_NOTICES__` seeds it: what AI Juggler itself said, what
 * happened, the mutes, the quiet hours; preview/needs.tsx stands in for
 * the backend's Needs you).
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import { pageOf, type Page, type PageRequest } from "../paged";
import type { NeedSnoozeInfo } from "../../types/generated/NeedSnoozeInfo";
import type { NoticeInfo } from "../../types/generated/NoticeInfo";
import type { NoticeMuteInfo } from "../../types/generated/NoticeMuteInfo";
import type { NoticeSummaryInfo } from "../../types/generated/NoticeSummaryInfo";

export type { NeedSnoozeInfo, NoticeInfo, NoticeMuteInfo, NoticeSummaryInfo };

/** A need as the preview's stand-in for the backend hands it over. */
export type StandInNeed = Pick<NoticeInfo, "kind" | "title" | "detail" | "project_id" | "project" | "target" | "action"> & { key: string };

export type NoticeSection = "needs" | "earlier" | "banners";

/** The backend's events (`views.rs`). */
export const NOTICES_CHANGED = "notifications://changed";
export const NOTICE_OPEN = "notifications://open";
export const NEW_SESSION_ASKED = "app://new-session";

/** The preview's seed. */
export interface NoticeSeed {
  /** AI Juggler's own needs (an account almost out of room). */
  app?: NoticeInfo[];
  /** What happened, newest first. */
  earlier?: NoticeInfo[];
  mutes?: NoticeMuteInfo[];
  /** Inside the quiet hours until then ("09:00"). */
  quiet_until?: string | null;
}

const BANNER_KINDS = new Set(["quota"]);

// --- the preview's stand-in -------------------------------------------------

type Stand = { synced: NoticeInfo[]; keys: Map<string, NoticeInfo>; app: NoticeInfo[]; earlier: NoticeInfo[]; mutes: NoticeMuteInfo[]; snoozes: Map<string, number | null>; quiet: string | null; next: number; listeners: Set<(summary: NoticeSummaryInfo) => void> };
let stand: Stand | null = null;

function standIn(): Stand {
  if (stand) return stand;
  const seed = (window as unknown as { __AIJ_NOTICES__?: NoticeSeed }).__AIJ_NOTICES__ ?? {};
  stand = { synced: [], keys: new Map(), app: structuredClone(seed.app ?? []), earlier: structuredClone(seed.earlier ?? []), mutes: structuredClone(seed.mutes ?? []), snoozes: new Map(), quiet: seed.quiet_until ?? null, next: 10_000, listeners: new Set() };
  return stand;
}

/** Test seam: forget the stand-in (a new seed is read). */
export function resetNoticeStandIn(): void {
  stand = null;
}

function mutedIn(s: Stand, row: NoticeInfo): boolean {
  return s.mutes.some((mute) => (mute.scope === "kind" && mute.value === row.kind) || (mute.scope === "project" && mute.value === row.project_id));
}

/** The snoozes on now (one whose time has come ends, as the tick does). */
function snoozesOn(s: Stand): Map<string, number | null> {
  for (const [subject, until] of s.snoozes) if (until !== null && until <= Date.now()) s.snoozes.delete(subject);
  return s.snoozes;
}

function shaped(s: Stand, row: NoticeInfo): NoticeInfo {
  const muted = mutedIn(s, row);
  const subject = row.subject ?? `notice:${row.id}`;
  const on = snoozesOn(s);
  const snoozed = row.needs && on.has(subject);
  return { ...row, subject, muted, snoozed, snoozed_until_ms: snoozed ? (on.get(subject) ?? null) : null, unread: row.unread && !muted && !snoozed };
}

function openRows(s: Stand): NoticeInfo[] {
  return [...s.synced, ...s.app].map((row) => shaped(s, row)).sort((a, b) => a.created_ms - b.created_ms || a.id - b.id);
}

function bannersOf(s: Stand): NoticeInfo[] {
  return openRows(s)
    .filter((row) => BANNER_KINDS.has(row.kind) && !row.dismissed && !row.muted && !row.snoozed)
    .sort((a, b) => b.created_ms - a.created_ms)
    .slice(0, 3);
}

function summaryOf(s: Stand): NoticeSummaryInfo {
  const open = openRows(s);
  const earlier = s.earlier.map((row) => shaped(s, row));
  const quota = open.filter((row) => row.kind === "quota" && row.facts.resets_at_ms !== null).sort((a, b) => (a.facts.resets_at_ms ?? 0) - (b.facts.resets_at_ms ?? 0))[0];
  const count = (kind: string) => earlier.filter((row) => row.kind === kind).length;
  return {
    // D-49: a snoozed need is out of the count until it wakes.
    count: open.filter((row) => !row.snoozed).length,
    unread: open.filter((row) => row.unread).length + earlier.filter((row) => row.unread).length,
    snoozed: open.filter((row) => row.snoozed).length,
    muted: open.filter((row) => row.muted).length,
    quiet: s.quiet !== null,
    quiet_until: s.quiet,
    banners: bannersOf(s).length,
    digest: {
      finished: count("finished"),
      restarted: count("restarted"),
      resets: count("reset"),
      updates: count("update"),
      next_reset_account: quota?.facts.account ?? null,
      next_reset_ms: quota?.facts.resets_at_ms ?? null,
    },
    version: s.next,
  };
}

function changedIn(s: Stand): NoticeSummaryInfo {
  s.next += 1;
  const summary = summaryOf(s);
  for (const listener of s.listeners) listener(summary);
  return summary;
}

function everyRow(s: Stand): NoticeInfo[] {
  return [...s.synced, ...s.app, ...s.earlier];
}

// --- the calls ----------------------------------------------------------------

/** Where the model stands. */
export async function notificationsSummary(): Promise<NoticeSummaryInfo> {
  if (!hasLiveBackend()) return summaryOf(standIn());
  return invoke<NoticeSummaryInfo>("notifications_summary");
}

/** One page of a list. */
export async function notificationsPage(section: NoticeSection, request: PageRequest): Promise<Page<NoticeInfo>> {
  if (!hasLiveBackend()) {
    const s = standIn();
    const rows = section === "needs" ? openRows(s) : section === "banners" ? bannersOf(s) : s.earlier.map((row) => shaped(s, row));
    return pageOf(rows, request);
  }
  return invoke<Page<NoticeInfo>>("notifications_page", { section, request });
}

/** The preview's Needs you at once (the first render, no read to wait
 * for); `null` with a backend. */
export function standInNeedsPage(limit: number): Page<NoticeInfo> | null {
  return hasLiveBackend() ? null : pageOf(openRows(standIn()), { cursor: null, limit });
}

/** The preview's stand-in for the backend's tick: Needs you now (the
 * live app works it out in the backend, D-49; nothing sends it there). */
export function standInNeeds(needs: StandInNeed[]): NoticeSummaryInfo {
  const s = standIn();
  const keys = new Map<string, NoticeInfo>();
  s.synced = needs.map((need, index) => {
    const kept = s.keys.get(need.key);
    const row: NoticeInfo = kept
      ? { ...kept, kind: need.kind, title: need.title, detail: need.detail, project_id: need.project_id, project: need.project, target: need.target, action: need.action }
      : { id: (s.next += 1), kind: need.kind, needs: true, title: need.title, detail: need.detail, project_id: need.project_id, project: need.project, target: need.target, facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null }, action: need.action, meter: null, created_ms: Date.now() - (needs.length - index) * 60_000, unread: true, subject: need.key, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false };
    keys.set(need.key, row);
    return row;
  });
  s.keys = keys;
  return changedIn(s);
}

/** The preview's seam `__AIJ_NOTICES_FAIL__`: a write that fails, with
 * these words (a failure's toast, in the gallery). */
function refuseIfAsked(): void {
  const words = (window as unknown as { __AIJ_NOTICES_FAIL__?: string }).__AIJ_NOTICES_FAIL__;
  if (words) throw { code: "refused", user_message: words };
}

function mark(id: number | null, change: (row: NoticeInfo) => void): NoticeSummaryInfo {
  refuseIfAsked();
  const s = standIn();
  for (const row of everyRow(s)) if (id === null || row.id === id) change(row);
  return changedIn(s);
}

/** Marks one notice read, or all of them. */
export async function notificationsRead(id: number | null): Promise<NoticeSummaryInfo> {
  if (!hasLiveBackend()) return mark(id, (row) => (row.unread = false));
  return invoke<NoticeSummaryInfo>("notifications_read", { id });
}

/** Snoozes a subject (`s:<session>`, …): out of the Needs you count until
 * a time ahead, or until woken (`forever`); neither wakes it (D-49). */
export async function notificationsSnooze(subject: string, untilMs: number | null, forever = false): Promise<NoticeSummaryInfo> {
  if (!hasLiveBackend()) {
    refuseIfAsked();
    const s = standIn();
    if (untilMs === null && !forever) s.snoozes.delete(subject);
    else s.snoozes.set(subject, untilMs);
    if (untilMs !== null || forever) for (const row of everyRow(s)) if (row.needs && (row.subject ?? "") === subject) row.unread = false;
    return changedIn(s);
  }
  return invoke<NoticeSummaryInfo>("notifications_snooze", { subject, untilMs, forever });
}

/** The snoozes on now, a page at a time (the rail marks its rows). */
export async function notificationsSnoozes(request: PageRequest): Promise<Page<NeedSnoozeInfo>> {
  if (!hasLiveBackend()) return pageOf([...snoozesOn(standIn())].map(([subject, until_ms]) => ({ subject, until_ms })), request);
  return invoke<Page<NeedSnoozeInfo>>("notifications_snoozes", { request });
}

/** Closes a banner (the notice stays in the centre). */
export async function notificationsDismiss(id: number): Promise<NoticeSummaryInfo> {
  if (!hasLiveBackend()) return mark(id, (row) => ((row.dismissed = true), (row.unread = false)));
  return invoke<NoticeSummaryInfo>("notifications_dismiss", { id });
}

/** Mutes a project or a kind. */
export async function notificationsMute(scope: "project" | "kind", value: string, label: string): Promise<NoticeSummaryInfo> {
  if (!hasLiveBackend()) {
    refuseIfAsked();
    const s = standIn();
    s.mutes = [...s.mutes.filter((mute) => !(mute.scope === scope && mute.value === value)), { scope, value, label }];
    return changedIn(s);
  }
  return invoke<NoticeSummaryInfo>("notifications_mute", { scope, value, label });
}

/** Lets a project or a kind interrupt again. */
export async function notificationsUnmute(scope: string, value: string): Promise<NoticeSummaryInfo> {
  if (!hasLiveBackend()) {
    const s = standIn();
    s.mutes = s.mutes.filter((mute) => !(mute.scope === scope && mute.value === value));
    return changedIn(s);
  }
  return invoke<NoticeSummaryInfo>("notifications_unmute", { scope, value });
}

/** The mutes, a page at a time. */
export async function notificationsMutes(request: PageRequest): Promise<Page<NoticeMuteInfo>> {
  if (!hasLiveBackend()) return pageOf(standIn().mutes, request);
  return invoke<Page<NoticeMuteInfo>>("notifications_mutes", { request });
}

/** Every change in the model, with the summary. */
export function onNoticesChanged(handler: (summary: NoticeSummaryInfo) => void): () => void {
  if (!hasLiveBackend()) {
    const s = standIn();
    s.listeners.add(handler);
    return () => s.listeners.delete(handler);
  }
  return listened<NoticeSummaryInfo>(NOTICES_CHANGED, handler);
}

/** The menu-bar item asked to open a notice (an id) or the centre (null). */
export function onNoticeOpen(handler: (id: number | null) => void): () => void {
  if (!hasLiveBackend()) return () => undefined;
  return listened<number | null>(NOTICE_OPEN, handler);
}

/** The menu-bar item's New session…. */
export function onNewSessionAsked(handler: () => void): () => void {
  if (!hasLiveBackend()) return () => undefined;
  return listened<null>(NEW_SESSION_ASKED, () => handler());
}

function listened<T>(event: string, handler: (payload: T) => void): () => void {
  let stop: (() => void) | null = null;
  let gone = false;
  void listen<T>(event, (message) => handler(message.payload)).then((unlisten) => {
    if (gone) unlisten();
    else stop = unlisten;
  });
  return () => {
    gone = true;
    stop?.();
  };
}
