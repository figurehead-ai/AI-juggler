/**
 * 0.3 — a project's Pool tab: its own pool in brief — the sessions in it
 * and their states, what waits on you there — and "Open the pool ›" to
 * the pool's page, where it's watched and changed. A pool of yours for
 * this project is one click away too.
 */
import type { JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { PoolCardRows } from "../../components/PoolCards";
import { IconPool } from "../../components/icons-actions";
import { Button } from "../../components/primitives";
import { useActionHost } from "../../components/ItemMenu";
import { defaultPoolIdOf, levelLabel, usePoolMembers, usePools } from "../../lib/pools";
import { useTaskNeeds } from "../../lib/ipc/tasks";
import { sessionState } from "../../lib/rail-items";
import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";

export function ProjectPool({ project, sessions, now }: { project: ProjectInfo; sessions: InstanceInfo[]; now: number }): JSX.Element {
  const host = useActionHost();
  const poolId = defaultPoolIdOf(project.id);
  const pools = usePools()?.pools ?? [];
  const pool = pools.find((entry) => entry.id === poolId);
  const yours = pools.filter((entry) => entry.kind === "user" && entry.project_id === project.id);
  const list = usePoolMembers(poolId);
  const waiting = useTaskNeeds().filter((need) => need.pool_id === poolId);
  // Each state word, counted: "2 working · 1 asks you".
  const states = new Map<string, number>();
  for (const row of list.items) {
    const word = sessionState(sessions.find((s) => s.id === row.session.id) ?? row.session).word;
    states.set(word, (states.get(word) ?? 0) + 1);
  }
  const summary = [...states].map(([word, n]) => `${n} ${word.toLowerCase()}`).join(" · ");
  const open = (id: string) => host.openPool?.(id);
  return (
    <div className="flex w-full max-w-content flex-col gap-grid" data-testid="project-pool">
      <Section
        title={pool?.name ?? `${project.name} · all sessions`}
        icon={<IconPool width={15} height={15} />}
        sub={[pool ? `${pool.members} ${pool.members === 1 ? "session" : "sessions"}` : null, summary || null, pool ? `agents: ${levelLabel(pool.access.level)}` : null].filter(Boolean).join(" · ")}
        actions={
          <Button size="sm" variant="secondary" onClick={() => open(poolId)} data-testid="project-pool-open">
            Open the pool ›
          </Button>
        }
        testId="project-pool-members"
      >
        <PoolCardRows list={list} sessions={sessions} now={now} />
      </Section>
      {waiting.length > 0 && (
        <Section title="Waits on you" sub="from this pool's agents" testId="project-pool-waiting">
          <ul className="flex flex-col divide-y divide-hairline/70">
            {waiting.map((need) => (
              <li key={need.id} className="flex flex-wrap items-center gap-x-3 gap-y-1 py-2">
                <span className="min-w-0 flex-1 text-body text-ink">{need.title}</span>
                {need.task_title && <span className="truncate text-meta text-muted">{need.task_title}</span>}
              </li>
            ))}
          </ul>
          <p className="text-meta text-muted">Answer these in Needs you on Home, or on the pool's page.</p>
        </Section>
      )}
      {yours.length > 0 && (
        <Section title="Your pools for this project" testId="project-pool-yours">
          <ul className="flex flex-col gap-1">
            {yours.map((entry) => (
              <li key={entry.id}>
                <button type="button" onClick={() => open(entry.id)} className="flex w-full items-center gap-2.5 rounded-lg px-2 py-2 text-left text-body text-ink outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60">
                  <IconPool width={14} height={14} aria-hidden="true" className="shrink-0 text-muted" />
                  <span className="min-w-0 flex-1 truncate">{entry.name}</span>
                  <span className="text-meta text-muted">{entry.members === 1 ? "1 member" : `${entry.members} members`} ›</span>
                </button>
              </li>
            ))}
          </ul>
        </Section>
      )}
    </div>
  );
}
