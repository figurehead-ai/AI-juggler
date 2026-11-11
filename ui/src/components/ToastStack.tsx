/**
 * AJ-E2505 — the toasts as drawn (§2, the approved design of
 * 2026-10-27): bottom right, three at most, a time bar that stops while
 * hovered or focused, Undo or Retry when there is one, an error staying
 * until closed. The store and its rules are in Toast.tsx.
 */
import { keysOf } from "../lib/shortcuts";
import { useEffect, useRef, useState, useSyncExternalStore, type CSSProperties, type JSX, type RefObject } from "react";
import { dismissToast, pauseToast, resumeToast, toastStore, undoToast, type ToastEntry, type ToastOptions } from "./Toast";

// Read through the store at call time (Toast.tsx re-exports this file).
const subscribe = (listener: () => void) => toastStore.subscribe(listener);
const snapshotEntries = () => toastStore.entries();
const snapshotAnnouncement = () => toastStore.announcement();
const remove = (id: string) => toastStore.remove(id);

/** The one place toasts appear: the bottom right, three at most. Mounted once, by App. */
export function ToastHost(): JSX.Element {
  const rows = useSyncExternalStore(subscribe, snapshotEntries, snapshotEntries);
  const said = useSyncExternalStore(subscribe, snapshotAnnouncement, snapshotAnnouncement);
  return (
    <>
      {/* Always in the page, so a new message is read out (Req.5). */}
      <div role="status" aria-live="polite" aria-atomic="true" className="sr-only" data-testid="toast-announcer">
        {said}
      </div>
      {rows.length > 0 && (
        <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-[min(400px,calc(100vw-2rem))] flex-col items-stretch gap-2" data-testid="toast-stack">
          {rows.slice(-3).map((entry) => (
            <Toast key={entry.id} entry={entry} />
          ))}
        </div>
      )}
    </>
  );
}

function secondsUntil(at: number | null): number {
  return at === null ? 0 : Math.max(0, Math.ceil((at - Date.now()) / 1000));
}

const TONE_MARK: Record<NonNullable<ToastOptions["tone"]>, { glyph: string; className: string }> = {
  neutral: { glyph: "✓", className: "bg-accent-soft text-accent-hi" },
  success: { glyph: "✓", className: "bg-success-soft text-success" },
  warning: { glyph: "!", className: "bg-warning-soft text-warning" },
  error: { glyph: "!", className: "bg-danger-soft text-danger" },
};

const BUTTON = "shrink-0 rounded-lg border px-3 py-1.5 text-meta font-medium";

/** A toast's words. Lines break between its " · " parts, never inside a
 * name ("Claude / Code", the 0.4 review); a sentence after a name (":
 * archived — nothing deleted.") wraps as words do: kept on the name's
 * line it ran out of the toast's text (rc31). One part (a sentence)
 * wraps as words do: kept on one line it ran over Undo (the rc8 walk). */
function ToastWords({ message }: { message: string }): JSX.Element {
  const parts = message.split(" · ");
  return (
    <p className="text-pretty text-body font-medium text-ink">
      {parts.map((part, index) => {
        const at = part.indexOf(": ");
        const [name, sentence] = at < 0 ? [part, ""] : [part.slice(0, at + 1), part.slice(at + 1)];
        return (
          <span key={index}>
            {index > 0 && " · "}
            <span className={parts.length > 1 ? "whitespace-nowrap" : undefined}>{name}</span>
            {sentence}
          </span>
        );
      })}
    </p>
  );
}

/** Undo, with the seconds left. `below`: under the words. */
function UndoButton({ id, left, below, testId, buttonRef }: { id: string; left: number; below: boolean; testId: string | undefined; buttonRef: RefObject<HTMLButtonElement | null> }): JSX.Element {
  return (
    <button type="button" ref={buttonRef} onClick={() => void undoToast(id)} data-testid={testId ?? "toast-undo"} title={`Undo (${keysOf("undo")[0]})`} className={BUTTON + " border-accent/40 bg-accent-soft text-accent-hi hover:border-accent/60" + (below ? " mt-2 mr-2" : "")}>
      Undo ({left}s)
    </button>
  );
}

/** Its one other action: runs, then the toast goes. `below`: under the
 * words (a long label), else beside them. */
function ToastAction({ id, action, below, buttonRef }: { id: string; action: NonNullable<ToastOptions["action"]>; below: boolean; buttonRef: RefObject<HTMLButtonElement | null> }): JSX.Element {
  return (
    <button
      type="button"
      ref={buttonRef}
      onClick={() => {
        void remove(id);
        void action.run();
      }}
      data-testid="toast-action"
      // A long label ("Start another: web-storefront-checkout") is cut
      // with an ellipsis; its whole is the title and the accessible name.
      title={action.label}
      className={BUTTON + " max-w-full border-hairline text-ink hover:bg-surface-2" + (below ? " mt-2 self-start" : "")}
    >
      <span className="block truncate">{action.label}</span>
    </button>
  );
}

/** The time bar: it runs down over the toast's window and stops while
 * paused; with reduced motion it stands at the time left instead. */
function timeBar(entry: ToastEntry, paused: boolean): CSSProperties {
  const left = entry.pausedLeft ?? Math.max(0, (entry.expiresAt ?? 0) - Date.now());
  const reduce = typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduce) return { transform: `scaleX(${entry.total > 0 ? left / entry.total : 0})` };
  return { animation: `aij-toast-time ${entry.total}ms linear forwards`, animationDelay: `${left - entry.total}ms`, animationPlayState: paused ? "paused" : "running" };
}

/** One toast (the switch toast's look, AJ-E206, for every result). */
export function Toast({ entry }: { entry: ToastEntry }): JSX.Element {
  const { options, phase } = entry;
  const paused = entry.pausedLeft !== null;
  const [left, setLeft] = useState(() => secondsUntil(entry.expiresAt));
  useEffect(() => {
    setLeft(paused ? Math.ceil((entry.pausedLeft ?? 0) / 1000) : secondsUntil(entry.expiresAt));
    if (entry.expiresAt === null || paused) return;
    // display only: the countdown's seconds (the store owns the timing).
    const timer = window.setInterval(() => setLeft(secondsUntil(entry.expiresAt)), 1000);
    return () => window.clearInterval(timer);
  }, [entry.expiresAt, entry.pausedLeft, paused]);
  const undoRef = useRef<HTMLButtonElement | null>(null);
  const actionRef = useRef<HTMLButtonElement | null>(null);
  // Focus put on Undo for the user (a dialog just closed) is not their
  // pause: only a hover, or focus they moved there, holds the time.
  const placed = useRef(false);
  useEffect(() => {
    if (!options.focusWhenFree) return;
    // Free: on nothing, or resting on the page itself (a page opened
    // puts the keyboard on `<main>`, F11), never on a control.
    const active = document.activeElement;
    if (!active || active === document.body || active.tagName === "MAIN") {
      placed.current = true;
      // Undo, else its one action (Start another, D-96 F).
      (undoRef.current ?? actionRef.current)?.focus();
      placed.current = false;
    }
  }, [options.focusWhenFree]);
  // Under the words, a control never covers them (the rc31 walk: Undo
  // over "…archived — nothing deleted.")
  const below = options.actionBelow === true;
  // Undo, then its one action: beside the words, or under them.
  const controls = (
    <>
      {options.undo && <UndoButton id={entry.id} left={left} below={below} testId={options.testId} buttonRef={undoRef} />}
      {options.action && <ToastAction id={entry.id} action={options.action} below={below} buttonRef={actionRef} />}
    </>
  );
  const tone = phase === "failed" ? "error" : (options.tone ?? "neutral");
  const mark = TONE_MARK[tone];
  const hold = () => pauseToast(entry.id);
  const go = () => resumeToast(entry.id);
  return (
    <div
      className={"pointer-events-auto relative overflow-hidden rounded-xl border bg-surface shadow-card " + (tone === "error" ? "border-danger/50" : "border-hairline")}
      role="group"
      aria-label={options.message}
      data-testid={options.testId ? `${options.testId}-toast` : "toast"}
      data-phase={phase}
      data-tone={tone}
      onMouseEnter={hold}
      onMouseLeave={go}
      onFocus={() => !placed.current && hold()}
      onBlur={go}
    >
      <div className="flex items-start gap-3 px-4 py-3">
        <span aria-hidden="true" className={"mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full text-label font-bold " + mark.className}>
          {mark.glyph}
        </span>
        {phase === "counting" && (
          <>
            <div className="min-w-0 flex-1">
              <ToastWords message={options.message} />
              {options.detail && <p className="text-meta text-ink-2">{options.detail}</p>}
              {/* A long label goes under the words, never squeezing them
                  ("Start another: payments-api", D-96 F). */}
              {below && controls}
            </div>
            {!below && controls}
            <button type="button" onClick={() => dismissToast(entry.id)} aria-label={options.onExpire ? "Do it now" : "Close"} title={options.onExpire ? "Do it now" : "Close"} className="shrink-0 rounded-lg px-2 py-1 text-meta text-muted hover:bg-surface-2 hover:text-ink">
              ✕
            </button>
          </>
        )}
        {phase === "undoing" && (
          <p className="flex-1 text-body" data-testid={options.testId ? `${options.testId}ing` : "toast-undoing"}>
            {options.undo?.undoing ?? "Undoing…"}
          </p>
        )}
        {phase === "undone" && (
          <p className="flex-1 text-body" data-testid="toast-undone">
            {options.undo?.done ?? "Undone."}
          </p>
        )}
        {phase === "failed" && (
          <>
            <div className="flex min-w-0 flex-1 flex-col">
              <p className="text-body font-medium text-danger">Undo didn’t complete</p>
              <p className="text-meta text-ink-2">{entry.failure}</p>
            </div>
            <button type="button" onClick={() => void remove(entry.id)} className={BUTTON + " border-hairline hover:bg-surface-2"}>
              Close
            </button>
          </>
        )}
      </div>
      {phase === "counting" && entry.expiresAt !== null && (
        // The time left, as a bar (decoration: the countdown is in words on Undo).
        <span aria-hidden="true" data-testid="toast-time" className="absolute bottom-0 left-0 h-0.5 w-full origin-left bg-accent/70" style={timeBar(entry, paused)} />
      )}
    </div>
  );
}
