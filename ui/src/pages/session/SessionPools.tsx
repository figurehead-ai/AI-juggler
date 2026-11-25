/**
 * E31 (AJ-E3104 Req.3) and D-62 (AJ-E3004): a session's Pools tab. Every
 * pool it's in, how it got into each (named on its own, through its App,
 * Account, Tool or Mac, or as everyone in its project), its role and
 * level there; then the pools that left it out, each with the reason and
 * the step that would let it in. An unreadable sign-in says so.
 */
import type { JSX } from "react";

import { Chip } from "../../components/primitives";
import { useActionHost } from "../../components/ItemMenu";
import { IconPool } from "../../components/icons-actions";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { levelLabel, roleLabel } from "../../lib/pools";
import { useSessionPoolsRead } from "./useSessionPools";
import { useReadState } from "../../lib/loading";
import { LoadingState } from "../../components/LoadingState";
import { MACHINE_WORD } from "../../lib/machine-words";

/** How it got into a pool, in words. */
export function viaWords(via: string): string {
  switch (via) {
    case "session":
      return "You put it here (a switch keeps its place)";
    case "app":
      return "Through its App: its sessions join by themselves";
    case "account":
      return "Through its Account: every App signed in to it joins";
    case "tool":
      return "Through its tool: every session of it joins";
    case "mac":
      return `Through its ${MACHINE_WORD}: every session on it joins`;
    default:
      return "Every session of its projects is in it";
  }
}

/** In no pool, and how to put it in one. */
function NoPool(): JSX.Element {
  return (
    <p className="max-w-prose text-meta text-ink-2" data-testid="session-pools-none">
      <span className="font-medium text-ink">In no pool now.</span> Add it to one from a pool's diagram, or make a pool with New pool.
    </p>
  );
}

export function SessionPools({ session }: { session: InstanceInfo }): JSX.Element {
  const host = useActionHost();
  const { info: shown, error, round, retry } = useSessionPoolsRead(session.id);
  const read = useReadState(shown, "its pools", { error, round, isEmpty: (info) => info.pools.length === 0 && info.skipped.length === 0 && !info.account_unknown });
  // A skeleton at once, never a blank tab (the P4 walk: blank for 2.5 s
  // while a busy backend read the pools); AJ-E1552: only while it reads.
  if (shown === null || read.state === "empty")
    return (
      <LoadingState
        state={read.state}
        slow={read.slow}
        what="its pools"
        skeleton={
          <output className="flex flex-col gap-2" aria-label="Reading its pools" data-testid="session-pools-loading">
            <span className="h-14 rounded-lg bg-skeleton motion-safe:animate-pulse" />
            <span className="h-14 rounded-lg bg-skeleton motion-safe:animate-pulse" />
          </output>
        }
        empty={<NoPool />}
        onRetry={retry}
      >
        {null}
      </LoadingState>
    );
  return (
    <div className="flex w-full max-w-content flex-col gap-4" data-testid="session-pools">
      {shown.pools.length === 0 ? (
        <NoPool />
      ) : (
        <ul className="flex flex-col divide-y divide-hairline/70 rounded-card border border-hairline bg-surface" data-testid="session-pools-list">
          {shown.pools.map((place) => (
            <li key={place.pool_id}>
              <button
                type="button"
                onClick={() => host.openPool?.(place.pool_id)}
                disabled={!host.openPool}
                className="flex w-full items-center gap-3 px-3 py-2.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
                data-testid="related-pool"
                data-pool-id={place.pool_id}
              >
                <IconPool width={15} height={15} aria-hidden="true" className="shrink-0 text-muted" />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="flex min-w-0 items-center gap-2">
                    <span className="truncate text-body font-medium text-ink">{place.pool_name}</span>
                    {place.sealed && <Chip tone="danger">sealed</Chip>}
                  </span>
                  <span className="truncate text-meta text-ink-2" data-testid="session-pools-how">
                    {viaWords(place.via)}
                  </span>
                </span>
                <span className="flex shrink-0 flex-col items-end gap-0.5">
                  <Chip tone={place.role === "lead" ? "accent" : place.role === "reviewer" ? "live" : "neutral"}>{roleLabel(place.role)}</Chip>
                  {place.level && <span className="text-label text-muted">on its own: {levelLabel(place.level)}</span>}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
      {shown.skipped.length > 0 && (
        <section className="flex flex-col gap-1.5" aria-label="Left out of">
          <h3 className="text-label font-semibold uppercase tracking-wide text-muted">Left out of</h3>
          <ul className="flex flex-col gap-1.5" data-testid="session-pools-skipped">
            {shown.skipped.map((skip) => (
              <li key={skip.pool_id} className="flex flex-col gap-0.5 rounded-lg border border-hairline bg-surface-2 px-3 py-2">
                <button type="button" onClick={() => host.openPool?.(skip.pool_id)} className="self-start text-body font-medium text-ink hover:underline">
                  {skip.pool_name}
                </button>
                <span className="text-meta text-warning">{skip.words}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
      {shown.account_unknown && (
        <p className="max-w-prose text-meta text-ink-2" data-testid="session-pools-account-unknown">
          <span className="font-medium text-ink">Account unknown.</span> Its sign-in can't be read now, so pools that take an Account don't take it; it stays in through any other member.
        </p>
      )}
    </div>
  );
}
