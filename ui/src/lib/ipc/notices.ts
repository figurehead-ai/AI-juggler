/**
 * AJ-E2501 — the one notification model over IPC (ENGINEERING-GUIDE U3:
 * one thin file per feature): where it stands, its lists a page at a
 * time, the Needs you list kept in step, and the user's marks. In the
 * preview a small stand-in keeps the same rules (the seam
 * `__AIJ_NOTICES__` seeds it: what AI Juggler itself said, what happened,
 * the mutes, the quiet hours).
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import { pageOf, type Page, type PageRequest } from "../paged";
import type { NeedInput } from "../../types/generated/NeedInput";
import type { NoticeInfo } from "../../types/generated/NoticeInfo";
import type { NoticeMuteInfo } from "../../types/generated/NoticeMuteInfo";
import type { NoticeSummaryInfo } from "../../types/generated/NoticeSummaryInfo";
import type { OpenNowInput } from "../../types/generated/OpenNowInput";

export type { NeedInput, NoticeInfo, NoticeMuteInfo, NoticeSummaryInfo, OpenNowInput };

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

type Stand = { synced: NoticeInfo[]; keys: Map<string, NoticeInfo>; app: NoticeInfo[]; earlier: NoticeInfo[]; mutes: NoticeMuteInfo[]; quiet: string | null; next: number; listeners: Set<(summary: NoticeSummaryInfo) => void> };
let stand: Stand | null = null;

function standIn(): Stand {
  if (stand) return stand;
  const seed = (window as unknown as { __AIJ_NOTICES__?: NoticeSeed }).__AIJ_NOTICES__ ?? {};
  stand = { synced: [], keys: new Map(), app: structuredClone(seed.app ?? []), earlier: structuredClone(seed.earlier ?? []), mutes: structuredClone(seed.mutes ?? []), quiet: seed.quiet_until ?? null, next: 10_000, listeners: new Set() };
  return stand;
}

/** Test seam: forget the stand-in (a new seed is read). */
export function resetNoticeStandIn(): void {
  stand = null;
}

function mutedIn(s: Stand, row: NoticeInfo): boolean {
  return s.mutes.some((mute) => (mute.scope === "kind" && mute.value === row.kind) || (mute.scope === "project" && mute.value === row.project_id));
}

function shaped(s: Stand, row: NoticeInfo): NoticeInfo {
  const muted = mutedIn(s, row);
  const snoozed = row.snoozed_until_ms !== null && row.snoozed_until_ms > Date.now() ? row.snoozed_until_ms : null;
  return { ...row, muted, snoozed_until_ms: snoozed, unread: row.unread && !muted && snoozed === null };
}

function openRows(s: Stand): NoticeInfo[] {
  return [...s.synced, ...s.app].map((row) => shaped(s, row)).sort((a, b) => a.created_ms - b.created_ms || a.id - b.id);
}

function bannersOf(s: Stand): NoticeInfo[] {
  return openRows(s)
    .filter((row) => BANNER_KINDS.has(row.kind) && !row.dismissed && !row.muted && row.snoozed_until_ms === null)
    .sort((a, b) => b.created_ms - a.created_ms)
    .slice(0, 3);
}

function summaryOf(s: Stand): NoticeSummaryInfo {
  const open = openRows(s);
  const earlier = s.earlier.map((row) => shaped(s, row));
  const quota = open.filter((row) => row.kind === "quota" && row.facts.resets_at_ms !== null).sort((a, b) => (a.facts.resets_at_ms ?? 0) - (b.facts.resets_at_ms ?? 0))[0];
  const count = (kind: string) => earlier.filter((row) => row.kind === kind).length;
  return {
    count: open.length,
    unread: open.filter((row) => row.unread).length + earlier.filter((row) => row.unread).length,
    snoozed: open.filter((row) => row.snoozed_until_ms !== null).length,
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

/** The Needs you list now, and what is open. */
export async function notificationsSync(needs: NeedInput[], open: OpenNowInput): Promise<NoticeSummaryInfo> {
  if (!hasLiveBackend()) {
    const s = standIn();
    const keys = new Map<string, NoticeInfo>();
    s.synced = needs.map((need, index) => {
      const kept = s.keys.get(need.key);
      const row: NoticeInfo = kept
        ? { ...kept, kind: need.kind, title: need.title, detail: need.detail, project_id: need.project_id, project: need.project, target: need.target, action: need.action }
        : { id: (s.next += 1), kind: need.kind, needs: true, title: need.title, detail: need.detail, project_id: need.project_id, project: need.project, target: need.target, facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null }, action: need.action, meter: null, created_ms: Date.now() - (needs.length - index) * 60_000, unread: true, snoozed_until_ms: null, muted: false, dismissed: false };
      keys.set(need.key, row);
      return row;
    });
    s.keys = keys;
    return changedIn(s);
  }
  return invoke<NoticeSummaryInfo>("notifications_sync", { needs, open });
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

/** Snoozes a notice until a time ahead (or wakes it). */
export async function notificationsSnooze(id: number, untilMs: number | null): Promise<NoticeSummaryInfo> {
  if (!hasLiveBackend()) return mark(id, (row) => ((row.snoozed_until_ms = untilMs), untilMs !== null && (row.unread = false)));
  return invoke<NoticeSummaryInfo>("notifications_snooze", { id, untilMs });
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
