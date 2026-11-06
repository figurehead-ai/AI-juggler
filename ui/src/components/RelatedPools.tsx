/**
 * E31 (AJ-E3104; relations-map §4) — a page's Pools tab: the pools its
 * sessions are in, each one click from its page. Yours first (the ones
 * you named), then the projects' own pools; each with how many sessions
 * it holds and what its agents may do. The tab's count and this list come
 * from the same relation cache (`relations_related`), so they agree; the
 * design preview works them out from the sessions it shows.
 */
import { useMemo, type JSX } from "react";
import { hasLiveBackend, type InstanceInfo } from "../lib/bridge";
import type { PoolInfo } from "../lib/ipc/pools";
import type { EntityKind } from "../lib/ipc/relations";
import { levelLabel, poolIdsOf, usePools } from "../lib/pools";
import { useRelatedIds } from "../lib/relation-counts";
import { useActionHost } from "./ItemMenu";
import { IconPool } from "./icons-actions";
import { RelatedEmpty, type Teach } from "./related/RelatedEmpty";

const NO_EXTRA = (): boolean => false;

/** The pools `kind` `id`'s sessions are in, yours first, then by name;
 * `extra` adds pools that belong here though nothing is in them yet (a
 * pool you made for a project). `undefined` until known. */
export function usePagePools(kind: EntityKind | null, id: string, local: InstanceInfo[], extra: (pool: PoolInfo) => boolean = NO_EXTRA): PoolInfo[] | undefined {
  const info = usePools();
  const related = useRelatedIds(kind, id, "pool");
  const live = hasLiveBackend();
  return useMemo(() => {
    if (!info) return undefined;
    const ids = live ? related : [...new Set(local.flatMap((session) => poolIdsOf(info, session)))];
    if (!ids) return undefined;
    const wanted = new Set(ids);
    return info.pools
      .filter((pool) => wanted.has(pool.id) || extra(pool))
      .sort((a, b) => Number(a.kind !== "user") - Number(b.kind !== "user") || a.name.localeCompare(b.name));
  }, [info, related, live, local, extra]);
}

/** `inSection`: drawn inside a section's card, so its empty lesson is a
 * line, not a card of its own. */
export function RelatedPools({ pools, teach, testId = "related-pools", inSection = false }: { pools: PoolInfo[] | undefined; teach: Teach; testId?: string; inSection?: boolean }): JSX.Element {
  const host = useActionHost();
  if (pools === undefined) return <div className="h-24" data-testid={`${testId}-loading`} />;
  if (pools.length === 0 && inSection)
    return (
      <p className="max-w-prose text-meta text-ink-2" data-testid={`${testId}-empty`}>
        <span className="font-medium text-ink">{teach.title}.</span> {teach.body}
      </p>
    );
  if (pools.length === 0) return <RelatedEmpty teach={teach} testId={`${testId}-empty`} />;
  return (
    <ul className={"flex w-full max-w-content flex-col divide-y divide-hairline/70 " + (inSection ? "" : "rounded-card border border-hairline bg-surface")} data-testid={testId}>
      {pools.map((pool) => (
        <li key={pool.id}>
          <button
            type="button"
            onClick={() => host.openPool?.(pool.id)}
            disabled={!host.openPool}
            className="flex w-full items-center gap-3 px-3 py-2.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
            data-testid="related-pool"
            data-pool-id={pool.id}
          >
            <IconPool width={15} height={15} aria-hidden="true" className="shrink-0 text-muted" />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate text-body font-medium text-ink">{pool.name}</span>
              <span className="truncate text-meta text-muted">
                {pool.kind === "user" ? "Your pool" : "Its project's own pool"} · agents: {levelLabel(pool.access.level)}
              </span>
            </span>
            <span className="shrink-0 text-meta tabular-nums text-ink-2">
              {pool.members === 1 ? "1 session" : `${pool.members} sessions`} ›
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
