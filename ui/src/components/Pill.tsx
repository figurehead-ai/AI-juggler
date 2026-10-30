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
