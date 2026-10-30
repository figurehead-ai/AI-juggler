/**
 * The Pools screen (redesign §5): every pool as a card — its canvas in
 * small, its name (a project's own pool says so), how many are in it,
 * what waits on you and how many tasks are moving. A card opens the
 * pool's canvas; New pool opens an empty one, its name ready to type.
 *
 * Instant and paged: the pools are the app's one copy (shown at once when
 * known, a skeleton before); a page of cards at a time, and only the cards
 * shown read their positions and tasks.
 */
import { useEffect, useMemo, useState, type JSX } from "react";

import { Page, PageGrid } from "../../components/Page";
import { Button } from "../../components/primitives";
import { IconPlus } from "../../components/icons-actions";
import { useActionHost } from "../../components/ItemMenu";
import { showToast } from "../../components/Toast";
import { errorText, type InstanceInfo, type ProjectInfo } from "../../lib/bridge";
import { useMessageNeeds } from "../../lib/ipc/messages";
import { onTasksChanged, useTaskNeeds } from "../../lib/ipc/tasks";
import { poolCanvasPreviews, type PoolPreviewInfo } from "../../lib/ipc/pool-canvas";
import { poolCreate, type PoolInfo } from "../../lib/ipc/pools";
import { poolIdOf, setPools, usePools } from "../../lib/pools";
import { sessionState } from "../../lib/rail-items";
import { editNameOf } from "../../pages/pool/PoolHeader";
import { PoolMini, type MiniNode } from "./PoolMini";

/** Cards a page shows; Show more adds as many. */
export const CARDS_PAGE = 12;

/** A name for a new pool no other pool has ("New pool", "New pool 2"…). */
export function newPoolName(taken: string[]): string {
  const names = new Set(taken.map((name) => name.toLowerCase()));
  for (let n = 1; ; n += 1) {
    const name = n === 1 ? "New pool" : `New pool ${n}`;
    if (!names.has(name.toLowerCase())) return name;
  }
}

/** The cards and their order: your pools (oldest first), then the
 * projects' pools with open sessions, then the rest that hold any; a
 * project's pool with no one in it waits until it has someone. */
export function cardOrder(pools: PoolInfo[], open: ReadonlyMap<string, number>): PoolInfo[] {
  const rank = (pool: PoolInfo) => (pool.kind === "user" ? 0 : (open.get(pool.id) ?? 0) > 0 ? 1 : 2);
  return pools
    .map((pool, index) => ({ pool, index }))
    .filter(({ pool }) => pool.kind === "user" || pool.members > 0 || (open.get(pool.id) ?? 0) > 0)
    .sort((a, b) => rank(a.pool) - rank(b.pool) || a.index - b.index)
    .map(({ pool }) => pool);
}

/** A card's line: "3 members · 1 needs you · 2 tasks moving", or quiet. */
export function cardWords(members: number, asks: number, moving: number): string {
  const parts = [`${members} ${members === 1 ? "member" : "members"}`];
  if (asks > 0) parts.push(`${asks} ${asks === 1 ? "needs" : "need"} you`);
  if (moving > 0) parts.push(`${moving} ${moving === 1 ? "task" : "tasks"} moving`);
  if (asks === 0 && moving === 0 && members > 0) parts.push("quiet");
  return parts.join(" · ");
}

function SkeletonCards(): JSX.Element {
  return (
    <PageGrid min={280} testId="pools-loading">
      {Array.from({ length: 3 }, (_, n) => (
        <div key={n} className="flex flex-col gap-3 rounded-card border border-hairline bg-surface p-3" role="status" aria-label="Reading the pools">
          <span className="h-[92px] rounded-lg bg-surface-3 motion-safe:animate-pulse" />
          <span className="h-4 w-2/3 rounded bg-surface-3 motion-safe:animate-pulse" />
          <span className="h-3 w-1/2 rounded bg-surface-3 motion-safe:animate-pulse" />
        </div>
      ))}
    </PageGrid>
  );
}

export function PoolsScreen({ sessions, projects }: { sessions: InstanceInfo[]; projects: ProjectInfo[] }): JSX.Element {
  const pools = usePools();
  const host = useActionHost();
  const messageNeeds = useMessageNeeds();
  const taskNeeds = useTaskNeeds();
  const [shown, setShown] = useState(CARDS_PAGE);
  const [busy, setBusy] = useState(false);
  const [previews, setPreviews] = useState<Map<string, PoolPreviewInfo>>(new Map());

  // Each open session (what the rail holds), by its pool.
  const byPool = useMemo(() => {
    const out = new Map<string, InstanceInfo[]>();
    for (const session of sessions) {
      if (sessionState(session).state === "ended") continue;
      const id = poolIdOf(pools, session);
      out.set(id, [...(out.get(id) ?? []), session]);
    }
    return out;
  }, [sessions, pools]);
  const open = useMemo(() => new Map([...byPool].map(([id, list]) => [id, list.length])), [byPool]);
  const ordered = useMemo(() => cardOrder(pools?.pools ?? [], open), [pools, open]);
  const visible = ordered.slice(0, shown);
  const visibleKey = visible.map((pool) => pool.id).join("|");
  // Positions and tasks moving, for the cards shown; again when tasks move.
  useEffect(() => {
    if (!visibleKey) return;
    let alive = true;
    const read = () =>
      void poolCanvasPreviews(visibleKey.split("|")).then(
        (rows) => alive && setPreviews(new Map(rows.map((row) => [row.pool_id, row]))),
        () => undefined,
      );
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [visibleKey]);

  const asksOf = (poolId: string) =>
    messageNeeds.filter((need) => need.pool_id === poolId && (need.kind === "ask" || need.kind === "deliver" || need.kind === "question")).length +
    taskNeeds.filter((need) => need.pool_id === poolId).length +
    (byPool.get(poolId) ?? []).filter((session) => sessionState(session).needs).length;

  const create = async () => {
    setBusy(true);
    try {
      const name = newPoolName((pools?.pools ?? []).map((pool) => pool.name));
      const info = await poolCreate(name, null);
      setPools(info);
      const made = info.pools.find((pool) => pool.name === name);
      if (made) {
        editNameOf(made.id);
        host.openPool?.(made.id);
      }
    } catch (cause) {
      showToast({ message: "Couldn't make a pool", detail: errorText(cause), tone: "error" });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Page
      title="Pools"
      subtitle="Sessions that work together. Open one to watch it, answer it and change it on its canvas."
      actions={
        <Button variant="primary" onClick={() => void create()} disabled={busy} data-testid="pools-new">
          <IconPlus width={15} height={15} aria-hidden="true" className="mr-1.5" />
          New pool
        </Button>
      }
      testId="pools-screen"
    >
      {pools === null ? (
        <SkeletonCards />
      ) : (
        <>
          <PageGrid min={280} testId="pools-cards">
            {visible.map((pool) => {
              const members = byPool.get(pool.id) ?? [];
              const preview = previews.get(pool.id);
              const placed = new Map((preview?.positions ?? []).map((row) => [row.session_id, { x: row.x, y: row.y }]));
              const nodes: MiniNode[] = members.slice(0, 24).map((session) => {
                const state = sessionState(session);
                const role = pools.placements.find((row) => row.session_id === session.id)?.role ?? "member";
                return { id: session.id, role, alert: state.needs, working: state.state === "working" };
              });
              const asks = asksOf(pool.id);
              const project = pool.project_id ? projects.find((row) => row.id === pool.project_id) : undefined;
              return (
                <button
                  key={pool.id}
                  type="button"
                  onClick={() => host.openPool?.(pool.id)}
                  className={"group flex flex-col gap-2.5 rounded-card border bg-surface p-3 text-left outline-none transition-colors hover:border-accent/60 focus-visible:ring-2 focus-visible:ring-accent/60 " + (asks > 0 ? "border-warning/60" : "border-hairline")}
                  data-testid="pool-card-tile"
                  data-pool={pool.id}
                >
                  <PoolMini nodes={nodes} placed={placed} />
                  <span className="flex min-w-0 items-baseline gap-2">
                    <span className="truncate text-body font-semibold text-ink">{pool.kind === "default" ? (project?.name ?? "No project") : pool.name}</span>
                    {pool.kind === "default" && <span className="shrink-0 text-label text-muted">{pool.project_id ? "project pool" : "sessions with no project"}</span>}
                  </span>
                  <span className={"text-meta " + (asks > 0 ? "text-warning" : "text-muted")} data-testid="pool-card-words">
                    {cardWords(pool.members, asks, preview?.moving ?? 0)}
                  </span>
                </button>
              );
            })}
          </PageGrid>
          {ordered.length > shown && (
            <Button variant="ghost" className="self-start" onClick={() => setShown((was) => was + CARDS_PAGE)} data-testid="pools-more">
              Show {Math.min(CARDS_PAGE, ordered.length - shown)} more of {ordered.length - shown}
            </Button>
          )}
        </>
      )}
    </Page>
  );
}
