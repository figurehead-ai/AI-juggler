/**
 * AJ-E3007 step 4, AJ-E3011's Review (§1, §2): the whole pool read back
 * in sentences — its name and where, who's in, who leads, how everyone
 * else joins, who may message whom, what they may do on their own and the
 * seal — then what each member can actually do here (full, shell only,
 * watched, or joins later), and the agent-tools consent where it applies
 * (§7). Make pool follows.
 */
import type { JSX, ReactNode } from "react";

import { leadOf, type Draft, type Reach } from "../../lib/new-pool";
import { ruleOf, TALK_ROLES } from "../../lib/pool-templates";
import { TALK_WORDS, joinSentence, leadSentence, levelSentence, memberSentence, roleCard } from "../../lib/pool-words";
import { openNowWords } from "../../lib/pool-open-now";
import { PoolToolsCard } from "../../pages/pool/setup/PoolToolsCard";
import { StepHead } from "./StepHead";

const CAN: Record<Reach, string> = {
  full: "sees · messages · tasks · can be asked",
  shell: "acts through its shell: the juggler-agent command",
  watched: "watched: shown, can't act until continued in AI Juggler",
  later: "its sessions here join by themselves, now and later",
};

/** The talk grid in a sentence: "Everyone may message everyone" or its
 * exceptions. */
export function talkSentence(draft: Pick<Draft, "talk">): string {
  // An Observer that may message no one "only reads", said once.
  const observerReads = TALK_ROLES.filter((to) => to !== "observer").every((to) => ruleOf(draft.talk, "observer", to) === "blocked");
  const changed: string[] = [];
  for (const from of TALK_ROLES) {
    for (const to of TALK_ROLES) {
      const rule = ruleOf(draft.talk, from, to);
      if (rule !== "allowed" && !(from === "observer" && observerReads)) changed.push(`${roleCard(from).label} to ${roleCard(to).label}: ${TALK_WORDS[rule].label.toLowerCase()}`);
    }
  }
  const reads = observerReads ? "; an Observer only reads" : "";
  return changed.length === 0 ? `Everyone may message everyone${reads}.` : `${changed.join("; ")}. The rest is allowed${reads}.`;
}

/** Who's in, in a sentence: the members by name, and whether new sessions
 * join by themselves. */
export function whoSentence(draft: Pick<Draft, "members">, nameOf: (ref: string, label: string) => string = (_, label) => label): string {
  const named = draft.members.filter((member) => member.ref !== "all");
  const everyone = draft.members.some((member) => member.ref === "all");
  if (named.length === 0) return everyone ? "Every session in these projects, as it starts." : "No one yet: add members now, or on the canvas later.";
  const joins = named.some((member) => member.reach === "later");
  const names = named.map((member) => nameOf(member.ref, member.label)).join(", ");
  if (everyone) return `${names}, and every other session in these projects as it starts.`;
  return `${names}. ${joins ? "Their new sessions here join by themselves." : "Nothing else joins by itself."}`;
}

export function NewPoolReview({
  draft,
  name,
  where,
  nameOf = (_, label) => label,
  openNow,
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
}): JSX.Element {
  const lead = leadOf(draft);
  const leadName = lead.kind === "session" ? nameOf(lead.ref, draft.members.find((member) => member.ref === lead.ref)?.label ?? "A session") : null;
  const nextName = lead.kind === "next" ? nameOf(lead.app, draft.members.find((member) => member.ref === lead.app)?.label ?? "An App") : null;
  const everyone = draft.members.find((member) => member.ref === "all");
  const named = draft.members.filter((member) => member.ref !== "all");
  return (
    <div className="flex max-w-3xl flex-col gap-5" data-testid="new-pool-step-review">
      <StepHead step="review" />
      <dl className="flex flex-col gap-1" data-testid="new-pool-review">
        <Sentence title="Name and where">
          «{name}» works in {where}.
        </Sentence>
        <Sentence title="Who's in">{whoSentence(draft, nameOf)}</Sentence>
        <Sentence title="Who leads">{leadSentence(leadName, nextName)}</Sentence>
        <Sentence title="Everyone else">
          {joinSentence(where, everyone?.role ?? null, everyone?.level ?? draft.level)}
          {named
            .filter((member) => member.role !== "lead")
            .map((member) => (
              <span key={member.ref} className="block" data-testid="new-pool-review-role">
                {memberSentence(nameOf(member.ref, member.label), member.role, member.level, draft.level)}
              </span>
            ))}
        </Sentence>
        <Sentence title="Who may message whom">{talkSentence(draft)}</Sentence>
        <Sentence title="On their own">{levelSentence(draft.level)}</Sentence>
        <Sentence title="Kept apart">{draft.sealed ? "Sealed: its sessions are in no other pool, and nothing leaves it through AI Juggler." : "Not sealed: its sessions can be in other pools too."}</Sentence>
      </dl>
      {named.length > 0 && (
        <section className="flex flex-col gap-1.5" aria-label="What each member can do here">
          <span className="section-label">What each member can do here</span>
          <ul className="flex flex-col gap-0.5">
            {named.map((member) => (
              <li key={member.ref} className="flex flex-wrap items-baseline gap-x-3 gap-y-0.5 rounded-lg bg-surface-2 px-3 py-2">
                <span className="min-w-0 flex-1 truncate text-body font-medium text-ink">{nameOf(member.ref, member.label)}</span>
                <span className="text-meta text-ink-2">{roleCard(member.role).label}</span>
                <span className="text-meta text-muted">{member.reach === "later" && openNow ? openNowWords(openNow.get(member.ref) ?? 0) : CAN[member.reach]}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
      <PoolToolsCard testId="new-pool-tools" />
      <p className="text-meta text-ink-2">You can change all of this on the pool's canvas.</p>
    </div>
  );
}

function Sentence({ title, children }: { title: string; children: ReactNode }): JSX.Element {
  return (
    <div className="grid grid-cols-1 gap-x-4 gap-y-0.5 rounded-lg px-1 py-1.5 sm:grid-cols-[11rem_1fr]">
      <dt className="section-label pt-0.5">{title}</dt>
      <dd className="text-body text-ink">{children}</dd>
    </div>
  );
}
