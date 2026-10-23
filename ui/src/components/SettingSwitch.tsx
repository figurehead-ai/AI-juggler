/**
 * One switch in Settings, with one line of why (and a word on what just
 * happened, when there is one): Sessions' and Notifications' alike.
 */
import type { JSX } from "react";

export function SettingSwitch({
  id,
  label,
  hint,
  checked,
  nested = false,
  disabled = false,
  note,
  onChange,
}: {
  id: string;
  label: string;
  hint: string;
  checked: boolean;
  nested?: boolean;
  disabled?: boolean;
  note?: string | null;
  onChange: (on: boolean) => void;
}): JSX.Element {
  return (
    <div className={"flex items-start gap-2 text-xs" + (nested ? " ml-5" : "") + (disabled ? " opacity-60" : "")} data-testid={`pref-${id}`}>
      <input
        id={`pref-${id}-input`}
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
        aria-describedby={`pref-${id}-hint`}
        className="mt-0.5 size-3.5 accent-[var(--color-accent)]"
      />
      <span className="flex flex-col gap-0.5">
        <label htmlFor={`pref-${id}-input`} className="font-medium text-ink">
          {label}
        </label>
        <span id={`pref-${id}-hint`} className="text-[11px] leading-relaxed text-muted">
          {hint}
        </span>
        {note && (
          <span className="text-[11px] text-ink" role="status">
            {note}
          </span>
        )}
      </span>
    </div>
  );
}
