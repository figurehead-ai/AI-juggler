import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";

import type { InstanceInfo } from "../lib/bridge";
import type { PoolsInfo } from "../lib/ipc/pools";
import { filterByPool, hasUserPools, openPoolSheet, usePools } from "../lib/pools";
import { GROUP_LABELS, type GroupBy } from "../lib/railview";
import { useActionHost } from "./ItemMenu";
import { Select } from "./RailControls";

/**
 * AJ-E1702: pools in the rail, kept out of WorkflowRail so the rail stays
 * within its size. Pools stay out of sight until you make one: then the
 * list can group by pool (and does, the first time), and Filters gains a
 * pool picker. A pool's header opens its page, like a project's.
 */
export function useRailPools(group: GroupBy, setGroup: (group: GroupBy) => void): {
  pools: PoolsInfo | null;
  /** The group to show: not "pool" once no pool is left. */
  group: GroupBy;
  groupOptions: { value: string; label: string }[];
  /** Only the chosen pool's sessions (all, with none chosen). */
  filter: (rows: InstanceInfo[]) => InstanceInfo[];
  /** A pool is picked in Filters. */
  narrowed: boolean;
  reset: () => void;
  /** The Filters pool picker, when there are pools to pick. */
  picker: ReactNode;
} {
  const pools = usePools();
  const withPools = hasUserPools(pools);
  const [poolFilter, setPoolFilter] = useState<string | null>(null);
  const choice = poolFilter !== null && pools?.pools.some((pool) => pool.id === poolFilter) ? poolFilter : null;
  // The first pool made while the list is open groups the list by pool.
  const had = useRef<boolean | null>(null);
  useEffect(() => {
    if (pools === null) return;
    if (withPools && had.current === false) setGroup("pool");
    had.current = withPools;
  }, [withPools, pools, setGroup]);
  return {
    pools,
    group: group === "pool" && !withPools ? "project" : group,
    groupOptions: GROUP_LABELS.filter((entry) => entry.id !== "pool" || withPools).map((entry) => ({ value: entry.id, label: `Group: ${entry.label}` })),
    filter: (rows) => filterByPool(rows, choice, pools),
    narrowed: choice !== null,
    reset: () => setPoolFilter(null),
    picker:
      withPools && pools ? (
        <Select
          ariaLabel="Filter by pool"
          value={choice ?? ""}
          onChange={(value) => setPoolFilter(value || null)}
          options={[{ value: "", label: "Pool: all" }, ...pools.pools.map((pool) => ({ value: pool.id, label: pool.name }))]}
        />
      ) : null,
  };
}

/** A pool group's header: opens the pool's page. */
export function RailPoolHeader({
  id,
  label,
  rollup,
  count,
  selected,
  children,
}: {
  /** The group's id (`pool:<id>`). */
  id: string;
  label: string;
  rollup: string;
  count: number;
  selected: boolean;
  children?: ReactNode;
}): JSX.Element {
  const host = useActionHost();
  const poolId = id.slice("pool:".length);
  return (
    <button
      type="button"
      onClick={() => host.openPool?.(poolId)}
      title={rollup || undefined}
      aria-current={selected ? "true" : undefined}
      className={
        "flex min-w-0 flex-1 items-center justify-between gap-2 rounded-md py-0.5 pl-1 pr-2.5 text-left text-[11.5px] font-semibold outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
        (selected ? "bg-surface text-ink" : "text-muted hover:text-ink")
      }
      data-testid="rail-pool"
    >
      <span className="min-w-0 truncate">{label}</span>
      {rollup && <span className="sr-only">· {rollup}</span>}
      <span className="flex shrink-0 items-center gap-1.5">
        {children}
        <span className="text-[10px] font-normal text-faint">{count}</span>
      </span>
    </button>
  );
}

/** Under the pool groups: make another. */
export function NewPoolLink(): JSX.Element {
  return (
    <button
      type="button"
      onClick={() => openPoolSheet({ mode: "create" })}
      className="mt-0.5 self-start rounded-md py-1 pl-7 pr-2 text-[11px] text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="rail-new-pool"
    >
      New pool
    </button>
  );
}
