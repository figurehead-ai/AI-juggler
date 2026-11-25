import { useId, type JSX } from "react";
import { usageStatsOpenDocs, type UsageStatsMode } from "../lib/ipc/usage-stats";
import { ChoiceOption, type ControlIcon } from "./controls";
import { IconInactive } from "./icons-controls";
import { IconComputer } from "./icons-actions";
import { IconCloud } from "./icons-menu";

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
      "Once a week, send Figurehead counts of which features you used. No code, prompts, paths, project, app or account names, and no ID.",
  },
  {
    value: "local",
    title: "Keep stats on this computer only",
    consequence: "Count locally so you can see them. Nothing is sent.",
  },
  {
    value: "off",
    title: "Off",
    consequence: "Don't count anything.",
  },
];

/** AJ-E1558 (D-126): each choice's icon: sent, kept here, off. */
const USAGE_ICONS: Record<UsageStatsMode, ControlIcon> = { share: IconCloud, local: IconComputer, off: IconInactive };

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
      className="flex flex-col gap-2 rounded-xl bg-surface-2 px-3 py-3"
      data-testid={testId}
      data-chosen={value ?? "none"}
      disabled={disabled}
    >
      <legend className="float-left mb-1 w-full px-1 text-meta font-semibold text-ink">{legend}</legend>
      {hint && <p className="text-label leading-relaxed text-muted">{hint}</p>}
      {/* AJ-E1558 (D-126, PAT-SURF): each choice a card with its icon chip,
          the chosen one a fill; no outlined box around them. */}
      {USAGE_STATS_OPTIONS.map((option) => (
        <div key={option.value} data-testid="usage-stats-option" data-value={option.value} data-selected={value === option.value ? "true" : "false"}>
          <ChoiceOption
            type="radio"
            look="card"
            name={`${base}-usage-stats`}
            value={option.value}
            icon={USAGE_ICONS[option.value]}
            label={option.title}
            hint={option.consequence}
            checked={value === option.value}
            onChange={() => onChange(option.value)}
            className={disabled ? "cursor-default opacity-70" : "cursor-pointer"}
          />
        </div>
      ))}
      <button
        type="button"
        onClick={() => void usageStatsOpenDocs()}
        className="self-start text-label text-ink underline underline-offset-2 hover:text-accent-hi focus:outline-none focus:ring-2 focus:ring-accent"
        data-testid="usage-stats-docs"
      >
        See exactly what's collected
      </button>
    </fieldset>
  );
}
