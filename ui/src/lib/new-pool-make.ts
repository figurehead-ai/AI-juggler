/**
 * New pool's Make (AJ-E3011): the draft into a pool — made, or an
 * existing one set up (`into`) — then read back, so every chosen member
 * is in it or said not to be, with why (round 6).
 */
import { errorText } from "./bridge";
import { poolJoin, poolRules, poolSetLeadNext, poolSetScope, poolSetSealed, poolSetTalk } from "./ipc/pool-model";
import { poolCreate, poolSetAccess, poolSetSpansMacs, type PoolInfo } from "./ipc/pools";
import { talkChanges, type Draft } from "./new-pool";

/** A chosen member the made pool doesn't hold, and why (round 6: never
 * dropped silently). */
export interface NotAdded {
  ref: string;
  label: string;
  why: string;
}

export async function makePool(draft: Draft, into?: string, { keepScope = false }: { keepScope?: boolean } = {}): Promise<{ id: string; notAdded: NotAdded[] }> {
  let made: Pick<PoolInfo, "id"> | undefined;
  if (into) made = { id: into };
  else {
    const before = await poolCreate(draft.name.trim(), draft.every || draft.projects.length !== 1 ? null : draft.projects[0] ?? null);
    made = before.pools.find((pool) => pool.name === draft.name.trim() && pool.kind === "user");
  }
  if (!made) throw new Error("The pool wasn't made");
  // A project's own pool works in its project: its scope isn't set.
  if (!keepScope) await poolSetScope(made.id, draft.every, draft.every ? [] : draft.projects);
  if (draft.level !== "work_together") await poolSetAccess(made.id, draft.level);
  // Each member on its own: one refused never loses the rest (round 6,
  // Casey's run: two Apps dropped without a word).
  const refusedWhy = new Map<string, string>();
  for (const member of draft.members) {
    try {
      await poolJoin(made.id, member.ref, member.role, member.level);
    } catch (cause) {
      refusedWhy.set(member.ref, errorText(cause));
    }
  }
  for (const change of talkChanges(draft.talk)) await poolSetTalk(made.id, change.from, change.to, change.rule);
  // AJ-E3011: its App's next session leads, once the App is in.
  if (draft.leadNext && !draft.members.some((member) => member.role === "lead")) await poolSetLeadNext(made.id, draft.leadNext.slice("app:".length));
  if (draft.sealed) await poolSetSealed(made.id, true);
  if (draft.spansMacs) await poolSetSpansMacs(made.id, true);
  // Read back: every chosen member is in it, or said not to be, with why.
  const kept = new Set((await poolRules(made.id)).members.map((rule) => rule.member));
  const notAdded = draft.members.filter((member) => !kept.has(member.ref)).map((member) => ({ ref: member.ref, label: member.label, why: refusedWhy.get(member.ref) ?? "AI Juggler didn't keep it in the pool" }));
  return { id: made.id, notAdded };
}
