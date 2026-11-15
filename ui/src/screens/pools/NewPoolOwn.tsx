/**
 * AJ-E3011 step 6, What they may do on their own (§5), then the seal, if
 * wanted. The seal is a serious choice: its consequence, an example, and
 * "What does this mean?" with the guide.
 */
import type { JSX } from "react";

import { ChoiceOption } from "../../components/controls";
import { Explain } from "../../components/switch/Explain";
import type { Draft } from "../../lib/new-pool";
import { POOLS_GUIDE } from "../../lib/pool-words";
import { OwnLevelChoice } from "../../pages/pool/setup/OwnLevelChoice";
import { StepHead } from "./StepHead";

export function NewPoolOwn({ draft, onDraft, where }: { draft: Draft; onDraft: (next: Draft) => void; where: string }): JSX.Element {
  return (
    <div className="flex max-w-3xl flex-col gap-5" data-testid="new-pool-step-own">
      <StepHead step="own" />
      <OwnLevelChoice value={draft.level} onChange={(level) => onDraft({ ...draft, level })} label={`What sessions in this pool may do on their own`} testId="new-pool-level" />
      <div className="flex flex-col gap-2">
        <span className="section-label">If wanted</span>
        <div className={"rounded-xl p-1 " + (draft.sealed ? "bg-surface-3" : "bg-surface-2")} data-testid="new-pool-seal">
          <ChoiceOption
            type="checkbox"
            checked={draft.sealed}
            onChange={(event) => onDraft({ ...draft, sealed: event.target.checked })}
            label={<span className="font-semibold text-ink">Seal this pool: keep its work apart</span>}
            hint={
              <>
                Its sessions leave other pools. No links, and no messages or docs out through AI Juggler. Claude Code sessions AI Juggler starts here also leave Claude's own messages between sessions, from their next start.
                <span className="mt-0.5 block text-muted">Example: a client's project in {where}, kept out of your other pools.</span>
              </>
            }
          />
        </div>
        <Explain
          testId="new-pool-seal-explain"
          guide={`${POOLS_GUIDE}#make-a-pool`}
          tips={["A sealed pool's sessions are in no other pool, and nothing leaves it through AI Juggler.", "A switch into or out of a sealed pool asks first.", "You can unseal it later from its diagram; its sessions go back into the pools they were in."]}
        />
      </div>
    </div>
  );
}
