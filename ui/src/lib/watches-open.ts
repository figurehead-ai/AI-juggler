/**
 * AJ-E3816 — the doors into the Watches page: what it should open on, by
 * the watch's or the target's id (actions target by id). A session row's
 * Watch…, a page's chip, a watch alert's Edit watch, Bot's Change… and
 * the notification centre's link each call `openWatches`; the page takes
 * the intent when it opens (or at once, when it is open already).
 */
import type { WatchDraftInfo } from "../types/generated/WatchDraftInfo";
import type { WatchTarget } from "../types/generated/WatchTarget";

export type WatchesIntent =
  /** The form, new, filled in (a target picked, or a whole draft). */
  | { kind: "new"; draft?: WatchDraftInfo; target?: WatchTarget }
  /** The form, editing one watch. */
  | { kind: "edit"; id: string }
  /** The list, showing only what watches one target. */
  | { kind: "filter"; target: WatchTarget; name: string }
  /** The list. */
  | { kind: "list" };

let pending: WatchesIntent | null = null;
const listeners = new Set<() => void>();

/** Opens the Watches page on `intent`. */
export function openWatches(intent: WatchesIntent = { kind: "list" }): void {
  pending = intent;
  for (const listener of listeners) listener();
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "watches" } }));
}

/** The intent waiting for the page, taken once. */
export function takeWatchesIntent(): WatchesIntent | null {
  const intent = pending;
  pending = null;
  return intent;
}

/** Calls `listener` when a door opens the page (it may be open already). */
export function onWatchesIntent(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The watch id a watch alert names: its key is `watch:<id>:…`. */
export function watchIdOfNotice(notice: { kind: string; subject: string }): string | null {
  if (notice.kind !== "watch") return null;
  const id = notice.subject.split(":")[1] ?? "";
  return /^w-[0-9a-f]{1,40}$/.test(id) ? id : null;
}
