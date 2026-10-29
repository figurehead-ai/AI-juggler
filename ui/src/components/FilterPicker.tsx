/**
 * 0.3 — a filter as a picker: its name and value on a chip ("Project:
 * all ▾") that opens a row menu of the choices, each with an icon, a hint
 * under it where it has one, and ✓ on the one chosen. No system select
 * (the design rule). Built on RowMenu's MenuItem, the same as the order
 * menu, so the shared Picker (branch controls-0.3) can replace it in one
 * place.
 */
import { useState, type JSX, type ReactNode } from "react";
import { MenuItem, MenuSeparator, RowMenu } from "./RowMenu";

export type FilterOption = { id: string; label: string; hint?: string; icon?: ReactNode };

export function FilterPicker({
  label,
  value,
  all,
  options,
  onChange,
  testId,
}: {
  label: string;
  /** The option chosen; `null` is "all". */
  value: string | null;
  /** The words for no filter ("All", "Anyone"). */
  all: FilterOption;
  options: FilterOption[];
  onChange: (next: string | null) => void;
  testId: string;
}): JSX.Element {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const chosen = options.find((option) => option.id === value);
  const shown = chosen?.label ?? all.label;
  const pick = (next: string | null) => {
    setAt(null);
    onChange(next);
  };
  const line = (option: FilterOption, on: boolean, id: string | null) => (
    <MenuItem key={option.id} checked={on} testId={`${testId}-${id ?? "all"}`} onClick={() => pick(id)}>
      <span className="flex items-start gap-2">
        <span aria-hidden="true" className="mt-0.5 flex size-3.5 shrink-0 items-center justify-center text-muted">
          {option.icon}
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className={"truncate " + (on ? "font-semibold" : "")}>{option.label}</span>
          {option.hint && <span className="truncate text-label leading-snug text-muted">{option.hint}</span>}
        </span>
        <span aria-hidden="true" className={"mt-0.5 shrink-0 text-accent-hi " + (on ? "" : "invisible")}>
          ✓
        </span>
      </span>
    </MenuItem>
  );
  return (
    <>
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={at !== null}
        aria-label={`${label}: ${shown}`}
        data-testid={testId}
        // Under its chip, never over it (the sweep: a menu covering its
        // own button).
        onClick={(event) => {
          const box = event.currentTarget.getBoundingClientRect();
          setAt({ x: box.left, y: box.bottom + 4 });
        }}
        className={
          "inline-flex h-8 max-w-64 items-center gap-1.5 rounded-full border px-3 text-meta outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent/60 " +
          (value !== null ? "border-accent/45 bg-accent-soft text-ink" : "border-hairline bg-surface text-muted hover:text-ink")
        }
      >
        <span className="shrink-0">{label}:</span>
        <span className="truncate font-medium text-ink">{shown}</span>
        <svg aria-hidden="true" width="8" height="8" viewBox="0 0 10 10" className="shrink-0">
          <path d="M2 3.5 H8 L5 7 Z" fill="currentColor" />
        </svg>
      </button>
      {at && (
        <RowMenu heading={label} ariaLabel={label} at={at} onClose={() => setAt(null)} wide>
          {line(all, value === null, null)}
          {options.length > 0 && <MenuSeparator />}
          <div className="flex max-h-72 flex-col gap-0.5 overflow-y-auto">{options.map((option) => line(option, option.id === value, option.id))}</div>
        </RowMenu>
      )}
    </>
  );
}
