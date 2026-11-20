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
 *
 * D-126: drawn as the one `Alert`, each tone at its level (BANNER_LEVEL),
 * the title on a line of its own.
 */
import type { JSX, ReactNode } from "react";
import { Alert, type AlertLevel } from "./Alert";
import { WhyButton } from "./WhyTip";

export type BannerTone = "accent" | "warning" | "danger" | "neutral";

export interface BannerAction {
  label: string;
  onClick: () => void;
  /** The one primary (filled). */
  primary?: boolean;
  /** A quiet one (no border). */
  quiet?: boolean;
  disabled?: boolean;
  /** Why it can't act now: greyed, saying so on hover and focus
   * (`WhyButton`), as the door it opens would. */
  why?: string | null;
  testId?: string;
}

/** D-126: each tone is one of the Alert's three levels. A fact (an
 * update, a reminder) is info, never amber; amber only when the user must
 * act; red when something broke. */
export const BANNER_LEVEL: Record<BannerTone, AlertLevel> = {
  accent: "info",
  neutral: "info",
  warning: "needs-you",
  danger: "broken",
};

const METER: Record<BannerTone, string> = { accent: "bg-accent", neutral: "bg-accent", warning: "bg-warning", danger: "bg-danger" };

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
  /** The close's name for screen readers ("Close the banner about West"). */
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
  return (
    <Alert
      level={BANNER_LEVEL[tone]}
      stack
      live={role ?? null}
      testId={testId}
      data-tone={tone}
      title={
        <span className="flex flex-wrap items-center gap-2">
          {title}
          {aside}
        </span>
      }
      below={
        <>
          {meter && (
            <span
              role="meter"
              aria-label={meter.label}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(meter.used)}
              className="mt-2 block h-1.5 w-56 max-w-full overflow-hidden rounded-full bg-surface-3"
            >
              <span className={"block h-full rounded-full " + METER[tone]} style={{ width: `${Math.min(100, Math.max(0, meter.used))}%` }} />
            </span>
          )}
          {children}
        </>
      }
      actions={
        actions.length > 0
          ? actions.map((action) => (
              <WhyButton key={action.label} size="sm" variant={action.primary ? "primary" : action.quiet ? "ghost" : "secondary"} why={action.why ?? null} onClick={action.onClick} disabled={action.disabled} data-testid={action.testId}>
                {action.label}
              </WhyButton>
            ))
          : undefined
      }
      onDismiss={onDismiss}
      dismissLabel={dismissLabel ?? `Close: ${title}`}
    >
      {detail}
    </Alert>
  );
}
