/**
 * AJ-E3011 step 5, Who may message whom (§6). D-126: three ways first,
 * as cards with a small drawing each — Everyone talks freely, Members
 * ask you first, Members go through the lead — then each pair in the
 * talk table (Allowed calm, colour only for an exception), a click on a
 * cell moving it on. The ways are the talk rules there are (no new
 * backend): each sets the whole table, and a table changed by hand is
 * its own mix, no way chosen.
 */
import type { JSX } from "react";

import { RadioCard, RadioCards } from "../../components/controls";
import type { Draft } from "../../lib/new-pool";
import { nextRule, pairKey, ruleOf } from "../../lib/pool-templates";
import { OWN_MIX, presetOf, TALK_PRESETS } from "../../lib/pool-wizard-words";
import { TalkTable } from "../../pages/pool/setup/TalkTable";
import { TalkArt } from "./illustrations";
import { StepHead } from "./StepHead";

export function NewPoolTalk({ draft, onDraft }: { draft: Draft; onDraft: (next: Draft) => void }): JSX.Element {
  const preset = presetOf(draft.talk);
  return (
    <div className="flex flex-col gap-7" data-testid="new-pool-step-talk">
      <StepHead step="talk" />
      <RadioCards label="Who may message whom" value={preset} onChange={(id) => onDraft({ ...draft, talk: { ...TALK_PRESETS.find((one) => one.id === id)!.grid } })} columns={3} testId="new-pool-talk-presets">
        {TALK_PRESETS.map((one) => (
          <RadioCard
            key={one.id}
            value={one.id}
            layout="stack"
            logo={
              <span aria-hidden="true" className="flex h-16 w-36 items-center justify-center rounded-[10px] bg-canvas-hi">
                <TalkArt preset={one.id} />
              </span>
            }
            title={one.label}
            sub={one.line}
            testId="new-pool-talk-preset"
          />
        ))}
      </RadioCards>
      <section aria-labelledby="new-pool-pairs-title" className="flex flex-col gap-2.5">
        <h4 id="new-pool-pairs-title" className="flex flex-wrap items-baseline gap-x-2.5">
          <span className="section-label">Each pair</span>
          {preset === null && (
            <span className="text-label font-medium text-ink-2" data-testid="new-pool-talk-mix">
              {OWN_MIX}
            </span>
          )}
        </h4>
        <div className="rounded-card bg-surface p-3 inset-ring-1 inset-ring-hairline/75">
          <TalkTable
            ruleOf={(from, to) => ruleOf(draft.talk, from, to)}
            onCycle={(from, to) => onDraft({ ...draft, talk: { ...draft.talk, [pairKey(from, to)]: nextRule(ruleOf(draft.talk, from, to), from) } })}
            testId="new-pool-talk"
          />
        </div>
      </section>
    </div>
  );
}
