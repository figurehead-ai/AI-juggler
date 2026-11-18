/**
 * The Pools screen (redesign §5): every pool as a card — its canvas in
 * small, its name (a project's own pool says so), how many are in it,
 * what waits on you and how many tasks are moving. A card opens the
 * pool's canvas; New pool opens the guided setup (AJ-E3011).
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
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { useMessageNeeds } from "../../lib/ipc/messages";
import { onTasksChanged, useTaskNeeds } from "../../lib/ipc/tasks";
import { poolCanvasPreviews, type PoolPreviewInfo } from "../../lib/ipc/pool-canvas";
import { poolRules, type PoolRulesInfo } from "../../lib/ipc/pool-model";
import { useApps, type AppModel } from "../../lib/apps";
import { shortName } from "../../lib/session-titles";
import type { PoolInfo } from "../../lib/ipc/pools";
import { doneNewPoolRequest, needWords, onNewPoolRequest, poolIdsOf, poolNeedCount, roleOf, scopeLabel, takeNewPoolRequest, usePools, warmPool, type NewPoolRequest } from "../../lib/pools";
import { sessionState } from "../../lib/rail-items";
import { NewPool } from "./NewPool";
import { miniTeam, PoolMini } from "./PoolMini";
import { DeletePool, PoolCard, RenamePool } from "./PoolCard";

/** X2: how many of the first cards' pools are read ahead, and when. */
const WARM_FIRST = 4;
const WARM_AFTER_MS = 250;

/** Cards a page shows; Show more adds as many (AJ-E2807 R2: paged past 24). */
export const CARDS_PAGE = 24;

/** A name for a new pool no other pool has ("New pool", "New pool 2"…). */
export function newPoolName(taken: string[]): string {
  const names = new Set(taken.map((name) => name.toLowerCase()));
  for (let n = 1; ; n += 1) {
    const name = n === 1 ? "New pool" : `New pool ${n}`;
    if (!names.has(name.toLowerCase())) return name;
  }
}

/** The cards and their order (AJ-E2807 R2): the pools that ask you
 * first, then by their latest activity; then your pools, the projects'
 * pools with open sessions, the rest that hold any. A project's pool with
 * no one in it waits until it has someone. */
export function cardOrder(
  pools: PoolInfo[],
  open: ReadonlyMap<string, number>,
  asks: ReadonlyMap<string, number> = new Map(),
  recent: ReadonlyMap<string, number> = new Map(),
): PoolInfo[] {
  const rank = (pool: PoolInfo) => (pool.kind === "user" ? 0 : (open.get(pool.id) ?? 0) > 0 ? 1 : 2);
  const asking = (pool: PoolInfo) => ((asks.get(pool.id) ?? 0) > 0 ? 0 : 1);
  return pools
    .map((pool, index) => ({ pool, index }))
    .filter(({ pool }) => pool.kind === "user" || pool.members > 0 || (open.get(pool.id) ?? 0) > 0)
    .sort((a, b) => asking(a.pool) - asking(b.pool) || (recent.get(b.pool.id) ?? 0) - (recent.get(a.pool.id) ?? 0) || rank(a.pool) - rank(b.pool) || a.index - b.index)
    .map(({ pool }) => pool);
}

/** A card's line: "3 sessions · 1 needs you · 2 tasks moving", or quiet.
 * It counts the pool's sessions (`PoolInfo.members`), as its header does
 * ("3 sessions open in AI Juggler"): a pool's members are its rows (an
 * Account, an App), and the rc33 P5 walk read "0 members" on a pool whose
 * Account member was there, with no session started yet. */
export function cardWords(sessions: number, asks: number, moving: number): string {
  const parts = [`${sessions} ${sessions === 1 ? "session" : "sessions"}`];
  if (asks > 0) parts.push(needWords(asks));
  if (moving > 0) parts.push(`${moving} ${moving === 1 ? "task" : "tasks"} moving`);
  if (asks === 0 && moving === 0 && sessions > 0) parts.push("quiet");
  return parts.join(" · ");
}

function SkeletonCards(): JSX.Element {
  return (
    <PageGrid min={280} testId="pools-loading">
      {Array.from({ length: 3 }, (_, n) => (
        <div key={n} className="flex flex-col gap-3 rounded-card border border-hairline bg-surface p-3" role="status" aria-label="Reading the pools">
          <span className="h-[92px] rounded-lg bg-skeleton motion-safe:animate-pulse" />
          <span className="h-4 w-2/3 rounded bg-skeleton motion-safe:animate-pulse" />
          <span className="h-3 w-1/2 rounded bg-skeleton motion-safe:animate-pulse" />
        </div>
      ))}
    </PageGrid>
  );
}

/** A pool member's name on its card's thumbnail: a session's title, an
 * App's name. */
function memberNameOf(apps: readonly AppModel[] | null): (ref: string, sessions: readonly InstanceInfo[]) => string {
  return (ref, sessions) => {
    if (ref.startsWith("session:")) {
      const session = sessions.find((one) => `session:${one.id}` === ref);
      return session ? shortName(session) : "";
    }
    return apps?.find((app) => `app:${app.id}` === ref)?.name ?? "";
  };
}

export function PoolsScreen({ sessions, projects, accounts }: { sessions: InstanceInfo[]; projects: ProjectInfo[]; accounts: AccountInfo[] }): JSX.Element {
  const pools = usePools();
  const host = useActionHost();
  const messageNeeds = useMessageNeeds();
  const taskNeeds = useTaskNeeds();
  const [shown, setShown] = useState(CARDS_PAGE);
  const [wizard, setWizard] = useState<NewPoolRequest | false>(false);
  // A project's "Keep it apart…" (and any other door) opens it here.
  useEffect(() => {
    const take = () => {
      const request = takeNewPoolRequest();
      if (request) setWizard(request);
    };
    take();
    return onNewPoolRequest(take);
  }, []);
  const [previews, setPreviews] = useState<Map<string, PoolPreviewInfo>>(new Map());
  // D-126 §4d: each shown card's team (its members, their open sessions).
  const [teams, setTeams] = useState<Map<string, PoolRulesInfo>>(new Map());
  const apps = useApps(accounts);
  const memberName = memberNameOf(apps);

  // Each open session (what the rail holds), by its pool.
  const byPool = useMemo(() => {
    const out = new Map<string, InstanceInfo[]>();
    for (const session of sessions) {
      if (sessionState(session).state === "ended") continue;
      for (const id of poolIdsOf(pools, session)) out.set(id, [...(out.get(id) ?? []), session]);
    }
    return out;
  }, [sessions, pools]);
  const open = useMemo(() => new Map([...byPool].map(([id, list]) => [id, list.length])), [byPool]);
  const asksOf = (poolId: string) => poolNeedCount(poolId, messageNeeds, taskNeeds, byPool.get(poolId) ?? []);
  const asks = new Map((pools?.pools ?? []).map((pool) => [pool.id, asksOf(pool.id)]));
  const recent = new Map([...byPool].map(([id, list]) => [id, Math.max(0, ...list.map((row) => row.last_activity_ms ?? row.started_at_ms))]));
  const ordered = cardOrder(pools?.pools ?? [], open, asks, recent);
  const [renaming, setRenaming] = useState<PoolInfo | null>(null);
  const [deleting, setDeleting] = useState<PoolInfo | null>(null);
  const cardHost = { renamePool: setRenaming, deletePool: setDeleting };
  const visible = ordered.slice(0, shown);
  const visibleKey = visible.map((pool) => pool.id).join("|");
  // Positions and tasks moving, for the cards shown; again when tasks move.
  useEffect(() => {
    if (!visibleKey) return;
    let alive = true;
    const ids = visibleKey.split("|");
    const read = () => {
      void poolCanvasPreviews(ids).then(
        (rows) => alive && setPreviews(new Map(rows.map((row) => [row.pool_id, row]))),
        () => undefined,
      );
      void Promise.all(ids.map((id) => poolRules(id).catch(() => null))).then((rows) => alive && setTeams(new Map(rows.flatMap((row) => (row ? [[row.pool_id, row] as const] : [])))));
    };
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [visibleKey]);

  // X2: the first cards' pools are read ahead once the screen has painted,
  // in idle time and one after another, so a click paints at once without
  // a hover first (a keyboard, a quick click).
  useEffect(() => {
    const first = visibleKey ? visibleKey.split("|").slice(0, WARM_FIRST) : [];
    const timer = window.setTimeout(() => {
      void first.reduce<Promise<void>>((before, id) => before.then(() => warmPool(id)), Promise.resolve());
    }, WARM_AFTER_MS);
    return () => window.clearTimeout(timer);
  }, [visibleKey]);

  return (
    <Page
      title="Pools"
      subtitle="Teams of your AI sessions. Open one to watch it, answer it and change it on its diagram."
      actions={
        <Button variant="primary" onClick={() => setWizard({ template: "blank", project: null })} data-testid="pools-new">
          <IconPlus width={15} height={15} aria-hidden="true" className="mr-1.5" />
          New pool
        </Button>
      }
      testId="pools-screen"
    >
      {wizard && (
        <NewPool
          sessions={sessions}
          projects={projects}
          accounts={accounts}
          project={wizard.project}
          startWith={wizard.template}
          fromProject={wizard.project !== null && wizard.template === "blank"}
          onClose={() => {
            doneNewPoolRequest();
            setWizard(false);
          }}
        />
      )}
      {pools === null ? (
        <SkeletonCards />
      ) : (
        <>
          <PageGrid min={280} fill testId="pools-cards">
            {visible.map((pool) => {
              const members = byPool.get(pool.id) ?? [];
              const preview = previews.get(pool.id);
              const team = miniTeam(teams.get(pool.id) ?? null, members.slice(0, 24), (id) => roleOf(pools, id, pool.id), { apps: apps ?? [], sessions: members }, (ref) => memberName(ref, members));
              const project = pool.project_id ? projects.find((row) => row.id === pool.project_id) : undefined;
              return (
                <PoolCard key={pool.id} pool={pool} local={cardHost} asks={asks.get(pool.id) ?? 0} onOpen={() => host.openPool?.(pool.id)}>
                  <PoolMini lead={team.lead} members={team.members} known={teams.has(pool.id)} />
                  <span className="flex min-w-0 items-baseline gap-2 pr-6">
                    <span className="truncate text-body font-semibold text-ink" data-testid="pool-card-name">{pool.kind === "default" ? (project?.name ?? "No project") : pool.name}</span>
                    <span className="min-w-0 truncate text-label text-muted" data-testid="pool-card-scope">
                      {pool.kind === "default" ? (pool.project_id ? "(project pool)" : "sessions with no project") : scopeLabel(pool, projects)}
                    </span>
                  </span>
                  <span className={"text-meta " + ((asks.get(pool.id) ?? 0) > 0 ? "text-warning" : "text-muted")} data-testid="pool-card-words">
                    {cardWords(pool.members, asks.get(pool.id) ?? 0, preview?.moving ?? 0)}
                  </span>
                </PoolCard>
              );
            })}
          </PageGrid>
          {ordered.length === 0 && (
            <p className="rounded-lg border border-dashed border-hairline px-4 py-6 text-center text-body text-ink-2" data-testid="pools-empty">
              {projects.length === 0 ? "Pools appear with your first project." : "No pool has anyone in it yet. A project's pool shows here once a session runs in it; New pool makes one of your own."}
            </p>
          )}
          {renaming && <RenamePool pool={renaming} onClose={() => setRenaming(null)} />}
          {deleting && <DeletePool pool={deleting} members={deleting.members} onClose={() => setDeleting(null)} />}
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
