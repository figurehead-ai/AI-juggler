/**
 * AJ-E2110 — the parts every Settings pane is made of: the pane's title,
 * sections (a header, a rounded card, a one-line footer saying what it
 * does), rows (a title, a line of why, and a control on the right) and
 * the switch. One look for every pane.
 */
import { useEffect, useId, type JSX, type ReactNode } from "react";
import { InSettingsSection } from "../../components/CardTitle";
import type { PaneTone } from "./panes";

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
  useEffect(() => {
    if (!anchor) return;
    const jump = () => {
      document.getElementById(`settings-${anchor}`)?.scrollIntoView?.({ block: "start" });
      document.getElementById(`settings-${anchor}-title`)?.focus({ preventScroll: true });
    };
    jump();
    // Once more after the screen around it has set its own scroll.
    const frame = requestAnimationFrame(jump);
    return () => cancelAnimationFrame(frame);
  }, [anchor]);
  return (
    <div className="flex flex-col gap-7" data-testid={`settings-pane-${id}`} data-pane={id}>
      <h2 className="text-2xl font-bold tracking-tight text-ink">{title}</h2>
      {children}
    </div>
  );
}

/** One section: a 13 px header, its card (rows, or a settings card
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
      <h3 id={`settings-${id}-title`} tabIndex={-1} className="px-1 text-[13px] font-bold text-ink outline-none">
        {title}
      </h3>
      <InSettingsSection.Provider value>
        {card ? (
          <div className="flex flex-col gap-3">{children}</div>
        ) : (
          <ul className="flex flex-col divide-y divide-hairline rounded-card border border-hairline bg-surface shadow-card">{children}</ul>
        )}
      </InSettingsSection.Provider>
      {footer && <p className="px-1 text-xs leading-relaxed text-muted">{footer}</p>}
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
}: {
  title: string;
  description: ReactNode;
  icon?: ReactNode;
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  testId?: string;
  note?: string | null;
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
      {note && (
        <span className="text-xs text-ink" role="status">
          {note}
        </span>
      )}
    </SettingRow>
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
        (checked ? "border-accent bg-accent" : "border-hairline bg-surface-3")
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
