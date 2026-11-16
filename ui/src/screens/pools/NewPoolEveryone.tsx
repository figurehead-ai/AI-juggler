/**
 * AJ-E3011 step 4, Everyone else (§2, §4): "New sessions in web join as
 * [Member ▾]" — the old "Everyone in this pool's projects" row as a
 * labelled control that reads back as a sentence — then the four roles as
 * cards, and each member's role and what it may do on its own ("Same as
 * the pool" first), each row read back as a sentence. Round 2 (#2): an
 * App whose next session leads is one row: "Its next session leads · its
 * others join as [Member ▾]"; the roles are a plain reference (#9), and
 * each picker has its label ("Role:", "On its own:", #10).
 */
import type { JSX } from "react";

import { EVERYONE, withLead, type Draft, type DraftMember } from "../../lib/new-pool";
import type { PoolRole } from "../../lib/ipc/pools";
import { LEADS_WHEN, joinSentence, memberSentence, nextLeadsLine, nextLeadsSentence, nextSessionWords } from "../../lib/pool-words";
import { IconLead } from "../../components/icons-canvas";
import { JoinAs } from "../../pages/pool/setup/JoinAs";
import { MemberOwnLevel } from "../../pages/pool/setup/OwnLevelChoice";
import { RoleCards, RolePicker, RoleWhy, holdersOf } from "../../pages/pool/setup/RoleCards";
import { StepHead } from "./StepHead";

export function NewPoolEveryone({ draft, onDraft, where, nameOf }: { draft: Draft; onDraft: (next: Draft) => void; where: string; nameOf: (ref: string, label: string) => string }): JSX.Element {
  const setMember = (ref: string, change: Partial<DraftMember>) => onDraft({ ...draft, members: draft.members.map((member) => (member.ref === ref ? { ...member, ...change } : member)) });
  const everyone = draft.members.find((member) => member.ref === "all");
  const setJoin = (role: PoolRole | null) => {
    if (role === null) onDraft({ ...draft, members: draft.members.filter((member) => member.ref !== "all") });
    else if (everyone) setMember("all", { role });
    else onDraft({ ...draft, members: [...draft.members, { ...EVERYONE, role }] });
  };
  const named = draft.members.filter((member) => member.ref !== "all");
  const holders = holdersOf([
    ...named.map((member) => ({ name: member.ref === draft.leadNext ? `${nameOf(member.ref, member.label)}'s other sessions` : nameOf(member.ref, member.label), role: member.role })),
    ...(everyone ? [{ name: "new sessions", role: everyone.role }] : []),
    ...(draft.leadNext ? [{ name: nextSessionWords(nameOf(draft.leadNext, draft.members.find((member) => member.ref === draft.leadNext)?.label ?? "An App")), role: "lead" }] : []),
  ]);
  return (
    <div className="flex max-w-3xl flex-col gap-5" data-testid="new-pool-step-everyone">
      <StepHead step="everyone" />
      <div className="flex flex-col gap-1.5">
        <JoinAs where={where} value={everyone?.role ?? null} onChange={setJoin} testId="new-pool-join" />
        <p className="text-meta text-ink" data-testid="new-pool-join-sentence">
          {joinSentence(where, everyone?.role ?? null, everyone?.level ?? draft.level)}
        </p>
      </div>
      <div className="flex flex-col gap-2">
        <span className="section-label">What each role does</span>
        <RoleCards holders={holders} testId="new-pool-role-cards" />
      </div>
      {named.length > 0 && (
        <div className="flex flex-col gap-2">
          <span className="section-label">Each member</span>
          <ul className="flex flex-col gap-1" data-testid="new-pool-roles">
            {named.map((member) => {
              const name = nameOf(member.ref, member.label);
              const leadsNext = member.ref === draft.leadNext;
              return (
                <li key={member.ref} className="flex flex-col gap-1.5 rounded-xl bg-surface-2 px-3 py-2.5" data-testid="new-pool-member-row" data-leads-next={leadsNext || undefined}>
                  <span className="flex flex-wrap items-center gap-2">
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="text-body font-medium text-ink">{name}</span>
                      {leadsNext && (
                        // As its card on the diagram says it; the sentence
                        // below reads the rest back.
                        <span className="inline-flex items-center gap-1 text-label font-medium text-ink-2" data-testid="new-pool-member-leads-next" title={nextLeadsLine(member.role)}>
                          <IconLead aria-hidden="true" width={12} height={12} />
                          {LEADS_WHEN}
                        </span>
                      )}
                    </span>
                    <RolePicker
                      member={member.kind}
                      name={name}
                      value={member.role}
                      leadsNext={leadsNext}
                      onChange={(role) => {
                        // An App's Lead: its next session leads (round 3, #1), as Who leads sets it.
                        if (role === "lead") return onDraft(withLead(draft, member.kind === "app" ? { kind: "next", app: member.ref } : { kind: "session", ref: member.ref }));
                        const base = leadsNext ? withLead(draft, { kind: "none" }) : draft;
                        onDraft({ ...base, members: base.members.map((one) => (one.ref === member.ref ? { ...one, role } : one)) });
                      }}
                      testId="new-pool-member-role"
                    />
                    <MemberOwnLevel name={name} value={member.level} poolLevel={draft.level} onChange={(level) => setMember(member.ref, { level })} testId="new-pool-member-level" />
                  </span>
                  <RoleWhy member={member.kind} testId="new-pool-member-lead-why" />
                  <span className="text-meta text-ink-2" data-testid="new-pool-member-sentence">
                    {leadsNext ? nextLeadsSentence(name, member.role, member.level, draft.level) : memberSentence(name, member.role, member.level, draft.level)}
                  </span>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}
