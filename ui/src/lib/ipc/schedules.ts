/**
 * AJ-E2027 (ADR-0039 §6) — the scheduler's calls: set a reminder (the
 * feed's Remind me, Bot's confirmed draft), the user's list a page at a
 * time with its counts, and the writes the user's own click makes, each
 * by the schedule's id: move or rename, pause or resume, Pause all, and
 * remove. Settings › Advanced reads the app's own schedules, read-only.
 *
 * Without a backend (the preview, the tests) the calls read
 * `window.__AIJ_SCHEDULES__`: its schedules and the calls made.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { Page } from "../../types/generated/Page";
import type { PageRequest } from "../../types/generated/PageRequest";
import type { RemindRequest } from "../../types/generated/RemindRequest";
import type { ScheduleInfo } from "../../types/generated/ScheduleInfo";
import type { ScheduleListWhich } from "../../types/generated/ScheduleListWhich";
import type { SchedulesSummary } from "../../types/generated/SchedulesSummary";

export type { RemindRequest, ScheduleInfo, ScheduleListWhich, SchedulesSummary };

/** The stand-in's state, for previews and tests. */
export interface SchedulesSeam {
  /** The user's schedules. */
  yours?: ScheduleInfo[];
  /** The app's own. */
  app?: ScheduleInfo[];
  /** The scheduler isn't there (an older backend): Remind me hides. */
  off?: boolean;
  calls?: [string, unknown][];
}

declare global {
  interface Window {
    __AIJ_SCHEDULES__?: SchedulesSeam;
  }
}

function seam(): SchedulesSeam {
  window.__AIJ_SCHEDULES__ ??= {};
  return window.__AIJ_SCHEDULES__;
}

function noted(name: string, args: unknown): void {
  (seam().calls ??= []).push([name, args]);
}

function refuseOff(): void {
  if (seam().off) throw new Error("Reminders aren't available in this version.");
}

const live = (s: ScheduleInfo) => s.ended_ms === null;

function counted(list: ScheduleInfo[]): SchedulesSummary {
  const shown = list.filter(live);
  return { active: shown.filter((s) => !s.paused).length, paused: shown.filter((s) => s.paused).length };
}

function touched(id: string, change: (s: ScheduleInfo) => ScheduleInfo): ScheduleInfo {
  const list = seam().yours ?? [];
  const at = list.findIndex((s) => s.id === id);
  if (at < 0) throw new Error("That reminder is gone.");
  const next = change(list[at]!);
  list[at] = next;
  return next;
}

/** Sets a reminder; a time already past is due at once. */
export async function scheduleRemind(request: RemindRequest): Promise<ScheduleInfo> {
  if (hasLiveBackend()) return call("schedule_remind", { request });
  refuseOff();
  noted("schedule_remind", request);
  const made: ScheduleInfo = {
    id: `s-preview-${(seam().yours ?? []).length + 1}`,
    title: request.title,
    handler: "reminder",
    source: "ai_juggler",
    trigger: { kind: "at", at_ms: request.at_ms },
    modifiers: { window: null, jitter_ms: 0, lifetime: { kind: "once" } },
    next_due_ms: request.at_ms,
    last_run_ms: null,
    last_outcome: null,
    paused: false,
    ended_ms: null,
    created_ms: Date.now(),
    created_by: request.chat_turn ? "bot" : "user",
    about: request.about ?? null,
  };
  (seam().yours ??= []).unshift(made);
  return made;
}

/** One page of a list: the user's own, or the app's (read-only). */
export async function schedulesPage(which: ScheduleListWhich, request: PageRequest): Promise<Page<ScheduleInfo>> {
  if (hasLiveBackend()) return call("schedule_list", { request, which });
  refuseOff();
  const all = (which === "app" ? seam().app : seam().yours) ?? [];
  const start = request.cursor ? Number(request.cursor) : 0;
  const limit = request.limit ?? 50;
  const next = start + limit < all.length ? String(start + limit) : null;
  return { items: all.slice(start, start + limit), next_cursor: next, total: null };
}

/** The user's list at a glance: how many run, how many are paused. */
export async function schedulesSummary(): Promise<SchedulesSummary> {
  if (hasLiveBackend()) return call("schedule_summary");
  refuseOff();
  return counted(seam().yours ?? []);
}

/** Moves a reminder to another time, or renames it. */
export async function scheduleUpdate(id: string, atMs: number, title: string): Promise<ScheduleInfo> {
  if (hasLiveBackend()) return call("schedule_update", { id, atMs, title });
  noted("schedule_update", { id, atMs, title });
  return touched(id, (s) => ({ ...s, title, trigger: { kind: "at", at_ms: atMs }, next_due_ms: atMs, ended_ms: null }));
}

/** Pauses or resumes one. */
export async function schedulePause(id: string, paused: boolean): Promise<ScheduleInfo> {
  if (hasLiveBackend()) return call("schedule_pause", { id, paused });
  noted("schedule_pause", { id, paused });
  return touched(id, (s) => ({ ...s, paused }));
}

/** Pause all, or Resume all. */
export async function schedulesPauseAll(paused: boolean): Promise<SchedulesSummary> {
  if (hasLiveBackend()) return call("schedule_pause_all", { paused });
  noted("schedule_pause_all", { paused });
  const list = seam().yours ?? [];
  list.forEach((s, i) => {
    if (live(s)) list[i] = { ...s, paused };
  });
  return counted(list);
}

/** Removes one. */
export async function scheduleRemove(id: string): Promise<void> {
  if (hasLiveBackend()) return call("schedule_remove", { id });
  noted("schedule_remove", { id });
  const s = seam();
  s.yours = (s.yours ?? []).filter((one) => one.id !== id);
}
