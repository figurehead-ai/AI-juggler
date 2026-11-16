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
export const ownPoolName = (project: string | undefined) => `${project ?? "This project"}'s own pool`;

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
