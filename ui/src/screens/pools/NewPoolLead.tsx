/**
 * AJ-E3011 step 3, Who leads: the inline lead picker (§3). Never a dead
 * end: sessions open now, an App's next session, or no lead for now.
 * D-126: a drawing of what a lead does in three steps, then the choices
 * as cards with their logos (LeadCards, the diagram's words).
 * Round 2 (#3, #18): the App's row says it leads from its next session in
 * these projects, and that the pool's diagram can start one; the why-line
 * says leaving it for now is a choice.
 */
import type { JSX } from "react";

import { leadCandidates, leadOf, withLead, type Candidate, type Draft } from "../../lib/new-pool";
import { nextSessionWords } from "../../lib/pool-words";
import { useApps, type AppModel } from "../../lib/apps";
import { LEAD_STEPS } from "../../lib/pool-wizard-words";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { LeadArt } from "./illustrations";
import { LeadCards } from "./LeadCards";
import { isAdded, unavailableWords, useConnectorStatuses } from "../../lib/connector";
import { useAgentBridge } from "../../lib/ipc/agent-bridge";
import { StepHead } from "./StepHead";

export function NewPoolLead({
  draft,
  onDraft,
  sessions,
  nameOf,
  where = "these projects",
  open: instances = [],
  apps: given,
}: {
  draft: Draft;
  onDraft: (next: Draft) => void;
  sessions: readonly Candidate[];
  nameOf: (ref: string, label: string) => string;
  /** Its projects in words ("web"). */
  where?: string;
  /** The sessions open now (each choice's logo). */
  open?: readonly InstanceInfo[];
  /** Every App, the CLI ones too (each choice's logo). */
  apps?: readonly AppModel[] | null;
}): JSX.Element {
  const read = useApps();
  const known = given ?? read;
  const open = leadCandidates(sessions);
  // A session named in the draft that leads, though it isn't open (a
  // template's), stays offered.
  const picked = draft.members.filter((member) => member.kind === "session" && member.reach !== "watched" && !open.some((row) => row.ref === member.ref));
  const rows = [...picked.map((member) => ({ ref: member.ref, name: member.label, detail: "In this pool" })), ...open.map((row) => ({ ref: row.ref, name: row.label, detail: row.detail }))];
  // Round 6 (Casey's run): a desktop App leads through AI Juggler's
  // connector; one that can't take it says why, beside it.
  const desktop = draft.members.filter((member) => member.kind === "app" && !member.ref.startsWith("app:cli:")).map((member) => member.ref.slice("app:".length));
  const statuses = useConnectorStatuses(desktop);
  const bridge = useAgentBridge();
  const apps = draft.members
    .filter((member) => member.kind === "app")
    .map((member) => {
      const status = statuses.get(member.ref.slice("app:".length));
      if (!status || member.ref.startsWith("app:cli:")) return { ref: member.ref, name: nameOf(member.ref, member.label) };
      if (status.state === "unavailable") return { ref: member.ref, name: nameOf(member.ref, member.label), blocked: `Can't lead: ${unavailableWords(status, bridge)}` };
      return { ref: member.ref, name: nameOf(member.ref, member.label), note: isAdded(status.state) ? null : `It takes charge once AI Juggler's connector is in it (Who's in offers it), as soon as one of its sessions starts in ${where}.` };
    });
  // Sessions another app runs can't lead from there (their App can).
  const elsewhere = draft.members.filter((member) => member.kind === "session" && member.reach === "watched");
  const choice = leadOf(draft);
  return (
    <div className="flex flex-col gap-6" data-testid="new-pool-step-lead">
      <StepHead step="lead" />
      <div className="flex flex-wrap items-center gap-x-6 gap-y-3 rounded-card bg-surface py-3 pl-2 pr-5 inset-ring-1 inset-ring-hairline/75" data-testid="new-pool-lead-how">
        <LeadArt />
        <ol className="flex flex-col gap-2.5">
          {LEAD_STEPS.map((line, at) => (
            <li key={line} className="flex items-center gap-2.5 text-meta text-ink-2">
              <span aria-hidden="true" className="flex size-5 shrink-0 items-center justify-center rounded-full bg-surface-3 text-label font-semibold text-ink-2">
                {at + 1}
              </span>
              {line}
            </li>
          ))}
        </ol>
      </div>
      <LeadCards
        sessions={rows}
        apps={apps}
        value={choice}
        where={where}
        logos={{ apps: known ?? [], sessions: instances }}
        onChange={(next) => onDraft(withLead(draft, next, next.kind === "session" ? open.find((row) => row.ref === next.ref) : undefined))}
        testId="new-pool-lead"
      />
      {apps.length > 0 && (
        // The pool isn't made yet: no session can start into it.
        <p className="text-meta text-muted" data-testid="new-pool-lead-start-later">
          Once the pool is made, Start one now on its App's card starts that session.
        </p>
      )}
      {elsewhere.length > 0 && (
        <p className="text-meta text-ink-2" data-testid="new-pool-lead-elsewhere">
          {elsewhere.map((member) => nameOf(member.ref, member.label)).join(", ")} {elsewhere.length === 1 ? "runs" : "run"} in another app, so {elsewhere.length === 1 ? "it" : "they"} can't lead from here: add {elsewhere.length === 1 ? "its" : "their"} App in Who's in, and its next session can lead once AI Juggler's connector is in it.
        </p>
      )}
      {apps.length === 0 && open.length === 0 && elsewhere.length === 0 && (
        <p className="text-meta text-muted" data-testid="new-pool-lead-add-app">
          To have a session lead later, add its App in Who's in: then “{nextSessionWords("its App")}” shows here.
        </p>
      )}
    </div>
  );
}
