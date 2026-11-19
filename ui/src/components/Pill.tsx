/**
 * A small state word in a rounded outline (12 px, never smaller): a
 * thread's state (E18), a task's status (E19). One look for both.
 */
import type { JSX, ReactNode } from "react";

export type PillTone = "neutral" | "warning" | "danger" | "accent" | "live" | "success";

const TONES: Record<PillTone, string> = {
  neutral: "border-hairline bg-surface-2 text-muted",
  // Its tint on the plain surface, not on the row's: on a selected row
  // (surface-3) the light theme's amber fell under 4.5:1 (axe, a task in
  // review, 2026-10-27).
  warning: "border-warning/30 bg-surface [background-image:linear-gradient(var(--color-warning-soft),var(--color-warning-soft))] text-warning",
  danger: "border-danger/30 bg-danger-soft text-danger",
  accent: "border-accent/25 bg-accent-soft text-accent-hi",
  live: "border-live/25 bg-live-soft text-live",
  success: "border-success/25 bg-success-soft text-success",
};

/** `wrap`: a sentence, not a word — it may take two lines where its
 * column is narrow, never running past its card (a thread's "2 of 6 agent
 * messages in a row without you" did, beside a pool's canvas). */
export function Pill({ tone, children, testId, wrap = false }: { tone: PillTone; children: ReactNode; testId?: string; wrap?: boolean }): JSX.Element {
  const shape = wrap ? "min-w-0 max-w-full rounded-lg" : "shrink-0 whitespace-nowrap rounded-full";
  return (
    <span className={`inline-flex items-center ${shape} border px-2 py-0.5 text-label font-medium ${TONES[tone]}`} data-testid={testId}>
      {children}
    </span>
  );
}

/**
 * D-126 — the fixed state pills, the same on every screen (a pool's
 * header, a card, a row): what a state is, never a tone picked per call.
 * - `neutral`: a plain chip (a project's name: web);
 * - `progress`: the accent, for what is under way (Setting up · 5 of 6);
 * - `done`: a green dot (Done); `asks`: an amber dot, only when it waits
 *   for the user (Asks you); `error`: red with a stop mark (Stopped with
 *   an error). The marks differ in shape, never in colour alone.
 */
export type StateVariant = "neutral" | "progress" | "done" | "asks" | "error";

const STATES: Record<StateVariant, { box: string; mark: "dot" | "stop" | null }> = {
  neutral: { box: "bg-surface-3 px-2.5 text-ink-2", mark: null },
  progress: { box: "bg-accent-soft px-2.5 text-accent-hi", mark: null },
  done: { box: "px-0.5 text-success", mark: "dot" },
  asks: { box: "px-0.5 text-warning", mark: "dot" },
  error: { box: "px-0.5 text-danger", mark: "stop" },
};

export function StatePill({ variant, children, testId }: { variant: StateVariant; children: ReactNode; testId?: string }): JSX.Element {
  const look = STATES[variant];
  return (
    <span className={`inline-flex h-[22px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full text-label font-medium ${look.box}`} data-variant={variant} data-testid={testId}>
      {look.mark === "dot" && <span aria-hidden="true" className="size-1.5 shrink-0 rounded-full bg-current" />}
      {look.mark === "stop" && (
        <svg aria-hidden="true" width={12} height={12} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2.2} strokeLinecap="round" className="shrink-0">
          <circle cx="12" cy="12" r="8.5" />
          <path d="m6 18 12-12" />
        </svg>
      )}
      {children}
    </span>
  );
}
