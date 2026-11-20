/**
 * AJ-E3011 round 5 (Sam's diagram path, SD 4/7: a question card, a
 * checklist, the diagram and a cramped side panel at once): an empty
 * pool's page is its guided setup — New pool's own steps, the same
 * component, in place, prefilled with the pool's projects (and its name;
 * a project's own pool keeps its scope) — with "Show the diagram" beside
 * its title. Set up, or with members, the pool opens on its diagram.
 */
import type { JSX } from "react";

import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { PoolRuleInfo } from "../../lib/ipc/pool-model";
import type { PoolInfo } from "../../lib/ipc/pools";
import type { PoolRules } from "../../lib/pool-rules";
import { asRole } from "../../lib/pool-words";
import { NewPool } from "../../screens/pools/NewPool";
import type { ExistingPool } from "../../screens/pools/NewPoolFrame";

/** An empty pool as its setup starts: its projects, its everyone
 * member's role, and (a project's own) its fixed scope. */
export function existingOf(pool: Pick<PoolInfo, "kind" | "scope_all" | "scope" | "project_id">, rules: readonly PoolRuleInfo[]): ExistingPool {
  const everyone = rules.find((rule) => rule.member === "all");
  return {
    every: pool.scope_all,
    projects: pool.kind === "default" ? [pool.project_id] : [...pool.scope],
    everyoneRole: everyone ? asRole(everyone.role) : null,
    fixedScope: pool.kind === "default",
  };
}

/** A project's own pool's name, as its setup says it. */
const ownPoolName = (project: string | undefined) => `${project ?? "This project"}'s own pool`;

export function EmptyPoolSetup({ pool, rules, projects, sessions, accounts, onDiagram }: { pool: PoolInfo; rules: PoolRules; projects: ProjectInfo[]; sessions: InstanceInfo[]; accounts: AccountInfo[]; onDiagram: () => void }): JSX.Element {
  const project = pool.project_id ? projects.find((one) => one.id === pool.project_id) : undefined;
  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-card border border-hairline bg-surface" data-testid="pool-page" data-setup="true">
      <NewPool
        embedded
        sessions={sessions}
        projects={projects}
        accounts={accounts}
        project={pool.project_id}
        into={{ id: pool.id, name: pool.kind === "default" ? ownPoolName(project?.name) : pool.name }}
        existing={existingOf(pool, rules.rules?.members ?? [])}
        onClose={() => {
          onDiagram();
          rules.reload();
        }}
      />
    </div>
  );
}

/** Round 6 (Sam: "Open the pool" showed a blank panel for ~3 s): the
 * setup's own frame at once while the pool is read — its title, its
 * steps and a step's place — so nothing is blank and nothing jumps. */
export function SetupSkeleton({ title }: { title: string }): JSX.Element {
  const bar = "rounded-md bg-surface-3 motion-safe:animate-pulse";
  return (
    <output aria-label="Reading the pool" className="flex h-full min-h-0 flex-col overflow-hidden rounded-card border border-hairline bg-surface" data-testid="pool-page-loading">
      <span className="flex items-center gap-3 bg-surface px-5 py-3">
        <span className="text-title font-semibold text-ink">{title}</span>
      </span>
      <span className="flex min-h-0 flex-1">
        <span className="flex w-60 shrink-0 flex-col gap-2 bg-surface p-3">
          {Array.from({ length: 8 }, (_, at) => (
            <span key={at} className={bar + " h-7"} />
          ))}
        </span>
        <span className="flex min-w-0 flex-1 flex-col gap-3 bg-canvas px-6 py-5">
          <span className={bar + " h-5 w-48"} />
          <span className={bar + " h-4 w-80"} />
          <span className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {Array.from({ length: 3 }, (_, at) => (
              <span key={at} className={bar + " h-28"} />
            ))}
          </span>
        </span>
      </span>
    </output>
  );
}
