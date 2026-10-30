/**
 * 0.3 — Pools, for now: every pool as a plain list, each opening its
 * page. A placeholder kept small on purpose: the pools builder (branch
 * pools-0.3) replaces this file with the real screen.
 */
import type { JSX } from "react";
import { Page } from "../../components/Page";
import { IconPool } from "../../components/icons-actions";
import { useActionHost } from "../../components/ItemMenu";
import { usePools } from "../../lib/pools";

export function PoolsScreen(): JSX.Element {
  const host = useActionHost();
  const pools = usePools()?.pools ?? null;
  return (
    <Page title="Pools" subtitle="Sessions that work together, each pool with its members, tasks and messages." testId="pools-screen">
      <ul className="flex flex-col divide-y divide-hairline/70" aria-label="Pools" aria-busy={pools === null}>
        {(pools ?? []).map((pool) => (
          <li key={pool.id}>
            <button type="button" onClick={() => host.openPool?.(pool.id)} className="flex w-full items-center gap-3 rounded-lg px-3 py-3 text-left outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="pools-row">
              <IconPool width={16} height={16} aria-hidden="true" className="shrink-0 text-muted" />
              <span className="min-w-0 flex-1 truncate text-body font-medium text-ink">{pool.name}</span>
              <span className="text-meta text-muted">{pool.members === 1 ? "1 member" : `${pool.members} members`}</span>
            </button>
          </li>
        ))}
      </ul>
    </Page>
  );
}
