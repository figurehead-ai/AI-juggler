/**
 * AJ-E3011 step 3, Who leads: the inline lead picker (§3). Never a dead
 * end: sessions open now, an App's next session, or no lead for now.
 * Round 2 (#3, #18): the App's row says it leads from its next session in
 * these projects, and that the pool's diagram can start one; the why-line
 * says leaving it for now is a choice.
 */
import type { JSX } from "react";

import { leadCandidates, leadOf, withLead, type Candidate, type Draft } from "../../lib/new-pool";
import { nextSessionWords } from "../../lib/pool-words";
import { LeadPicker } from "../../pages/pool/setup/LeadPicker";
import { StepHead } from "./StepHead";

export function NewPoolLead({
  draft,
  onDraft,
  sessions,
  nameOf,
  where = "these projects",
}: {
  draft: Draft;
  onDraft: (next: Draft) => void;
  sessions: readonly Candidate[];
  nameOf: (ref: string, label: string) => string;
  /** Its projects in words ("web"). */
  where?: string;
}): JSX.Element {
  const open = leadCandidates(sessions);
  // A session named in the draft that leads, though it isn't open (a
  // template's), stays offered.
  const picked = draft.members.filter((member) => member.kind === "session" && member.reach !== "watched" && !open.some((row) => row.ref === member.ref));
  const rows = [...picked.map((member) => ({ ref: member.ref, name: member.label, detail: "In this pool" })), ...open.map((row) => ({ ref: row.ref, name: row.label, detail: row.detail }))];
  const apps = draft.members.filter((member) => member.kind === "app").map((member) => ({ ref: member.ref, name: nameOf(member.ref, member.label) }));
  const choice = leadOf(draft);
  return (
    <div className="flex max-w-3xl flex-col gap-4" data-testid="new-pool-step-lead">
      <StepHead step="lead" />
      <LeadPicker
        sessions={rows}
        apps={apps}
        value={choice}
        where={where}
        onChange={(next) => onDraft(withLead(draft, next, next.kind === "session" ? open.find((row) => row.ref === next.ref) : undefined))}
        testId="new-pool-lead"
      />
      {apps.length > 0 && (
        // The pool isn't made yet: no session can start into it.
        <p className="text-meta text-muted" data-testid="new-pool-lead-start-later">
          Once the pool is made, Start one now on its App's card starts that session.
        </p>
      )}
      {apps.length === 0 && open.length === 0 && (
        <p className="text-meta text-muted" data-testid="new-pool-lead-add-app">
          To have a session lead later, add its App in Who's in: then «{nextSessionWords("its App")}» shows here.
        </p>
      )}
    </div>
  );
}
