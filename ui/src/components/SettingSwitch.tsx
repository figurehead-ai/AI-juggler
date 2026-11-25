/**
 * One switch in Settings, with one line of why (and a word on what just
 * happened, when there is one): Sessions' and Notifications' alike.
 *
 * AJ-E1558 (D-126 rich mode, TGL-1, TGL-2): the kit's row — its icon chip
 * (the accent's soft step while on), the title in ink, one line of why,
 * and a switch on the right (it takes effect at once). The whole row is
 * the switch's label.
 */
import type { JSX } from "react";
import { RowLead, SwitchBox, type RowIcon } from "./controls";

export function SettingSwitch({
  id,
  icon,
  label,
  hint,
  checked,
  nested = false,
  disabled = false,
  note,
  testId,
  onChange,
}: {
  id: string;
  /** The row's icon (every making row has one, D-126). */
  icon: RowIcon;
  label: string;
  hint: string;
  checked: boolean;
  nested?: boolean;
  disabled?: boolean;
  note?: string | null;
  /** The switch's own test id (the row is `pref-<id>`). */
  testId?: string;
  onChange: (on: boolean) => void;
}): JSX.Element {
  return (
    <div
      className={"flex items-center gap-3 rounded-[10px] px-3 py-[9px] " + (nested ? "ml-10 " : "") + (disabled ? "opacity-60" : "hover:bg-surface-2")}
      data-testid={`pref-${id}`}
      data-checked={checked}
    >
      <RowLead icon={icon} on={checked && !disabled} />
      <span className="flex min-w-0 flex-1 flex-col">
        <label htmlFor={`pref-${id}-input`} className="text-body font-medium text-ink">
          {label}
        </label>
        <span id={`pref-${id}-hint`} className="text-meta text-muted">
          {hint}
        </span>
        {note && (
          <span className="text-label text-ink-2" role="status">
            {note}
          </span>
        )}
      </span>
      <SwitchBox
        id={`pref-${id}-input`}
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
        aria-describedby={`pref-${id}-hint`}
        data-testid={testId}
      />
    </div>
  );
}
