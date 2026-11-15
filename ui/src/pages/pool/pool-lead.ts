/**
 * D-92, AJ-E3011: who leads a made pool, set one way from every control
 * that sets it (the pool's Who leads, an App's role picker): the chosen
 * session takes Lead and every other Lead becomes a Member; an App's next
 * session (the pool's `lead_next`) or no lead clears the session leads
 * first, then sets or clears the wait.
 */
import { poolSetLeadNext } from "../../lib/ipc/pool-model";
import { poolSetRole } from "../../lib/ipc/pools";
import type { LeadChoice } from "../../lib/new-pool";
import { setPools } from "../../lib/pools";

export async function setPoolLead(poolId: string, leads: readonly string[], choice: LeadChoice): Promise<void> {
  for (const lead of leads) if (choice.kind !== "session" || lead !== choice.ref) setPools(await poolSetRole(lead, "member", poolId));
  if (choice.kind === "session") setPools(await poolSetRole(choice.ref, "lead", poolId));
  else await poolSetLeadNext(poolId, choice.kind === "next" ? choice.app.slice("app:".length) : null);
}
