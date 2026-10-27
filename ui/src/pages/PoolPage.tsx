import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";

import { Button, Card, Chip } from "../components/primitives";
import { IconSessions } from "../components/icons";
import { PoolAccessPanel } from "../components/PoolAccess";
import { AgentToolsQuestion } from "../components/AgentTools";
import { PoolCardRows } from "../components/PoolCards";
import { SettingSwitch } from "../components/SettingSwitch";
import { errorText, type InstanceInfo, type ProjectInfo } from "../lib/bridge";
import { poolSetSpansMacs } from "../lib/ipc/pools";
import { levelLabel, openPoolSheet, poolById, setPools, usePoolMembers, usePools } from "../lib/pools";
import { PageHeader, PageShell } from "./PageShell";
import { pageTabs } from "./registry";
import { lazyNamed } from "../lib/lazy";
import { PoolLinks } from "./messages/PoolLinks";

// E18: the Messages tab loads only when it opens.
const PoolMessages = lazyNamed(() => import("./messages/PoolMessages"), "PoolMessages");
// E19: and so does the Tasks tab (its code and its first read).
const PoolTasks = lazyNamed(() => import("./tasks/PoolTasks"), "PoolTasks");

/** Keeps a tab mounted once it was shown (its scroll and its open task
 * stay when you look at another tab and back). */
function TasksOnce({ onShown, children }: { onShown: () => void; children: ReactNode }): JSX.Element {
  useEffect(onShown, [onShown]);
  return <>{children}</>;
}

/** How often an open pool page reads its cards again: they change on the
 * read model's 2 s tick, and a page left open needn't ask more often. */
const CARDS_EVERY_MS = 10_000;

/**
 * A pool's page (AJ-E1703): its members' context cards first, then what
 * agents may do in it (D-28) and whether it spans your paired Macs
 * (D-32). Editing the pool itself (name, members, roles) is the pool
 * sheet's, one click away. Until it's answered, the members' tab opens
 * with the one-time question about agent tools (D-30).
 */
export function PoolPage({ poolId, sessions, projects, now }: { poolId: string; sessions: InstanceInfo[]; projects: ProjectInfo[]; now: number }): JSX.Element {
  const pools = usePools();
  const pool = poolById(pools, poolId);
  const [error, setError] = useState<string | null>(null);
  const [tasksShown, setTasksShown] = useState(false);
  const members = pool?.members ?? 0;
  // Its members a page at a time (remembered: shown at once when opened
  // again, read again behind them).
  const list = usePoolMembers(poolId);
  const refresh = list.refresh;
  useEffect(() => {
    const timer = window.setInterval(refresh, CARDS_EVERY_MS);
    return () => window.clearInterval(timer);
  }, [refresh]);
  // A session came or left (the count says so): read them again.
  const counted = useRef(members);
  useEffect(() => {
    if (counted.current === members) return;
    counted.current = members;
    refresh();
  }, [members, refresh]);
  const named = list.items.map((row) => ({ session_id: row.card.session_id, name: row.card.name }));

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
        render={(tab, visible) =>
          tab === "tasks" ? (
            visible || tasksShown ? (
              <TasksOnce onShown={() => setTasksShown(true)}>
                <PoolTasks poolId={poolId} sessions={sessions} projects={projects} now={now} />
              </TasksOnce>
            ) : null
          ) : tab === "messages" ? (
            <PoolMessages poolId={poolId} members={named} now={now} />
          ) : tab === "members" ? (
            <div className="flex flex-col gap-2">
              {/* D-30: asked once, the first time pools are used. */}
              <AgentToolsQuestion className="mb-2" />
              <PoolCardRows list={list} sessions={sessions} now={now} />
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
              <PoolLinks poolId={pool.id} />
            </Card>
          ) : null
        }
      />
    </div>
  );
}
