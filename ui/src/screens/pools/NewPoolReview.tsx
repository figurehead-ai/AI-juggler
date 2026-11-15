/**
 * AJ-E3007 step 4, AJ-E3011's Review (§1, §2): the whole pool read back
 * in sentences — its name and where, who's in, who leads, how everyone
 * else joins, who may message whom, what they may do on their own, the
 * seal and the team tools — then what each member can actually do here,
 * and the team tools card. Round 3: the sentences and the card are the
 * pool inspector's own (PoolSentences, PoolToolsCard), from one shape, so
 * the two paths can't say a pool differently. Make pool follows.
 */
import type { JSX } from "react";

import type { Draft, Reach } from "../../lib/new-pool";
import { roleCard, type SetupStep } from "../../lib/pool-words";
import { openNowWords } from "../../lib/pool-open-now";
import { shapeOfDraft } from "../../lib/pool-sentences";
import { draftPlaces, type ToolsPlace } from "../../lib/team-tools";
import { PoolSentences, Sentence, type SentenceRow } from "../../pages/pool/setup/PoolSentences";
import { PoolToolsCard, useTeamTools } from "../../pages/pool/setup/PoolToolsCard";
import { StepHead } from "./StepHead";

const CAN: Record<Reach, string> = {
  full: "sees · messages · tasks · can be asked",
  shell: "acts through its shell: the juggler-agent command",
  watched: "watched: shown, can't act until continued in AI Juggler",
  later: "its sessions here join by themselves, now and later",
};

/** Review's rows, and the step each one's Change goes back to. */
const ROWS: readonly SentenceRow[] = ["who", "lead", "everyone", "talk", "own", "sealed", "tools"];
const STEP_OF: Partial<Record<SentenceRow, SetupStep>> = { who: "who", lead: "lead", everyone: "everyone", talk: "talk", own: "own", sealed: "own" };

export function NewPoolReview({
  draft,
  name,
  where,
  nameOf = (_, label) => label,
  openNow,
  projects = [],
  onChange,
}: {
  draft: Draft;
  /** The name it gets (typed, else the suggestion). */
  name: string;
  /** Its projects in words ("web", "every project"). */
  where: string;
  nameOf?: (ref: string, label: string) => string;
  /** Each member's open sessions here now (`openNowByMember`): a broad
   * member says them as its canvas card will (the rc27 P1 walk). */
  openNow?: ReadonlyMap<string, number>;
  /** The projects by name (the team tools say which are off). */
  projects?: readonly ToolsPlace[];
  /** Back to a step to change it (each sentence's Change). */
  onChange?: (step: SetupStep) => void;
}): JSX.Element {
  const shape = shapeOfDraft(draft, nameOf);
  const places = draftPlaces(draft, projects);
  const tools = useTeamTools(places.projects, places.apps);
  const named = draft.members.filter((member) => member.ref !== "all");
  const changeOf = (row: SentenceRow) => {
    const step = STEP_OF[row];
    return onChange && step ? () => onChange(step) : undefined;
  };
  return (
    <div className="flex max-w-3xl flex-col gap-5" data-testid="new-pool-step-review">
      <StepHead step="review" />
      <PoolSentences
        shape={shape}
        where={where}
        rows={ROWS}
        open={(ref) => openNow?.get(ref)}
        tools={tools}
        changeOf={changeOf}
        first={
          <Sentence title="Name and where" onChange={onChange ? () => onChange("where") : undefined} testId="new-pool-review">
            «{name}» works in {where}.
          </Sentence>
        }
        testId="new-pool-review"
      />
      {named.length > 0 && (
        <section className="flex flex-col gap-1.5" aria-label="What each member can do here">
          <span className="section-label">What each member can do here</span>
          <ul className="flex flex-col gap-0.5">
            {named.map((member) => (
              <li key={member.ref} className="flex flex-wrap items-baseline gap-x-3 gap-y-0.5 rounded-lg bg-surface-2 px-3 py-2">
                <span className="min-w-0 flex-1 truncate text-body font-medium text-ink">{nameOf(member.ref, member.label)}</span>
                <span className="text-meta text-ink-2">{member.ref === draft.leadNext ? "Its next session leads" : roleCard(member.role).label}</span>
                <span className="text-meta text-muted">{member.reach === "later" && openNow ? openNowWords(openNow.get(member.ref) ?? 0, where) : CAN[member.reach]}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
      <PoolToolsCard testId="new-pool-tools" projects={places.projects} apps={places.apps} />
      <p className="text-meta text-ink-2">You can change all of this on the pool's diagram.</p>
    </div>
  );
}
