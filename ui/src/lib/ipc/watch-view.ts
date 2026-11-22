/**
 * AJ-E3816 — the Watches view's calls (ADR-0036 §2.6): the live watches
 * (at most 50, one bounded read) and the ended ones a page at a time, the
 * counts, what a target can be watched for (only what its Apps report,
 * D-127), a draft's echo in parts (nothing written), and the writes the
 * user's own click makes, each by the watch's id: create, change, pause or
 * resume, remove, and bring back (Undo).
 *
 * Without a backend (the preview, the tests) the page reads
 * `window.__AIJ_WATCH_VIEW__`: its watches, catalogue, counts and the
 * calls made; a draft's words come from the preview's stand-in echo,
 * tested against the backend's own cases.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import { previewHooks } from "../preview-hooks";
import type { CatalogueEntry } from "../../types/generated/CatalogueEntry";
import type { EchoParts } from "../../types/generated/EchoParts";
import type { Page } from "../../types/generated/Page";
import type { PageRequest } from "../../types/generated/PageRequest";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";
import type { WatchId } from "../../types/generated/WatchId";
import type { WatchInfo } from "../../types/generated/WatchInfo";
import type { WatchListWhich } from "../../types/generated/WatchListWhich";
import type { WatchSummaryInfo } from "../../types/generated/WatchSummaryInfo";
import type { WatchTarget } from "../../types/generated/WatchTarget";

export type { CatalogueEntry, EchoParts, WatchDraftInfo, WatchInfo, WatchSummaryInfo, WatchTarget };

/** A draft checked and in words. */
export interface DraftWords {
  echo: EchoParts;
  words: string;
  /** Why it can't be made (a code), else null. */
  problem: string | null;
}

/** The stand-in's state, for previews and tests. */
export interface WatchViewSeam {
  watches?: WatchInfo[];
  /** What each target kind (or `kind:id`) can be watched for. */
  catalogue?: Record<string, CatalogueEntry[]>;
  /** The targets' names for the stand-in echo, by `kind:id` or id. */
  names?: Record<string, string>;
  /** A draft's words, from the preview's stand-in echo. */
  draft?: (draft: WatchDraftInfo) => DraftWords;
  /** A made watch, from a draft (the stand-in's). */
  made?: (draft: WatchDraftInfo, id: string) => WatchInfo;
  calls?: [string, unknown][];
  /** Every read waits this long (ms), to see the skeleton. */
  slow?: number;
}

declare global {
  interface Window {
    __AIJ_WATCH_VIEW__?: WatchViewSeam;
  }
}

function seam(): WatchViewSeam {
  window.__AIJ_WATCH_VIEW__ ??= {};
  return window.__AIJ_WATCH_VIEW__;
}

/** The preview's stand-ins (the design preview only; never with a backend). */
function previewed(): ReturnType<typeof previewHooks>["watches"] | null {
  try {
    return previewHooks().watches;
  } catch {
    return null;
  }
}

function previewMade(): ((draft: WatchDraftInfo, id: string) => WatchInfo) | undefined {
  const hooks = previewed();
  return hooks ? (draft, id) => hooks.made(draft, id, { createdAgo: 0 }, seam().names) : undefined;
}

function noted(name: string, args: unknown): void {
  (seam().calls ??= []).push([name, args]);
}

async function later<T>(value: T): Promise<T> {
  const wait = seam().slow ?? 0;
  if (wait > 0) await new Promise((done) => setTimeout(done, wait));
  return value;
}

const ENDED_KEPT_MS = 30 * 24 * 3_600_000;

function heldBy(which: WatchListWhich, watch: WatchInfo, now: number): boolean {
  const kept = watch.endedMs === null || now - watch.endedMs <= ENDED_KEPT_MS;
  if (which === "live") return watch.state !== "ended";
  if (which === "ended") return watch.state === "ended" && kept;
  return kept;
}

/** One page of watches: `live` (watching and paused), `ended`, or all. */
export async function watchesPage(which: WatchListWhich, request: PageRequest): Promise<Page<WatchInfo>> {
  if (hasLiveBackend()) return call("watches_list", { request, which, lang: "en" });
  const now = Date.now();
  const all = (seam().watches ?? []).filter((watch) => heldBy(which, watch, now));
  const start = request.cursor ? Number(request.cursor) : 0;
  const limit = request.limit ?? 50;
  const items = all.slice(start, start + limit);
  const next = start + limit < all.length ? String(start + limit) : null;
  return later({ items, next_cursor: next, total: null });
}

function summarise(watches: WatchInfo[], now: number): WatchSummaryInfo {
  const midnight = new Date(now);
  midnight.setHours(0, 0, 0, 0);
  const out: WatchSummaryInfo = { watching: 0, paused: 0, ended: 0, toldToday: 0, neverTold: 0, evenInQuietHours: 0, limit: 50 };
  for (const watch of watches) {
    if (watch.state === "ended") {
      if (heldBy("ended", watch, now)) out.ended += 1;
      continue;
    }
    if (watch.state === "paused") out.paused += 1;
    else out.watching += 1;
    if (watch.watch.evenInQuietHours) out.evenInQuietHours += 1;
    if (watch.lastToldMs === null) out.neverTold += 1;
    else if (watch.lastToldMs >= midnight.getTime()) out.toldToday += 1;
  }
  return out;
}

/** The counts: watching, paused, ended, told today, never told, the limit. */
export async function watchesSummary(): Promise<WatchSummaryInfo> {
  if (hasLiveBackend()) return call("watches_summary");
  return later(summarise(seam().watches ?? [], Date.now()));
}

function catalogueKey(target: WatchTarget): string {
  return target.kind === "any" ? "any" : `${target.kind}:${target.id}`;
}

/** What `target` can be watched for: only what its Apps report. */
export async function watchCatalogue(target: WatchTarget): Promise<CatalogueEntry[]> {
  if (hasLiveBackend()) return call("watches_catalogue", { target, lang: "en" });
  const book = seam().catalogue ?? previewed()?.WATCHES_CATALOGUE ?? {};
  return later(book[catalogueKey(target)] ?? book[target.kind] ?? []);
}

/** A draft checked and in words; nothing is written. */
export async function watchDraftWords(draft: WatchDraftInfo): Promise<DraftWords> {
  if (hasLiveBackend()) {
    const preview = await call("watches_preview", { draft, lang: "en" });
    return { echo: preview.echo, words: preview.words, problem: preview.problem };
  }
  const hooks = previewed();
  const words = seam().draft?.(draft) ?? hooks?.draftWords(draft, seam().names);
  return words ?? { echo: { head: [{ text: "Tell you when it happens.", name: false }], lifetime: "Once.", quiet: null }, words: "Tell you when it happens. Once.", problem: null };
}

function replaced(made: WatchInfo): WatchInfo {
  const s = seam();
  const list = s.watches ?? [];
  const at = list.findIndex((w) => w.watch.id === made.watch.id);
  s.watches = at >= 0 ? list.map((w, i) => (i === at ? made : w)) : [made, ...list];
  return made;
}

function standIn(id: WatchId, change: (watch: WatchInfo) => WatchInfo): WatchInfo {
  const found = (seam().watches ?? []).find((w) => w.watch.id === id);
  if (!found) throw new Error("not_found");
  return replaced(change(found));
}

/** Makes the watch the user confirmed (their click). */
export async function watchCreate(draft: WatchDraftInfo): Promise<WatchInfo> {
  if (hasLiveBackend()) return call("watches_create", { draft, lang: "en" });
  noted("watches_create", draft);
  const id = `w-${(seam().watches?.length ?? 0) + 1}0000`;
  const made = (seam().made ?? previewMade())?.(draft, id);
  if (!made) throw new Error("unavailable");
  return replaced(made);
}

/** Changes a watch, by id. */
export async function watchUpdate(id: WatchId, draft: WatchDraftInfo): Promise<WatchInfo> {
  if (hasLiveBackend()) return call("watches_update", { id, draft, lang: "en" });
  noted("watches_update", { id, draft });
  const made = (seam().made ?? previewMade())?.(draft, id);
  if (!made) throw new Error("unavailable");
  return standIn(id, (was) => ({ ...made, told: was.told, lastToldMs: was.lastToldMs, lastToldCode: was.lastToldCode, lastToldWords: was.lastToldWords, watch: { ...made.watch, paused: was.watch.paused, createdMs: was.watch.createdMs }, state: was.state === "paused" ? "paused" : "active", version: was.version + 1 }));
}

/** Pauses or resumes a watch, by id. */
export async function watchPause(id: WatchId, paused: boolean): Promise<WatchInfo> {
  if (hasLiveBackend()) return call("watches_pause", { id, paused, lang: "en" });
  noted("watches_pause", { id, paused });
  return standIn(id, (was) => ({ ...was, state: paused ? "paused" : "active", watch: { ...was.watch, paused }, version: was.version + 1 }));
}

/** The removed ones the stand-in can bring back. */
const removed = new Map<string, { at: number; watch: WatchInfo }>();

/** Removes a watch, by id: it stops at once. */
export async function watchRemove(id: WatchId): Promise<WatchInfo> {
  if (hasLiveBackend()) return call("watches_remove", { id, lang: "en" });
  noted("watches_remove", { id });
  const list = seam().watches ?? [];
  const at = list.findIndex((w) => w.watch.id === id);
  if (at < 0) throw new Error("not_found");
  const gone = list[at]!;
  removed.set(id, { at, watch: gone });
  seam().watches = list.filter((_, i) => i !== at);
  return gone;
}

/** Brings back a watch just removed, by id (Undo). */
export async function watchRestore(id: WatchId): Promise<WatchInfo> {
  if (hasLiveBackend()) return call("watches_restore", { id, lang: "en" });
  noted("watches_restore", { id });
  const kept = removed.get(id);
  if (!kept) throw new Error("not_found");
  removed.delete(id);
  const list = [...(seam().watches ?? [])];
  list.splice(Math.min(kept.at, list.length), 0, kept.watch);
  seam().watches = list;
  return kept.watch;
}
