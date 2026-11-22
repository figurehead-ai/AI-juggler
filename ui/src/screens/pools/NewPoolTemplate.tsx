/**
 * AJ-E3007 step 1, AJ-E3011's "How to start": "Start from scratch" as a
 * full card first, then the templates. Each template's card says what it
 * sets in plain words, so the choice is visible before going on
 * (wireframe 1). What a pool is, is the side's (NewPoolStartSide).
 *
 * D-126: the kit's choice cards (RadioCards, one radio group for all of
 * them: the arrows move and choose), Start from scratch with its plus
 * tile and "Suggested for a first pool", each template with a small
 * drawing of its team. Round 6 (Sam, from a project or its own pool):
 * the templates fold away behind "Or start from a template".
 */
import type { JSX } from "react";

import { Disclosure, RadioCard, RadioCards } from "../../components/controls";
import { IconPlus } from "../../components/icons-actions";
import { LogoTile } from "../../components/ToolLogo";
import { Pill } from "../../components/Pill";
import { POOL_TEMPLATES, type PoolTemplate } from "../../lib/pool-templates";
import { TemplateArt } from "./illustrations";
import { StepHead } from "./StepHead";

/** Each template's drawing (one not listed gets the team's). */
const SHAPES: Record<string, string> = {
  "writer-reviewer": "pair",
  "feature-team": "team",
  "across-repos": "repos",
  "reviewer-for-everything": "reviewer",
  "rules-by-account": "account",
  "build-mac": "mac",
  "sealed-client": "sealed",
  scouts: "scouts",
};

/** A template's card: its drawing, name, line, and what it sets. */
function Card({ template }: { template: PoolTemplate }): JSX.Element {
  return (
    <div data-testid="new-pool-template" data-template={template.id} className="flex min-w-0">
      <RadioCard
        value={template.id}
        logo={
          <span aria-hidden="true" className="flex h-12 w-[72px] items-center justify-center rounded-[10px] bg-canvas-hi">
            <TemplateArt shape={SHAPES[template.id] ?? "team"} />
          </span>
        }
        title={template.name}
        sub={template.description}
      >
        {template.chips.length > 0 && <span className="mt-1 text-label text-muted">{template.chips.map((chip) => chip.label).join(" · ")}</span>}
        {template.waits && <span className="text-label text-muted">{template.waits}</span>}
      </RadioCard>
    </div>
  );
}

/** Start from scratch, the full card first. */
function Scratch({ template, first }: { template: PoolTemplate; first: boolean }): JSX.Element {
  return (
    <div data-testid="new-pool-template" data-template={template.id} className="flex min-w-0 @xl:col-span-2">
      <RadioCard
        value={template.id}
        logo={<LogoTile icon={IconPlus} size={40} on />}
        title={
          <span className="inline-flex flex-wrap items-center gap-2">
            {template.name}
            {first && <Pill tone="accent">Suggested for a first pool</Pill>}
          </span>
        }
        sub={template.description}
      />
    </div>
  );
}

export function NewPoolTemplate({ chosen, onChoose, first, fold = false }: { chosen: string; onChoose: (id: string) => void; first: boolean; fold?: boolean }): JSX.Element {
  const scratch = POOL_TEMPLATES.find((template) => template.id === "blank")!;
  const templates = POOL_TEMPLATES.filter((template) => template.id !== "blank");
  const cards = templates.map((template) => <Card key={template.id} template={template} />);
  return (
    <div className="flex flex-col gap-7" data-testid="new-pool-step-template">
      <StepHead step="template" />
      <div data-testid="new-pool-templates">
        <RadioCards label="How to start" value={chosen} onChange={onChoose} columns={2}>
          <Scratch template={scratch} first={first} />
          {fold ? (
            // Round 6 (Sam, from a project's own pool): nine cards of
            // jargon to someone who wants none. Start from scratch is the
            // open path.
            <Disclosure summary="Or start from a template" open={chosen !== scratch.id} testId="new-pool-templates-fold" className="@xl:col-span-2">
              <div className="mt-2.5 grid grid-cols-1 gap-2.5 @xl:grid-cols-2">{cards}</div>
            </Disclosure>
          ) : (
            <>
              <p className="mt-4 flex items-baseline gap-2.5 @xl:col-span-2">
                <span className="section-label">Or start from a ready-made team</span>
                <span className="text-label text-faint">{templates.length} templates</span>
              </p>
              {cards}
            </>
          )}
        </RadioCards>
      </div>
    </div>
  );
}
