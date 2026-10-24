import { useMemo, useState, type ReactNode } from "react";

import type { InstanceInfo } from "../lib/bridge";
import { filterByPool, hasUserPools, usePools } from "../lib/pools";

/** The pool Home shows, remembered between visits. */
const HOME_POOL_KEY = "aij.home.pool";

function readHomePool(): string | null {
  try {
    return localStorage.getItem(HOME_POOL_KEY);
  } catch {
    return null;
  }
}

/**
 * AJ-E1702: Home's pool filter. Out of sight until you've made a pool;
 * then a picker in Home's header shows one pool's sessions (remembered),
 * or all of them.
 */
export function useHomePool(all: InstanceInfo[]): { sessions: InstanceInfo[]; picker: ReactNode } {
  const pools = usePools();
  const [picked, setPicked] = useState<string | null>(readHomePool);
  const withPools = hasUserPools(pools);
  const choice = withPools && picked !== null && pools?.pools.some((pool) => pool.id === picked) ? picked : null;
  const sessions = useMemo(() => filterByPool(all, choice, pools), [all, choice, pools]);
  const choose = (value: string | null) => {
    setPicked(value);
    try {
      if (value === null) localStorage.removeItem(HOME_POOL_KEY);
      else localStorage.setItem(HOME_POOL_KEY, value);
    } catch {
      // Not remembered this time; the filter still applies.
    }
  };
  const picker =
    withPools && pools ? (
      <select
        aria-label="Show one pool"
        value={choice ?? ""}
        onChange={(event) => choose(event.target.value || null)}
        className="ml-auto h-7 max-w-[12rem] truncate rounded-lg border border-hairline bg-surface-2 px-2 text-[12px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
        data-testid="home-pool-filter"
      >
        <option value="">Pool: all</option>
        {pools.pools.map((pool) => (
          <option key={pool.id} value={pool.id}>
            {pool.name}
          </option>
        ))}
      </select>
    ) : null;
  return { sessions, picker };
}
