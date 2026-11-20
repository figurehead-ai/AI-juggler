/**
 * 0.4.6 (D-126) — the one Alert, at three levels, so a colour always
 * means the same thing:
 *
 * - `info`: a fact ("Each app has its own folder"). Neutral, never
 *   amber; said politely (`status`).
 * - `needs-you`: only when the user must act ("Work Max needs its
 *   sign-in"). Amber edge and bell; said politely (`status`).
 * - `broken`: something failed ("The pool's bridge stopped"). Red tint,
 *   edge and ✕; said at once (`alert`).
 *
 * The icons differ in shape, so the level never rests on colour alone.
 * Its title leads in semibold ink and the words follow on the same line
 * (`stack` puts the title on a line of its own, as a window's banner
 * does); actions sit at its end, and a close when it can be closed.
 * `Banner` draws its tones through these levels.
 */
import type { HTMLAttributes, JSX, ReactNode, SVGProps } from "react";
import { IconBell, IconInfo } from "./icons-actions";
import { IconClose } from "./icons-controls";

export type AlertLevel = "info" | "needs-you" | "broken";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

/** Each level's box, icon and live-region role. */
export const ALERT_LEVELS: Record<AlertLevel, { box: string; icon: string; Icon: Icon; role: "status" | "alert" }> = {
  info: { box: "bg-surface-2", icon: "text-faint", Icon: IconInfo, role: "status" },
  "needs-you": { box: "bg-surface-2 shadow-[inset_3px_0_0_var(--color-warning)]", icon: "text-warning", Icon: IconBell, role: "status" },
  broken: {
    box: "bg-[color-mix(in_srgb,var(--color-danger)_10%,var(--color-surface))] shadow-[inset_3px_0_0_var(--color-danger)]",
    icon: "text-danger",
    Icon: IconClose,
    role: "alert",
  },
};

export function Alert({
  level,
  title,
  children,
  actions,
  below,
  onDismiss,
  dismissLabel,
  live,
  stack = false,
  testId,
  className = "",
  ...rest
}: {
  level: AlertLevel;
  /** What it is about, in semibold ink. */
  title?: ReactNode;
  /** The rest, after the title (an inline link may end it). */
  children?: ReactNode;
  /** Its buttons, at its end. */
  actions?: ReactNode;
  /** Under the words: a meter, an error from its action. */
  below?: ReactNode;
  /** Shown as × when given. */
  onDismiss?: () => void;
  /** The close's name for screen readers. */
  dismissLabel?: string;
  /** Its live region: the level's own by default; `null` for none, when
   * the region it sits in already speaks (the notification band). */
  live?: "status" | "alert" | null;
  /** The title on a line of its own (a window's banner). */
  stack?: boolean;
  testId?: string;
  className?: string;
} & Omit<HTMLAttributes<HTMLDivElement>, "role" | "title" | "children" | "className">): JSX.Element {
  const look = ALERT_LEVELS[level];
  const spoken = live === undefined ? look.role : (live ?? undefined);
  return (
    <div {...rest} role={spoken} data-level={level} data-testid={testId} className={`flex flex-wrap items-start gap-x-2.5 gap-y-2 rounded-xl px-3.5 py-3 text-meta text-ink-2 ${look.box} ${className}`}>
      <look.Icon aria-hidden="true" width={16} height={16} className={`mt-px shrink-0 ${look.icon}`} />
      <div className="min-w-[12rem] flex-1 text-pretty">
        {stack ? (
          <>
            {title && <p className="text-body font-semibold text-ink">{title}</p>}
            {children && <div>{children}</div>}
          </>
        ) : (
          <div>
            {title && <span className="font-semibold text-ink">{title}</span>}
            {title && children ? " " : null}
            {children}
          </div>
        )}
        {below}
      </div>
      {(actions || onDismiss) && (
        <div className="ml-auto flex flex-wrap items-center gap-2 self-center">
          {actions}
          {onDismiss && (
            <button type="button" onClick={onDismiss} aria-label={dismissLabel ?? "Close"} title="Close" className="rounded-lg px-2 py-1 text-section leading-none text-muted hover:bg-surface-3 hover:text-ink">
              ×
            </button>
          )}
        </div>
      )}
    </div>
  );
}
