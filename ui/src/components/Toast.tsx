/**
 * AJ-E1503 — one feedback system; §2's Toast (AJ-E2505, the approved
 * design of 2026-10-27). Every action answers the same way: a toast at
 * the bottom right says the result in words, with Undo where the action
 * can be undone (Max-Depth C1: undo beats confirm) or Retry where it
 * failed. Six seconds, a bar shows the time left, hovering or focusing it
 * pauses it; an error stays until it is closed. The switch undo window
 * (AJ-E206) is one of these.
 *
 * - `showToast` shows a result; with `undo` it carries an Undo button and
 *   a countdown. Nothing is undone unless the user clicks Undo (or ⌘Z).
 * - `holdAction` holds an action for the undo window, then sends it: Undo
 *   in time means it is never sent (a held Stop, Req.2). Closing the toast
 *   sends it at once; quitting flushes what is held (`flushHeld`).
 * - The timing lives here, not in the component: a held action is sent
 *   even if the toast never renders.
 * - Toasts are announced through one polite live region that is always
 *   in the page, and never take focus from anything (Req.5). `focusWhenFree`
 *   only lands on Undo when focus would otherwise fall to the page body.
 */
import { errorText } from "../lib/bridge";

export interface ToastUndo {
  /** Puts things back. A rejection keeps the toast up, saying so. */
  onUndo: () => unknown;
  /** The undo window in seconds (default 5). */
  seconds?: number;
  /** Shown while undoing ("Undoing…"). */
  undoing?: string;
  /** Shown once undone ("Undone."). */
  done?: string;
  /** Explains a failed undo; the cause's words come first. */
  failed?: (cause: string) => string;
}

export interface ToastOptions {
  /** A second toast with the same id replaces the first. */
  id?: string;
  /** The result, in words. */
  message: string;
  /** A quieter second part. */
  detail?: string;
  /** `error` stays until it is closed. */
  tone?: "neutral" | "success" | "warning" | "error";
  undo?: ToastUndo;
  /** Its one other action (Retry, Sign in…): runs, then the toast goes. */
  action?: { label: string; run: () => unknown };
  /** How long a toast without Undo stays, in seconds (default 6, §2). */
  seconds?: number;
  /** The window closed without Undo: what a held action sends. */
  onExpire?: () => unknown;
  /** Words for a failed `onExpire`. */
  expireFailed?: (cause: string) => string;
  /** Called once the toast is gone, whichever way it went. */
  onClose?: () => void;
  /** Lands on Undo when nothing else holds focus (a dialog just closed). */
  focusWhenFree?: boolean;
  /** Test ids: `<testId>-toast` on the toast, `<testId>` on Undo. */
  testId?: string;
}

type Phase = "counting" | "undoing" | "undone" | "failed";

export interface ToastEntry {
  id: string;
  options: ToastOptions;
  phase: Phase;
  /** When the window closes (epoch ms); null once it no longer counts. */
  expiresAt: number | null;
  /** Its whole window, ms (the time bar's length). */
  total: number;
  /** Paused (hovered): the ms that were left. */
  pausedLeft: number | null;
  /** The failed undo's words. */
  failure: string | null;
}

/** A plain toast's time on screen, seconds (§2). */
export const TOAST_SECONDS = 6;

// --- the store ---------------------------------------------------------------

let entries: ToastEntry[] = [];
let announcement = "";
let counter = 0;
const timers = new Map<string, number>();
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function announce(text: string): void {
  // The same words twice in a row must still be read out.
  announcement = announcement === text ? `${text} ` : text;
}

function clearTimer(id: string): void {
  const timer = timers.get(id);
  if (timer !== undefined) window.clearTimeout(timer);
  timers.delete(id);
}

function arm(id: string, ms: number, then: () => void): void {
  clearTimer(id);
  // The captures' seam: a paused toast holds still to be photographed.
  if ((window as unknown as { __AIJ_TOAST_PAUSED__?: boolean }).__AIJ_TOAST_PAUSED__) return;
  timers.set(
    id,
    window.setTimeout(() => {
      timers.delete(id);
      then();
    }, ms),
  );
}

function update(id: string, change: Partial<ToastEntry>): void {
  entries = entries.map((entry) => (entry.id === id ? { ...entry, ...change } : entry));
  emit();
}

function remove(id: string): ToastEntry | undefined {
  const entry = entries.find((row) => row.id === id);
  clearTimer(id);
  entries = entries.filter((row) => row.id !== id);
  emit();
  entry?.options.onClose?.();
  return entry;
}

/** The window passed (or the toast was closed): send what was held. */
async function settle(id: string): Promise<void> {
  const entry = entries.find((row) => row.id === id);
  if (!entry) return;
  const counting = entry.phase === "counting";
  remove(id);
  if (!counting || !entry.options.onExpire) return;
  try {
    await entry.options.onExpire();
  } catch (cause) {
    const words = errorText(cause);
    showToast({ tone: "error", message: entry.options.expireFailed?.(words) ?? `That didn’t finish: ${words}` });
  }
}

/** Shows a toast; returns its id. */
export function showToast(options: ToastOptions): string {
  counter += 1;
  const id = options.id ?? `toast-${counter}`;
  const seconds = options.undo ? (options.undo.seconds ?? 5) : (options.seconds ?? TOAST_SECONDS);
  // An error stays until it is closed (unless it holds an action to send).
  const stays = options.tone === "error" && !options.onExpire && !options.undo;
  if (entries.some((row) => row.id === id)) {
    clearTimer(id);
    entries = entries.filter((row) => row.id !== id);
  }
  const total = seconds * 1000;
  entries = [...entries, { id, options, phase: "counting", expiresAt: stays ? null : Date.now() + total, total, pausedLeft: null, failure: null }];
  const words = [options.message, options.detail].filter(Boolean).join(" ");
  announce(options.undo ? `${words} Undo is available for ${seconds} seconds.` : words);
  if (!stays) arm(id, total, () => void settle(id));
  emit();
  return id;
}

/** Hovered or focused: the time stops. */
export function pauseToast(id: string): void {
  const entry = entries.find((row) => row.id === id);
  if (!entry || entry.phase !== "counting" || entry.expiresAt === null || entry.pausedLeft !== null) return;
  clearTimer(id);
  update(id, { pausedLeft: Math.max(0, entry.expiresAt - Date.now()) });
}

/** Left: the time goes on from where it stopped. */
export function resumeToast(id: string): void {
  const entry = entries.find((row) => row.id === id);
  if (!entry || entry.pausedLeft === null) return;
  const left = entry.pausedLeft;
  update(id, { pausedLeft: null, expiresAt: Date.now() + left });
  arm(id, left, () => void settle(id));
}

/** Closes a toast now; a held action is sent at once. */
export function dismissToast(id: string): void {
  void settle(id);
}

/** Undoes a counting toast (the Undo button, or ⌘Z for the newest). */
export async function undoToast(id: string): Promise<void> {
  const entry = entries.find((row) => row.id === id);
  if (!entry || entry.phase !== "counting" || !entry.options.undo) return;
  const undo = entry.options.undo;
  clearTimer(id);
  update(id, { phase: "undoing", expiresAt: null, pausedLeft: null });
  announce(undo.undoing ?? "Undoing…");
  emit();
  try {
    await undo.onUndo();
    update(id, { phase: "undone" });
    announce(undo.done ?? "Undone.");
    emit();
    arm(id, 3000, () => void remove(id));
  } catch (cause) {
    const words = errorText(cause);
    const failure = undo.failed?.(words) ?? words;
    update(id, { phase: "failed", failure });
    announce(`Undo didn’t complete. ${failure}`);
    emit();
  }
}

/** ⌘Z outside a text field: undoes the newest toast that still can be. */
export function undoLatest(): boolean {
  const latest = [...entries].reverse().find((row) => row.phase === "counting" && row.options.undo);
  if (!latest) return false;
  void undoToast(latest.id);
  return true;
}

/**
 * Holds `run` for the undo window, then sends it. Undo in time and it is
 * never sent. Holding the same id again while it waits changes nothing.
 */
export function holdAction(options: { id: string; message: string; detail?: string; seconds?: number; run: () => unknown; undone?: string; failed?: (cause: string) => string; testId?: string }): void {
  if (isHeld(options.id)) return;
  showToast({
    id: options.id,
    message: options.message,
    detail: options.detail,
    testId: options.testId,
    undo: { seconds: options.seconds ?? 5, onUndo: () => undefined, done: options.undone ?? "Undone — nothing was sent." },
    onExpire: options.run,
    expireFailed: options.failed,
  });
}

/** True while `id` is held and not yet sent. */
export function isHeld(id: string): boolean {
  return entries.some((row) => row.id === id && row.phase === "counting" && row.options.onExpire !== undefined);
}

/** Sends everything held now (quitting must not drop a Stop). */
export async function flushHeld(): Promise<void> {
  const held = entries.filter((row) => row.phase === "counting" && row.options.onExpire);
  await Promise.all(held.map((row) => settle(row.id)));
}

/** Test seam: forget every toast and timer. */
export function resetToasts(): void {
  for (const id of timers.keys()) clearTimer(id);
  entries = [];
  announcement = "";
  emit();
}

/** The current toasts (newest last). */
export function toastEntries(): ToastEntry[] {
  return entries;
}

/** Store internals for the view (ToastStack.tsx). */
export const toastStore = { subscribe, entries: () => entries, announcement: () => announcement, remove: (id: string) => void remove(id) };

// The view lives in ToastStack.tsx; App and tests take it from here too.
export { Toast, ToastHost } from "./ToastStack";
