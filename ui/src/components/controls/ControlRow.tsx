/**
 * 0.4.6 (D-126) — the making screens' rows: an icon chip, a title, one
 * line of help, and the control on the right. `ToggleRow` is a switch
 * (on or off, at once: "Hold notifications at night"); `CheckRow` a
 * checkbox (one of a set to take along: "Copy settings from Work Max").
 * The whole row is the control's label, so a click anywhere on it flips
 * it; Space and Enter do too. The chip takes the accent while it is on.
 * `RowList` holds a few of them in one card.
 */
import { useId, type JSX, type KeyboardEvent, type ReactNode } from "react";
import type { ControlIcon } from "./ChoiceLine";
import { Checkbox } from "./Toggle";

interface RowProps {
  /** The chip's icon. */
  icon?: ControlIcon;
  title: ReactNode;
  /** One line on what it does. */
  help?: ReactNode;
  checked: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
  /** Why it can't be changed now (said with it, and as its tooltip). */
  reason?: string;
  testId?: string;
}

/** The chip, the words and the control, the row its label. */
function Row({ id, icon: Icon, title, help, checked, disabled, titleId, helpId, children }: Omit<RowProps, "onChange" | "testId"> & { id: string; titleId: string; helpId: string; children: ReactNode }): JSX.Element {
  return (
    <label
      htmlFor={id}
      className={"flex min-w-0 items-center gap-3 rounded-[10px] px-3 py-[9px] " + (disabled ? "opacity-60" : "cursor-pointer hover:bg-surface-2")}
      data-checked={checked}
    >
      {Icon && (
        <span aria-hidden="true" className={"flex size-[30px] shrink-0 items-center justify-center rounded-lg transition-colors " + (checked ? "bg-accent-soft text-accent-hi" : "bg-surface-3 text-ink-2")}>
          <Icon width={16} height={16} />
        </span>
      )}
      <span className="flex min-w-0 flex-1 flex-col">
        <span id={titleId} className="text-body font-medium text-ink">
          {title}
        </span>
        {help && (
          <span id={helpId} className="text-meta text-muted">
            {help}
          </span>
        )}
      </span>
      {children}
    </label>
  );
}

/** A switch row: on or off, at once. */
export function ToggleRow(props: RowProps): JSX.Element {
  const { checked, onChange, disabled = false, reason, help, testId } = props;
  const base = useId().replace(/:/g, "");
  const ids = { id: `${base}-switch`, titleId: `${base}-title`, helpId: `${base}-help`, whyId: `${base}-why` };
  const described = [help ? ids.helpId : null, disabled && reason ? ids.whyId : null].filter(Boolean).join(" ") || undefined;
  return (
    <Row {...props} id={ids.id} titleId={ids.titleId} helpId={ids.helpId} disabled={disabled}>
      <button
        id={ids.id}
        type="button"
        role="switch"
        aria-checked={checked}
        aria-labelledby={ids.titleId}
        aria-describedby={described}
        disabled={disabled}
        title={disabled ? reason : undefined}
        onClick={() => onChange(!checked)}
        data-testid={testId}
        className={"relative h-5 w-[34px] shrink-0 rounded-full transition-colors " + (checked ? "bg-accent-fill" : "bg-surface-3 inset-ring-1 inset-ring-hairline")}
      >
        <span aria-hidden="true" className={"absolute top-[3px] size-3.5 rounded-full transition-[left] " + (checked ? "left-[17px] bg-white" : "left-[3px] bg-faint")} />
      </button>
      {disabled && reason && (
        <span id={ids.whyId} className="sr-only">
          {reason}
        </span>
      )}
    </Row>
  );
}

/** A checkbox row: one of a set to take along. Enter ticks it too. */
export function CheckRow(props: RowProps): JSX.Element {
  const { checked, onChange, disabled = false, reason, help, testId } = props;
  const base = useId().replace(/:/g, "");
  const ids = { id: `${base}-check`, titleId: `${base}-title`, helpId: `${base}-help` };
  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== "Enter" || disabled) return;
    event.preventDefault();
    onChange(!checked);
  };
  return (
    <Row {...props} id={ids.id} titleId={ids.titleId} helpId={ids.helpId} disabled={disabled}>
      <Checkbox
        id={ids.id}
        checked={checked}
        disabled={disabled}
        reason={reason}
        aria-labelledby={ids.titleId}
        aria-describedby={help ? ids.helpId : undefined}
        onChange={(event) => onChange(event.target.checked)}
        onKeyDown={onKey}
        data-testid={testId}
      />
    </Row>
  );
}

/** A few rows in one card. */
export function RowList({ label, children, testId }: { label?: string; children: ReactNode; testId?: string }): JSX.Element {
  return (
    <div role={label ? "group" : undefined} aria-label={label} className="flex min-w-0 flex-col gap-0.5 rounded-card bg-surface p-1.5 inset-ring-1 inset-ring-hairline/75" data-testid={testId}>
      {children}
    </div>
  );
}
