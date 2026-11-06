/**
 * AJ-E3007 step 1: pick a template. Each card shows its roles and talk as
 * chips, so the choice is visible before going on (wireframe 1).
 */
import type { JSX } from "react";

import { Chip, type ChipTone } from "../../components/primitives";
import { POOL_TEMPLATES, type TemplateChip } from "../../lib/pool-templates";

const TONE: Record<TemplateChip["tone"], ChipTone> = {
  accent: "accent",
  live: "live",
  success: "success",
  warning: "warning",
  danger: "danger",
  plain: "neutral",
};

export function NewPoolTemplate({ chosen, onChoose }: { chosen: string; onChoose: (id: string) => void }): JSX.Element {
  return (
    <div className="flex flex-col gap-4">
      <p className="rounded-card border border-hairline bg-surface-2 px-3 py-2 text-meta text-ink-2">
        <span className="font-semibold text-ink">Tip</span> A template fills in roles and talk for you. You can change any of it on the canvas later.
      </p>
      <div>
        <h3 className="text-body font-semibold text-ink">What should this pool do?</h3>
        <p className="text-meta text-ink-2">Pick the closest one. Everything it sets is shown before you make it.</p>
      </div>
      <div role="radiogroup" aria-label="Template" className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-3" data-testid="new-pool-templates">
        {POOL_TEMPLATES.map((template) => {
          const on = template.id === chosen;
          return (
            <button
              key={template.id}
              type="button"
              role="radio"
              aria-checked={on}
              onClick={() => onChoose(template.id)}
              data-testid="new-pool-template"
              data-template={template.id}
              className={
                "flex flex-col gap-1.5 rounded-card border p-3 text-left outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent/60 " +
                (on ? "border-accent bg-accent-soft" : "border-hairline bg-surface hover:border-accent/50")
              }
            >
              <span className="text-body font-semibold text-ink">{template.name}</span>
              <span className="text-meta text-ink-2">{template.description}</span>
              {template.chips.length > 0 && (
                <span className="mt-0.5 flex flex-wrap gap-1">
                  {template.chips.map((chip) => (
                    <Chip key={chip.label} tone={TONE[chip.tone]}>
                      {chip.label}
                    </Chip>
                  ))}
                </span>
              )}
              {template.waits && <span className="text-label text-warning">{template.waits}</span>}
            </button>
          );
        })}
      </div>
    </div>
  );
}
