import { useEffect, useState, type JSX } from "react";

import { Button, Card, Chip } from "../components/primitives";
import { IconSessions } from "../components/icons";
import { PoolAccessPanel } from "../components/PoolAccess";
import { PoolCardRows } from "../components/PoolCards";
import { SettingSwitch } from "../components/SettingSwitch";
import { errorText, type InstanceInfo, type ProjectInfo } from "../lib/bridge";
import { poolCards, poolSetSpansMacs, type ContextCard } from "../lib/ipc/pools";
import { levelLabel, openPoolSheet, poolById, setPools, usePools } from "../lib/pools";
import { PageHeader, PageShell } from "./PageShell";
import { pageTabs } from "./registry";

/** How often an open pool page reads its cards again: they change on the
 * read model's 2 s tick, and a page left open needn't ask more often. */
const CARDS_EVERY_MS = 10_000;

/**
 * A pool's page (AJ-E1703): its members' context cards first, then what
 * agents may do in it (D-28) and whether it spans your paired Macs
 * (D-32). Editing the pool itself (name, members, roles) is the pool
 * sheet's, one click away.
 */
export function PoolPage({ poolId, sessions, projects, now }: { poolId: string; sessions: InstanceInfo[]; projects: ProjectInfo[]; now: number }): JSX.Element {
  const pools = usePools();
  const pool = poolById(pools, poolId);
  const [cards, setCards] = useState<ContextCard[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const members = pools?.members.filter((row) => row.pool_id === poolId).length ?? 0;

  useEffect(() => {
    let alive = true;
    const read = () =>
      void poolCards(poolId).then(
        (result) => alive && (setCards(result.cards), setError(null)),
        (cause) => alive && setError(errorText(cause)),
      );
    read();
    const timer = window.setInterval(read, CARDS_EVERY_MS);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [poolId, members]);

  if (pools !== null && !pool) {
    return (
      <Card className="m-6 p-6 text-sm text-muted" data-testid="pool-gone">
        That pool isn't there any more.
      </Card>
    );
  }
  const project = pool?.project_id ? projects.find((row) => row.id === pool.project_id) : undefined;
  return (
    <div className="flex h-full flex-col" data-testid="pool-page">
      <PageShell
        kind="pool"
        tabs={pageTabs("pool")}
        header={
          <PageHeader
            kind="pool"
            mark={<IconSessions width={18} height={18} />}
            name={pool?.name ?? "Pool"}
            chips={pool?.kind === "default" ? <Chip tone="neutral">Always here</Chip> : undefined}
            facts={
              <span>
                {members} {members === 1 ? "session" : "sessions"}
                {project ? ` · for ${project.name}` : ""}
                {pool ? ` · agents: ${levelLabel(pool.access.level)}` : ""}
              </span>
            }
            actions={
              <Button size="sm" variant="secondary" onClick={() => openPoolSheet({ mode: "pool", poolId })} data-testid="pool-page-edit">
                Edit pool
              </Button>
            }
          />
        }
        render={(tab) =>
          tab === "members" ? (
            <div className="flex flex-col gap-2">
              {error && (
                <p className="text-xs text-danger" role="alert">
                  {error}
                </p>
              )}
              {cards === null ? <p className="text-xs text-muted">Loading…</p> : <PoolCardRows cards={cards} sessions={sessions} now={now} />}
            </div>
          ) : pool ? (
            <Card className="flex flex-col gap-4 p-5">
              <PoolAccessPanel pool={pool} now={now} />
              {error && (
                <p className="text-xs text-danger" role="alert">
                  {error}
                </p>
              )}
              <SettingSwitch
                id="pool-spans-macs"
                label="Include my paired Macs"
                hint="Message and task content will travel to your other Macs."
                checked={pool.spans_macs}
                onChange={(on) => void poolSetSpansMacs(pool.id, on).then(setPools, (cause) => setError(errorText(cause)))}
              />
            </Card>
          ) : null
        }
      />
    </div>
  );
}
