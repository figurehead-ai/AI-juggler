import { useId } from "react";
import { usageStatsOpenDocs, type UsageStatsMode } from "../lib/bridge";

/**
 * AJ-E1105 (D-19): the usage-statistics consent. Three options with
 * equal weight — same size, same style, no default, none nudged — and
 * each says what it does in one sentence. `value === null` leaves every
 * radio unchecked; a caller that needs an answer blocks its own button
 * until one is picked. The titles and sentences are consent version 1:
 * change them only with a consent-version bump.
 */
export const USAGE_STATS_OPTIONS: { value: UsageStatsMode; title: string; consequence: string }[] = [
  {
    value: "share",
    title: "Share weekly usage stats",
    consequence:
      "Once a week, send Figurehead counts of which features you used. No code, prompts, paths, project or account names, and no ID.",
  },
  {
    value: "local",
    title: "Keep stats on this Mac only",
    consequence: "Count locally so you can see them. Nothing is sent.",
  },
  {
    value: "off",
    title: "Off",
    consequence: "Don't count anything.",
  },
];

export function UsageStatsChooser({
  value,
  onChange,
  disabled = false,
  legend = "Usage statistics",
  hint,
  testId = "usage-stats-chooser",
}: {
  value: UsageStatsMode | null;
  onChange: (mode: UsageStatsMode) => void;
  disabled?: boolean;
  legend?: string;
  hint?: string;
  testId?: string;
}): JSX.Element {
  const base = useId();
  return (
    <fieldset
      className="flex flex-col gap-2 rounded-xl border border-hairline px-4 py-3"
      data-testid={testId}
      data-chosen={value ?? "none"}
      disabled={disabled}
    >
      <legend className="px-1 text-xs font-medium text-ink">{legend}</legend>
      {hint && <p className="text-[11px] leading-relaxed text-muted">{hint}</p>}
      {USAGE_STATS_OPTIONS.map((option) => {
        const consequenceId = `${base}-${option.value}`;
        const checked = value === option.value;
        return (
          <label
            key={option.value}
            className={
              "flex cursor-default items-start gap-2.5 rounded-lg border px-3 py-2 text-xs transition-colors " +
              (checked ? "border-accent/40 bg-accent-soft" : "border-hairline hover:border-accent/30") +
              (disabled ? " opacity-70" : "")
            }
            data-testid="usage-stats-option"
            data-value={option.value}
            data-selected={checked ? "true" : "false"}
          >
            <input
              type="radio"
              name={`${base}-usage-stats`}
              value={option.value}
              checked={checked}
              onChange={() => onChange(option.value)}
              aria-describedby={consequenceId}
              className="mt-0.5 size-3.5 shrink-0 accent-[var(--color-accent)]"
            />
            <span className="flex flex-col gap-0.5">
              <span className="font-medium text-ink">{option.title}</span>
              <span id={consequenceId} className="text-[11px] leading-relaxed text-muted">
                {option.consequence}
              </span>
            </span>
          </label>
        );
      })}
      <button
        type="button"
        onClick={() => void usageStatsOpenDocs()}
        className="self-start text-[11px] text-ink underline underline-offset-2 hover:text-accent-hi focus:outline-none focus:ring-2 focus:ring-accent"
        data-testid="usage-stats-docs"
      >
        See exactly what's collected
      </button>
    </fieldset>
  );
}
