/**
 * AJ-E2110 — the parts every Settings pane is made of: the pane's title,
 * sections (a header, a rounded card, a one-line footer saying what it
 * does), rows (a title, a line of why, and a control on the right) and
 * the switch. One look for every pane.
 */
import { useEffect, useId, useRef, type JSX, type ReactNode } from "react";
import { InSettingsSection } from "../../components/CardTitle";
import { IconWarn } from "../../components/icons-menu";
import type { PaneTone } from "./panes";

/** How long a pane waits for its anchored section to appear (a section
 * drawn from data read on open, as Accounts' Tools on this Mac is). */
const ANCHOR_WAIT_MS = 2_000;

/** A pane: its title, then its sections. Opens on `anchor`'s section
 * when a link asked for one. */
export function Pane({
  id,
  title,
  anchor,
  children,
}: {
  id: string;
  title: string;
  anchor?: string;
  children: ReactNode;
}): JSX.Element {
  const pane = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!anchor) return;
    const find = () => document.getElementById(`settings-${anchor}`);
    let frame = 0;
    const jump = () => {
      find()?.scrollIntoView?.({ block: "start" });
      document.getElementById(`settings-${anchor}-title`)?.focus({ preventScroll: true });
    };
    const land = () => {
      jump();
      // Once more after the screen around it has set its own scroll.
      frame = requestAnimationFrame(jump);
    };
    if (find()) {
      land();
      return () => cancelAnimationFrame(frame);
    }
    // A section drawn once its data is read isn't there yet: jump when it
    // appears, not on whichever frame the read happened to beat (the
    // visual gate caught the pane opening at its top on some runs).
    const root = pane.current;
    if (!root || typeof MutationObserver === "undefined") return;
    const watch = new MutationObserver(() => {
      if (!find()) return;
      watch.disconnect();
      land();
    });
    watch.observe(root, { childList: true, subtree: true });
    const timer = window.setTimeout(() => watch.disconnect(), ANCHOR_WAIT_MS);
    return () => {
      watch.disconnect();
      window.clearTimeout(timer);
      cancelAnimationFrame(frame);
    };
  }, [anchor]);
  return (
    <div ref={pane} className="flex flex-col gap-7" data-testid={`settings-pane-${id}`} data-pane={id}>
      <h2 className="text-title text-ink">{title}</h2>
      {children}
    </div>
  );
}

/** One section: a 15 px header (the readability scale), its card (rows, or a settings card
 * that brings its own surface), and a muted one-line footer. */
export function PaneSection({
  id,
  title,
  footer,
  card = false,
  children,
}: {
  id: string;
  title: string;
  footer?: ReactNode;
  /** The child is a settings card with its own surface (and a title the
   * header already says); otherwise the children are rows. */
  card?: boolean;
  children: ReactNode;
}): JSX.Element {
  return (
    <section
      id={`settings-${id}`}
      aria-labelledby={`settings-${id}-title`}
      className="flex scroll-mt-4 flex-col gap-2"
      data-settings-section={id}
    >
      <h3 id={`settings-${id}-title`} tabIndex={-1} className="px-1 text-section font-semibold text-ink outline-none">
        {title}
      </h3>
      <InSettingsSection.Provider value>
        {card ? (
          <div className="flex flex-col gap-3">{children}</div>
        ) : (
          <ul className="flex flex-col divide-y divide-hairline rounded-card border border-hairline bg-surface shadow-card">{children}</ul>
        )}
      </InSettingsSection.Provider>
      {footer && <p className="px-1 text-xs leading-relaxed text-ink-2">{footer}</p>}
    </section>
  );
}

/** One row: what it is, one line of why, and its control. */
export function SettingRow({
  title,
  description,
  icon,
  control,
  testId,
  titleId,
  descriptionId,
  children,
}: {
  title: ReactNode;
  description?: ReactNode;
  icon?: ReactNode;
  control?: ReactNode;
  testId?: string;
  titleId?: string;
  descriptionId?: string;
  /** Anything under the row's words (a note, a status line). */
  children?: ReactNode;
}): JSX.Element {
  return (
    <li className="flex items-center gap-3 px-4 py-3" data-testid={testId}>
      {icon && <span className="flex size-7 shrink-0 items-center justify-center text-muted">{icon}</span>}
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span id={titleId} className="text-sm font-semibold text-ink">
          {title}
        </span>
        {description && (
          <span id={descriptionId} className="text-xs leading-relaxed text-muted">
            {description}
          </span>
        )}
        {children}
      </div>
      {control && <div className="flex shrink-0 items-center gap-2">{control}</div>}
    </li>
  );
}

/** A row with a switch, labelled by the row's title and described by
 * its line of why. */
export function SwitchRow({
  title,
  description,
  icon,
  checked,
  disabled = false,
  onChange,
  testId,
  note,
  noteTone = "info",
}: {
  title: string;
  description: ReactNode;
  icon?: ReactNode;
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  testId?: string;
  note?: string | null;
  /** How the note reads: a plain status line, or a warning. */
  noteTone?: RowNoteTone;
}): JSX.Element {
  const id = useId();
  return (
    <SettingRow
      title={title}
      titleId={`${id}-title`}
      description={description}
      descriptionId={`${id}-why`}
      icon={icon}
      testId={testId}
      control={
        <Switch checked={checked} disabled={disabled} onChange={onChange} labelledBy={`${id}-title`} describedBy={`${id}-why`} testId={testId ? `${testId}-switch` : undefined} />
      }
    >
      {note && <RowNote tone={noteTone}>{note}</RowNote>}
    </SettingRow>
  );
}

export type RowNoteTone = "info" | "warning";

/** A line under a row's words: what just happened (info), or something
 * to look at (warning: the app's warning icon and tone, as a notice's). */
export function RowNote({ tone = "info", children, testId }: { tone?: RowNoteTone; children: ReactNode; testId?: string }): JSX.Element {
  if (tone === "warning") {
    return (
      <span className="mt-1 flex items-start gap-1.5 text-xs leading-relaxed text-warning" role="status" data-tone="warning" data-testid={testId}>
        <IconWarn aria-hidden="true" width={14} height={14} className="mt-px shrink-0" />
        <span>{children}</span>
      </span>
    );
  }
  return (
    <span className="text-xs text-ink" role="status" data-testid={testId}>
      {children}
    </span>
  );
}

/** The switch: a real `role="switch"` button, always labelled. */
export function Switch({
  checked,
  disabled = false,
  onChange,
  labelledBy,
  describedBy,
  label,
  testId,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  labelledBy?: string;
  describedBy?: string;
  /** When nothing on screen names it. */
  label?: string;
  testId?: string;
}): JSX.Element {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={
        "relative h-5 w-9 shrink-0 rounded-full border transition-colors disabled:opacity-40 " +
        (checked ? "border-accent bg-accent" : "border-muted/60 bg-surface-3")
      }
      data-testid={testId}
    >
      <span
        aria-hidden="true"
        className={"absolute top-0.5 size-3.5 rounded-full bg-white shadow transition-all " + (checked ? "left-[18px]" : "left-0.5")}
      />
    </button>
  );
}

const TILE: Record<PaneTone, string> = {
  muted: "bg-muted",
  accent: "bg-accent",
  live: "bg-live",
  success: "bg-success",
  warning: "bg-warning",
  danger: "bg-danger",
  faint: "bg-faint",
};

/** A pane's 24 px tile: its colour, and a white stroke icon. */
export function PaneTile({ tone, children }: { tone: PaneTone; children: ReactNode }): JSX.Element {
  return (
    <span aria-hidden="true" className={`flex size-6 shrink-0 items-center justify-center rounded-md text-white ${TILE[tone]}`}>
      {children}
    </span>
  );
}
