/**
 * AJ-E3011 step 6, What they may do on their own (§5), then, if wanted,
 * keeping the pool's work apart (the seal). The seal is a serious choice:
 * its consequence, an example, and "What does this mean?" with the guide.
 * D-126: the three ways as cards with a drawing each (OwnCards, the
 * diagram's words), the seal a switch row under Optional.
 */
import type { JSX } from "react";

import { RowList, ToggleRow } from "../../components/controls";
import { IconKey } from "../../components/icons-actions";
import { Explain } from "../../components/switch/Explain";
import { KEEP_APART_GUIDE, SEAL_CLAUDE, SEAL_TIPS } from "../../lib/keep-apart";
import type { Draft } from "../../lib/new-pool";
import { OwnCards } from "./OwnCards";
import { StepHead } from "./StepHead";

export function NewPoolOwn({ draft, onDraft, where }: { draft: Draft; onDraft: (next: Draft) => void; where: string }): JSX.Element {
  return (
    <div className="flex flex-col gap-7" data-testid="new-pool-step-own">
      <StepHead step="own" />
      <OwnCards value={draft.level} onChange={(level) => onDraft({ ...draft, level })} label="What sessions in this pool may do on their own" testId="new-pool-level" />
      <section aria-labelledby="new-pool-optional-title" className="flex flex-col gap-2.5">
        <h4 id="new-pool-optional-title" className="section-label">
          Optional
        </h4>
        <div data-testid="new-pool-seal">
          <RowList>
            <ToggleRow
              icon={IconKey}
              title="Keep this pool's work apart"
              help={
                <>
                  Its sessions leave other pools. No links, and no messages or docs out through AI Juggler. {SEAL_CLAUDE}
                  <span className="mt-0.5 block">Example: a client's project in {where}, kept out of your other pools.</span>
                </>
              }
              checked={draft.sealed}
              onChange={(sealed) => onDraft({ ...draft, sealed })}
              testId="new-pool-seal-switch"
            />
          </RowList>
        </div>
        <Explain
          testId="new-pool-seal-explain"
          guide={KEEP_APART_GUIDE}
          tips={[...SEAL_TIPS]}
        />
      </section>
    </div>
  );
}
