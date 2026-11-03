/**
 * E31 (AJ-E3104 Req.3) — a session's Pools tab: the pool it's in, how it
 * got there and its role there. A session is in one pool until a session
 * can be in several (AJ-E3004); then this lists each, and the ones it's
 * kept out of.
 */
import type { JSX } from "react";
import type { InstanceInfo } from "../../lib/bridge";
import { defaultPoolIdOf, poolById, usePoolPlace, usePools } from "../../lib/pools";
import { RelatedPools } from "../../components/RelatedPools";

/** How it got into `poolId`, in words. */
function howWords(moved: boolean, role: string | null): string {
  const as = role === "lead" ? " It leads there." : role === "observer" ? " It watches there, as an observer." : "";
  return moved
    ? `You put it here, or the session it continues was here (a switch keeps its place).${as}`
    : `Every session of its project is in the project's own pool until you move it.${as}`;
}

export function SessionPools({ session }: { session: InstanceInfo }): JSX.Element {
  const pools = usePools();
  const place = usePoolPlace(session.id);
  const own = defaultPoolIdOf(session.project_id);
  const poolId = place?.pool_id ?? own;
  const pool = poolById(pools, poolId);
  return (
    <div className="flex w-full max-w-content flex-col gap-3" data-testid="session-pools">
      <RelatedPools
        pools={pools ? (pool ? [pool] : []) : undefined}
        testId="session-pools-list"
        teach={{ title: "Its pool isn't there any more", body: "It's back in its project's own pool the next time the pools are read." }}
      />
      {pool && (
        <p className="max-w-prose text-meta text-ink-2" data-testid="session-pools-how">
          {howWords(poolId !== own, place?.role ?? null)}
        </p>
      )}
    </div>
  );
}
