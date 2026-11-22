/**
 * What you change on a pool's canvas: put a session in, take one out,
 * make one the lead. Each answers in a toast, with Undo where it can be
 * undone (Max-Depth C1: undo beats confirm). Nothing here touches a
 * session itself: only its pool and its role.
 */
import { errorText } from "../../lib/bridge";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { poolMove, poolSetRole, type PoolInfo } from "../../lib/ipc/pools";
import { withPendingAdd, withPendingRole } from "../../lib/pool-pending";
import { POOL_RULES_CHANGED, memberRef, poolJoin, poolLeave } from "../../lib/ipc/pool-model";
import { defaultPoolIdOf, setPools, type PoolRole } from "../../lib/pools";
import { holdAction, showToast } from "../../components/Toast";
import { shortName } from "../../lib/session-titles";
import type { Point } from "./layout";
import type { MemberNode } from "./useCanvas";

type Place = (nodes: { session_id: string; x: number; y: number }[]) => Promise<void>;

const failed = (message: string) => (cause: unknown) => showToast({ message, detail: errorText(cause), tone: "error" });

/** A session's name as the pool shows it: the shared short name. */
export function nameOf(session: InstanceInfo): string {
  return shortName(session);
}

/** Why a session can't come into `pool`, or null when it can (D-62: the
 * pool's scope holds its project, or the pool takes every project). */
export function cannotJoin(pool: PoolInfo, session: InstanceInfo): string | null {
  if (pool.scope_all || pool.scope.includes(session.project_id ?? null)) return null;
  if (pool.scope.length === 1 && pool.scope[0] === null) return "This pool is for sessions with no project.";
  return pool.kind === "default" ? "This pool is for one project; that session works in another." : "That session's project isn't in this pool's scope. Add it to the scope first.";
}

/** Puts a session in the pool, at `at` on the canvas when it was dropped
 * there. D-62 decision 10: a session can be in several pools, so it
 * stays in the ones it's in (the rc31 walk: adding it to a second pool
 * took it out of the first); Undo takes it out of this one only. A
 * sealed pool's sessions are in no other pool (decision 14): there it
 * moves, and the toast names the pool it left (`fromName`). */
export async function addToPool(pool: PoolInfo, session: InstanceInfo, at: Point | null, place: Place, from: string | null, fromName: string | null = null): Promise<void> {
  const why = cannotJoin(pool, session);
  if (why) {
    showToast({ message: `Couldn't add ${nameOf(session)}`, detail: why, tone: "warning" });
    return;
  }
  const ref = memberRef("session", session.id);
  try {
    // Round 4 (#5): shown as being added at once; settles when the canvas has it.
    const join = () => (pool.sealed ? poolMove(session.id, pool.id) : poolJoin(pool.id, ref, "member"));
    setPools(await withPendingAdd({ poolId: pool.id, ref: `session:${session.id}`, label: nameOf(session), kind: "session" }, join));
    if (at) await place([{ session_id: session.id, x: at.x, y: at.y }]);
    const moved = pool.sealed && from !== null && from !== pool.id;
    showToast({
      message: `${nameOf(session)} is in ${pool.name}`,
      detail: moved ? `It left “${fromName ?? "its pool"}”: a sealed pool's sessions are in no other pool.` : undefined,
      undo: pool.sealed
        ? from
          ? { onUndo: async () => setPools(await poolMove(session.id, from)) }
          : undefined
        : { onUndo: async () => setPools(await poolLeave(pool.id, ref)) },
      testId: "canvas-added",
    });
  } catch (cause) {
    failed(`Couldn't add ${nameOf(session)}`)(cause);
  }
}

/** Why a member can't be taken out, or null when it can. */
export function cannotRemove(pool: PoolInfo): string | null {
  return pool.kind === "default" ? "Every session of its project is in this pool unless you put it in another one." : null;
}

/** How long a take-out waits for Undo before it's done, in seconds. */
const TAKE_OUT_HOLD_SECONDS = 5;

/** Takes a member out (back to its project's own pool) through the held
 * action, like Stop (AJ-E3011 round 4): it waits for Undo first, so a
 * misfire (VoiceOver, automation, a slip) costs nothing. */
export function removeFromPool(pool: PoolInfo, member: MemberNode): void {
  if (cannotRemove(pool)) return;
  const name = member.name;
  holdAction({
    id: `take-out:${pool.id}:${member.id}`,
    message: `Taking ${name} out of ${pool.name}.`,
    detail: "Undo keeps it in the pool.",
    seconds: TAKE_OUT_HOLD_SECONDS,
    testId: "canvas-removed",
    undone: `${name} stays in ${pool.name}.`,
    failed: (cause) => `Couldn't take ${name} out: ${cause}`,
    run: async () => setPools(await poolMove(member.id, defaultPoolIdOf(member.session.project_id))),
  });
}

/** The same for a wider member (an App, Account, Tool, Mac, everyone). */
export function takeOutMember(pool: PoolInfo, member: string, name: string): void {
  holdAction({
    id: `take-out:${pool.id}:${member}`,
    message: `Taking ${name} out of ${pool.name}.`,
    detail: "Undo keeps it in the pool.",
    seconds: TAKE_OUT_HOLD_SECONDS,
    testId: "pool-take-out",
    undone: `${name} stays in ${pool.name}.`,
    failed: (cause) => `Couldn't take ${name} out: ${cause}`,
    run: async () => {
      setPools(await poolLeave(pool.id, member));
      window.dispatchEvent(new CustomEvent(POOL_RULES_CHANGED, { detail: pool.id }));
    },
  });
}

/** A named session whose role is set: its card, its inspector or its List
 * view row (SK-13). */
export type RoleTarget = Pick<MemberNode, "id" | "role" | "name">;

/** Makes a member the pool's lead; the lead it had becomes a member.
 * Undo gives the roles back. */
export async function makeLead(pool: PoolInfo, member: RoleTarget, members: readonly Pick<MemberNode, "id" | "role" | "session">[]): Promise<void> {
  const before = members.filter((other) => other.role === "lead" && other.id !== member.id);
  const was = member.role as PoolRole;
  try {
    for (const lead of before) await poolSetRole(lead.id, "member", pool.id);
    setPools(await withPendingRole({ poolId: pool.id, ref: member.id, role: "lead" }, () => poolSetRole(member.id, "lead", pool.id)));
    showToast({
      message: `${member.name} leads now`,
      detail: before.length > 0 ? `${before.map((lead) => nameOf(lead.session)).join(", ")} ${before.length === 1 ? "is a member" : "are members"} again.` : undefined,
      undo: {
        onUndo: async () => {
          setPools(await poolSetRole(member.id, was, pool.id));
          for (const lead of before) setPools(await poolSetRole(lead.id, "lead", pool.id));
        },
      },
    });
  } catch (cause) {
    failed("Couldn't change the lead")(cause);
  }
}

/** Sets a member's role in `pool` (member, reviewer or observer). */
export async function setRole(pool: PoolInfo, member: RoleTarget, role: PoolRole): Promise<void> {
  try {
    setPools(await withPendingRole({ poolId: pool.id, ref: member.id, role }, () => poolSetRole(member.id, role, pool.id)));
  } catch (cause) {
    failed("Couldn't change its role")(cause);
  }
}
