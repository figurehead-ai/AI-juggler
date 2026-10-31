/**
 * §2's Banner (AJ-E2504, the approved design of 2026-10-27): one shared
 * shape for everything the window says above a screen — an account almost
 * out of room, AI Juggler's own service not answering, an update, a
 * parked session's reset, another computer's alert. Who, what and when in
 * plain words, a meter when there is one, its real actions, and a close.
 * Screens never hand-make their own.
 *
 * `role`: `status` (polite) for news, `alert` for what stops work (the
 * service not answering). The close says what it closes.
 */
import type { JSX, ReactNode } from "react";
import { Button } from "./primitives";

export type BannerTone = "accent" | "warning" | "danger" | "neutral";

export interface BannerAction {
  label: string;
  onClick: () => void;
  /** The one primary (filled). */
  primary?: boolean;
  /** A quiet one (no border). */
  quiet?: boolean;
  disabled?: boolean;
  testId?: string;
}

const TONES: Record<BannerTone, { box: string; mark: string; meter: string }> = {
  accent: { box: "border-accent/40 bg-gradient-to-r from-accent-soft to-transparent", mark: "bg-accent-soft text-accent-hi", meter: "bg-accent" },
  warning: { box: "border-warning/40 bg-gradient-to-r from-warning-soft to-transparent", mark: "bg-warning-soft text-warning", meter: "bg-warning" },
  danger: { box: "border-danger/40 bg-gradient-to-r from-danger-soft to-transparent", mark: "bg-danger-soft text-danger", meter: "bg-danger" },
  neutral: { box: "border-hairline bg-surface/60", mark: "bg-surface-3 text-muted", meter: "bg-accent" },
};

function Mark({ tone }: { tone: BannerTone }): JSX.Element {
  return (
    <span aria-hidden="true" className={"flex size-7 shrink-0 items-center justify-center rounded-lg " + TONES[tone].mark}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        {tone === "warning" ? (
          <>
            <path d="M12 9v4M12 17h.01" />
            <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
          </>
        ) : tone === "danger" ? (
          <>
            <circle cx="12" cy="12" r="9" />
            <path d="M12 8v5M12 16h.01" />
          </>
        ) : (
          <>
            <circle cx="12" cy="12" r="9" />
            <path d="M12 11v5M12 8h.01" />
          </>
        )}
      </svg>
    </span>
  );
}

export function Banner({
  tone = "warning",
  title,
  detail,
  meter,
  actions = [],
  onDismiss,
  dismissLabel,
  role,
  aside,
  testId,
  children,
}: {
  tone?: BannerTone;
  /** Who and what, in plain words. */
  title: string;
  /** When, and the rest, quieter. */
  detail?: ReactNode;
  /** A meter: how much is used, in percent (the bar fills that far). */
  meter?: { used: number; label: string };
  actions?: BannerAction[];
  /** Shown as × when given. */
  onDismiss?: () => void;
  /** The close's name for screen readers ("Close the banner about Mia"). */
  dismissLabel?: string;
  /** Its own live region: `alert` for what stops work. None when the
   * region it sits in already speaks (the notification band). */
  role?: "status" | "alert";
  /** Beside the title (a channel chip). */
  aside?: ReactNode;
  testId?: string;
  /** Under the lines (an error from its action). */
  children?: ReactNode;
}): JSX.Element {
  const look = TONES[tone];
  return (
    <div role={role} data-testid={testId} data-tone={tone} className={"flex flex-wrap items-center gap-3 rounded-xl border px-3.5 py-2 " + look.box}>
      <Mark tone={tone} />
      <div className="min-w-[14rem] flex-1">
        <p className="flex flex-wrap items-center gap-2 text-body font-semibold text-ink">
          {title}
          {aside}
        </p>
        {detail && <p className="text-meta text-ink-2">{detail}</p>}
        {meter && (
          <span
            role="meter"
            aria-label={meter.label}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(meter.used)}
            className="mt-2 block h-1.5 w-56 max-w-full overflow-hidden rounded-full bg-surface-3"
          >
            <span className={"block h-full rounded-full " + look.meter} style={{ width: `${Math.min(100, Math.max(0, meter.used))}%` }} />
          </span>
        )}
        {children}
      </div>
      {(actions.length > 0 || onDismiss) && (
        <div className="ml-auto flex flex-wrap items-center gap-2">
          {actions.map((action) => (
            <Button key={action.label} size="sm" variant={action.primary ? "primary" : action.quiet ? "ghost" : "secondary"} onClick={action.onClick} disabled={action.disabled} data-testid={action.testId}>
              {action.label}
            </Button>
          ))}
          {onDismiss && (
            <button type="button" onClick={onDismiss} aria-label={dismissLabel ?? `Close: ${title}`} title="Close" className="rounded-lg px-2 py-1 text-section leading-none text-muted hover:bg-surface-2 hover:text-ink">
              ×
            </button>
          )}
        </div>
      )}
    </div>
  );
}
