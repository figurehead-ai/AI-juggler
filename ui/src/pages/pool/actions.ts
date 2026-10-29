/**
 * What you change on a pool's canvas: put a session in, take one out,
 * make one the lead. Each answers in a toast, with Undo where it can be
 * undone (Max-Depth C1: undo beats confirm). Nothing here touches a
 * session itself: only its pool and its role.
 */
import { errorText, type InstanceInfo } from "../../lib/bridge";
import { poolMove, poolSetRole, type PoolInfo } from "../../lib/ipc/pools";
import { defaultPoolIdOf, setPools, type PoolRole } from "../../lib/pools";
import { showToast } from "../../components/Toast";
import type { Point } from "./layout";
import type { MemberNode } from "./useCanvas";

type Place = (nodes: { session_id: string; x: number; y: number }[]) => Promise<void>;

const failed = (message: string) => (cause: unknown) => showToast({ message, detail: errorText(cause), tone: "error" });

/** A session's name as the canvas shows it. */
export function nameOf(session: InstanceInfo): string {
  return session.label ?? session.title ?? session.git_branch ?? (session.cwd.replace(/\/+$/, "").split("/").pop() || "A session");
}

/** Why a session can't come into `pool`, or null when it can. */
export function cannotJoin(pool: PoolInfo, session: InstanceInfo): string | null {
  if (pool.project_id !== null && session.project_id !== pool.project_id) return "This pool is for one project; that session works in another.";
  if (pool.kind === "default" && pool.project_id === null && session.project_id) return "This pool is for sessions with no project.";
  return null;
}

/** Puts a session in the pool, at `at` on the canvas when it was dropped
 * there. Undo puts it back where it was. */
export async function addToPool(pool: PoolInfo, session: InstanceInfo, at: Point | null, place: Place, from: string | null): Promise<void> {
  const why = cannotJoin(pool, session);
  if (why) {
    showToast({ message: `Couldn't add ${nameOf(session)}`, detail: why, tone: "warning" });
    return;
  }
  try {
    setPools(await poolMove(session.id, pool.id));
    if (at) await place([{ session_id: session.id, x: at.x, y: at.y }]);
    showToast({
      message: `${nameOf(session)} is in ${pool.name}`,
      undo: from ? { onUndo: async () => setPools(await poolMove(session.id, from)) } : undefined,
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

/** Takes a member out (back to its project's own pool). Undo puts it
 * back with its role and its place. */
export async function removeFromPool(pool: PoolInfo, member: MemberNode, at: Point | undefined, place: Place): Promise<void> {
  if (cannotRemove(pool)) return;
  const name = nameOf(member.session);
  try {
    setPools(await poolMove(member.id, defaultPoolIdOf(member.session.project_id)));
    showToast({
      message: `${name} left ${pool.name}`,
      detail: "It's back in its project's own pool.",
      testId: "canvas-removed",
      undo: {
        onUndo: async () => {
          setPools(await poolMove(member.id, pool.id));
          if (member.role !== "member") setPools(await poolSetRole(member.id, member.role as PoolRole));
          if (at) await place([{ session_id: member.id, x: at.x, y: at.y }]);
        },
      },
    });
  } catch (cause) {
    failed(`Couldn't take ${name} out`)(cause);
  }
}

/** Makes a member the pool's lead; the lead it had becomes a member.
 * Undo gives the roles back. */
export async function makeLead(member: MemberNode, members: MemberNode[]): Promise<void> {
  const before = members.filter((other) => other.role === "lead" && other.id !== member.id);
  const was = member.role as PoolRole;
  try {
    for (const lead of before) await poolSetRole(lead.id, "member");
    setPools(await poolSetRole(member.id, "lead"));
    showToast({
      message: `${nameOf(member.session)} leads now`,
      detail: before.length > 0 ? `${before.map((lead) => nameOf(lead.session)).join(", ")} ${before.length === 1 ? "is a member" : "are members"} again.` : undefined,
      undo: {
        onUndo: async () => {
          setPools(await poolSetRole(member.id, was));
          for (const lead of before) setPools(await poolSetRole(lead.id, "lead"));
        },
      },
    });
  } catch (cause) {
    failed("Couldn't change the lead")(cause);
  }
}

/** Sets a member's role (member or observer). */
export async function setRole(member: MemberNode, role: PoolRole): Promise<void> {
  try {
    setPools(await poolSetRole(member.id, role));
  } catch (cause) {
    failed("Couldn't change its role")(cause);
  }
}
