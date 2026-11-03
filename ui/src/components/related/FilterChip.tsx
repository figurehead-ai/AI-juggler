/**
 * One filter, as a chip: its name and a choice of All or one value (the
 * shared Picker, so past 8 values it has a search field, and values that
 * would show nothing go last). History's and every related list's
 * (E31, AJ-E3103). Lit while a value is chosen. Hidden while it has fewer
 * than two values to choose from and none chosen: a filter that can't
 * narrow anything isn't offered.
 */
import type { JSX } from "react";
import { Picker } from "../controls";

export interface ChipOption {
  id: string;
  label: string;
  /** How many rows it shows, when known ("· 12" in the menu). */
  count?: number;
}

export function FilterChip({
  label,
  value,
  options,
  onChange,
  testId,
  all = "All",
}: {
  label: string;
  value: string | null;
  options: ChipOption[];
  onChange: (next: string | null) => void;
  testId: string;
  /** The words for no filter ("All", "Any"). */
  all?: string;
}): JSX.Element | null {
  if (options.length < 2 && value === null) return null;
  // AJ-E3105: the values with the most rows first, then All.
  const most = options
    .filter((option) => (option.count ?? 0) > 0)
    .sort((a, b) => (b.count ?? 0) - (a.count ?? 0))
    .slice(0, 5)
    .map((option) => option.id);
  return (
    <span
      className={
        "inline-flex h-8 items-center overflow-hidden rounded-full border px-0.5 text-meta transition-colors " +
        (value !== null ? "border-accent/45 bg-accent-soft text-ink" : "border-hairline bg-surface text-muted hover:text-ink")
      }
    >
      <Picker
        look="quiet"
        size="sm"
        // Its name as it reads on screen ("App: All"), so the button's
        // name contains what it shows (axe: label-content-name-mismatch).
        prefix={`${label}:`}
        ariaLabel={label}
        heading={label}
        value={value ?? ""}
        onChange={(next) => onChange(next || null)}
        groups={most.length > 0 ? [{ label: "Most here", values: most }] : []}
        options={[{ value: "", label: all, pinned: true }, ...options.map((option) => ({ value: option.id, label: option.count === undefined ? option.label : `${option.label} · ${option.count}`, empty: option.count === 0 }))]}
        testId={testId}
        className="max-w-60 rounded-full text-meta"
      />
    </span>
  );
}
