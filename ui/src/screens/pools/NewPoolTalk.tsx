/**
 * AJ-E3011 step 5, Who may message whom (§6): the talk table, Allowed
 * calm and colour only for an exception, read back as a sentence.
 */
import type { JSX } from "react";

import type { Draft } from "../../lib/new-pool";
import { nextRule, pairKey, ruleOf } from "../../lib/pool-templates";
import { TalkTable } from "../../pages/pool/setup/TalkTable";
import { talkSentence } from "./NewPoolReview";
import { StepHead } from "./StepHead";

export function NewPoolTalk({ draft, onDraft }: { draft: Draft; onDraft: (next: Draft) => void }): JSX.Element {
  return (
    <div className="flex max-w-3xl flex-col gap-4" data-testid="new-pool-step-talk">
      <StepHead step="talk" />
      <TalkTable
        ruleOf={(from, to) => ruleOf(draft.talk, from, to)}
        onCycle={(from, to) => onDraft({ ...draft, talk: { ...draft.talk, [pairKey(from, to)]: nextRule(ruleOf(draft.talk, from, to)) } })}
        testId="new-pool-talk"
      />
      <p className="text-meta text-ink" data-testid="new-pool-talk-sentence">
        {talkSentence(draft)}
      </p>
    </div>
  );
}
