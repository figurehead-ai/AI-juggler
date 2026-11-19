/**
 * 0.3 — a project's Pools tab (AJ-E3104: it was Pool): its own pool in
 * brief — the sessions in it and their states, what waits on you there —
 * and "Open the pool ›" to the pool's page, where it's watched and
 * changed. Then every other pool its sessions are in, and the pools you
 * made for it, each one click away.
 */
import { useCallback, useMemo, type JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { PoolCardRows } from "../../components/PoolCards";
import { IconPool } from "../../components/icons-actions";
import { Button } from "../../components/primitives";
import { useActionHost } from "../../components/ItemMenu";
import { defaultPoolIdOf, requestNewPool, usePoolMembers, usePools } from "../../lib/pools";
import { agentsHereLine } from "../../lib/pool-words";
import { useTaskNeeds } from "../../lib/ipc/tasks";
import { sessionState } from "../../lib/rail-items";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { PoolInfo } from "../../lib/ipc/pools";
import { RelatedPools, usePagePools } from "../../components/RelatedPools";
import { useRelationCounts } from "../../lib/relation-counts";

export function ProjectPool({ project, sessions, now }: { project: ProjectInfo; sessions: InstanceInfo[]; now: number }): JSX.Element {
  const host = useActionHost();
  const poolId = defaultPoolIdOf(project.id);
  const pools = usePools()?.pools ?? [];
  const pool = pools.find((entry) => entry.id === poolId);
  // Its other pools: the ones its sessions are in, and yours for it.
  const mine = useMemo(() => sessions.filter((s) => s.project_id === project.id), [sessions, project.id]);
  const forIt = useCallback((entry: PoolInfo) => entry.kind === "user" && entry.project_id === project.id, [project.id]);
  const others = usePagePools("project", project.id, mine, forIt)?.filter((entry) => entry.id !== poolId);
  const list = usePoolMembers(poolId);
  const waiting = useTaskNeeds().filter((need) => need.pool_id === poolId);
  const counted = useRelationCounts("pool", poolId);
  const summary = poolStates(
    list.items.map((row) => {
      const { state, word } = sessionState(sessions.find((s) => s.id === row.session.id) ?? row.session);
      return { word, ended: state === "ended" };
    }),
    counted?.sessions.ended ?? null,
  );
  const open = (id: string) => host.openPool?.(id);
  return (
    <div className="flex w-full max-w-content flex-col gap-grid" data-testid="project-pool">
      {/* Round 6 (Casey's run): "New pool for web…" sat under the own
          pool's rows, which load late and pushed it down under the
          pointer: a click landed on a session. It's at the tab's top now,
          where nothing that loads later moves it. */}
      <div className="flex justify-end">
        <Button size="sm" variant="secondary" onClick={() => newPoolFor(project.id)} data-testid="project-pool-new">
          New pool for {project.name}…
        </Button>
      </div>
      {/* AJ-E3011 §10: a project's own pool says what it is, apart from the
          pools you made. */}
      <Section
        title={`${project.name}'s own pool`}
        icon={<IconPool width={15} height={15} />}
        sub={[`every session in ${project.name}, as one team`, pool ? `${pool.members} ${pool.members === 1 ? "session" : "sessions"}` : null, summary || null, pool ? agentsHereLine(pool) : null].filter(Boolean).join(" · ")}
        actions={
          // Round 3 (#7): a click before the pools are read went nowhere;
          // until then the button waits, saying so, with a spinner.
          <Button size="sm" variant="secondary" disabled={!pool} aria-busy={!pool || undefined} title={pool ? undefined : "Reading the pool…"} onClick={() => open(poolId)} data-testid="project-pool-open">
            {!pool && <span aria-hidden="true" className="inline-block size-3 rounded-full border-2 border-muted border-t-transparent motion-safe:animate-spin" data-testid="project-pool-open-wait" />}
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
          <p className="text-meta text-ink-2">Answer these in Needs you on Home, or on the pool's page.</p>
        </Section>
      )}
      {/* AJ-E3011 §1: the project's door to the same guided setup as New
          pool, its project filled in. */}
      <Section title="Its other pools" sub="pools of yours that hold its sessions, or were made for it" testId="project-pool-yours">
        <RelatedPools
          pools={others}
          testId="project-other-pools"
          inSection
          teach={{
            title: "No pool of yours holds this project's sessions yet",
            body: "Its sessions are a team in its own pool, above. New pool sets up one of your own, step by step: who's in, who leads, who checks.",
          }}
        />
      </Section>
    </div>
  );
}

/** The guided setup (Pools › New pool), on `project`. */
function newPoolFor(project: string): void {
  requestNewPool({ template: "blank", project });
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "pools" } }));
}

/** Each state word, counted: "2 working · 1 asks you · 498 ended". The
 * live ones from the members' first page (open ones come first); the
 * ended ones from the pool's count once it's known, not that page's
 * (walk K2: a 500-session pool said "50 ended"). An ended row is skipped
 * by its state, whatever its word says of how it ended (the 0.4.6 RC:
 * "500 sessions · 50 finished · 500 ended", its first page's "Finished"
 * counted beside the pool's ended ones). */
export function poolStates(rows: readonly { word: string; ended: boolean }[], ended: number | null): string {
  const states = new Map<string, number>();
  for (const row of rows) {
    if (ended !== null && row.ended) continue;
    states.set(row.word, (states.get(row.word) ?? 0) + 1);
  }
  if (ended !== null && ended > 0) states.set("Ended", ended);
  return [...states].map(([word, n]) => `${n} ${word.toLowerCase()}`).join(" · ");
}
