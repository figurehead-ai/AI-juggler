/**
 * 0.4.6 (AJ-E3011 §3): who leads a draft pool, never a dead end. D-92:
 * one session leads. The picker offers the sessions open now, an App's
 * next session (the pool's `lead_next`), or no lead for now.
 */
import type { PoolRole } from "./ipc/pools";
import type { Candidate, Draft } from "./new-pool";

const MEMBER: PoolRole = "member";

/** Who leads, as the picker says it (D-92: one session). */
export type LeadChoice = { kind: "session"; ref: string } | { kind: "next"; app: string } | { kind: "none" };

/** The draft's lead: the session member holding Lead, else the App whose
 * next session leads, else none. */
export function leadOf(draft: Pick<Draft, "members" | "leadNext">): LeadChoice {
  const lead = draft.members.find((member) => member.role === "lead");
  if (lead) return { kind: "session", ref: lead.ref };
  return draft.leadNext ? { kind: "next", app: draft.leadNext } : { kind: "none" };
}

/** The draft with `choice` leading: a session takes Lead (added when it
 * isn't in yet, from `candidate`), every other Lead becomes a Member, and
 * the wait for an App's next session is set or cleared. */
export function withLead(draft: Draft, choice: LeadChoice, candidate?: Candidate): Draft {
  const members = draft.members.map((member) => (member.role === "lead" ? { ...member, role: MEMBER } : member));
  if (choice.kind === "session") {
    const at = members.findIndex((member) => member.ref === choice.ref);
    if (at >= 0) members[at] = { ...members[at]!, role: "lead" };
    else if (candidate) members.push({ ref: candidate.ref, kind: candidate.kind, label: candidate.label, role: "lead", level: null, reach: candidate.reach, shell: candidate.shell ?? null });
  }
  return { ...draft, members, leadNext: choice.kind === "next" ? choice.app : null };
}

/** The sessions that may lead: open now in the pool's projects, able to
 * act (not Watched), most recent first. */
export function leadCandidates(sessions: readonly Candidate[]): Candidate[] {
  return sessions.filter((row) => row.kind === "session" && row.blocked === null && row.group === "open" && row.reach !== "watched").sort((a, b) => b.lastMs - a.lastMs);
}

