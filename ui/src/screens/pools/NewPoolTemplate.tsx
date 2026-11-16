/**
 * AJ-E3007 step 1, AJ-E3011's "How to start": the intro (full on the
 * first pool, one line after), then "Start from scratch" as a full card
 * beside the templates (it replaces Blank and the small "Set up by hand on
 * the canvas" link). Each template card shows its roles and talk in plain
 * words, so the choice is visible before going on (wireframe 1).
 *
 * Calm (D-117): cards stand off the page by surface tone, not borders;
 * the chosen one has the selection ring (2 px accent, offset) and a raised
 * surface, never a dashed or coloured fill.
 */
import type { JSX } from "react";

import { Disclosure } from "../../components/controls";
import { IconPlus } from "../../components/icons-actions";
import { POOL_TEMPLATES, type PoolTemplate } from "../../lib/pool-templates";
import { PoolIntro } from "../../pages/pool/setup/PoolIntro";
import { FOCUS_RING, SELECTED } from "../../pages/pool/selection";
import { StepHead } from "./StepHead";

function Card({ template, on, onChoose }: { template: PoolTemplate; on: boolean; onChoose: (id: string) => void }): JSX.Element {
  const scratch = template.id === "blank";
  return (
    <button
      type="button"
      role="radio"
      aria-checked={on}
      onClick={() => onChoose(template.id)}
      data-testid="new-pool-template"
      data-template={template.id}
      className={"flex flex-col gap-1.5 rounded-xl p-3 text-left transition-colors " + FOCUS_RING + " " + (on ? SELECTED : "bg-surface hover:bg-surface-2")}
    >
      <span className="flex items-center gap-1.5 text-body font-semibold text-ink">
        {scratch && <IconPlus aria-hidden="true" width={14} height={14} className="text-muted" />}
        {template.name}
      </span>
      <span className="text-meta text-ink-2">{template.description}</span>
      {template.chips.length > 0 && <span className="mt-0.5 text-label text-muted">{template.chips.map((chip) => chip.label).join(" · ")}</span>}
      {template.waits && <span className="text-label text-muted">{template.waits}</span>}
    </button>
  );
}

export function NewPoolTemplate({ chosen, onChoose, first, fold = false }: { chosen: string; onChoose: (id: string) => void; first: boolean; fold?: boolean }): JSX.Element {
  const scratch = POOL_TEMPLATES.find((template) => template.id === "blank")!;
  const templates = POOL_TEMPLATES.filter((template) => template.id !== "blank");
  const cards = templates.map((template) => <Card key={template.id} template={template} on={template.id === chosen} onChoose={onChoose} />);
  return (
    <div className="flex flex-col gap-5" data-testid="new-pool-step-template">
      <PoolIntro first={first} testId="new-pool-intro" />
      <StepHead step="template" />
      {fold ? (
        // Round 6 (Sam, from a project's own pool): nine cards of jargon to
        // someone who wants none. Start from scratch is the open path.
        <div role="radiogroup" aria-label="How to start" className="flex flex-col gap-3" data-testid="new-pool-templates">
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
            <Card template={scratch} on={chosen === scratch.id} onChoose={onChoose} />
          </div>
          <Disclosure summary="Or start from a template" open={chosen !== scratch.id} testId="new-pool-templates-fold">
            <div className="mt-2 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">{cards}</div>
          </Disclosure>
        </div>
      ) : (
        <div role="radiogroup" aria-label="How to start" className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3" data-testid="new-pool-templates">
          <Card template={scratch} on={chosen === scratch.id} onChoose={onChoose} />
          {cards}
        </div>
      )}
    </div>
  );
}
