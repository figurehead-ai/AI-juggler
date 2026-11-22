/**
 * The 0.2 walk (F10): a project's Tasks and Messages, one click from its
 * page. Every project has its own pool, holding all its sessions (D-33);
 * its Tasks, Messages and Agent access are that pool's page's tabs. Before
 * this, they opened only from a Needs you card, or from a pool you made.
 *
 * R4 (Priya, "make sure nothing from Client A's work can reach the
 * others"; main's rulings): she looked here and never found "Keep it
 * apart…", which only the project's "…" menu had; and "on their own: Work
 * together" read as the opposite of what she wanted. The card says first
 * whether the project is kept apart, with the way to do it, then what's
 * true of its agents, the level's name second.
 */
import type { JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { useActionHost } from "../../components/ItemMenu";
import { Button } from "../../components/primitives";
import { IconLayers } from "../../components/usage/controls";
import { apartPool } from "../../lib/keep-apart";
import { usePools } from "../../lib/pools";
import { agentsHereLine } from "../../lib/pool-words";
import { rememberTab, type PageTabId } from "../registry";
import type { PoolInfo } from "../../lib/ipc/pools";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";

/** The project's own pool, once the pools are read. */
export function projectPool(pools: readonly PoolInfo[] | undefined, projectId: string): PoolInfo | null {
  return pools?.find((pool) => pool.kind === "default" && pool.project_id === projectId) ?? null;
}

const LINKS: { tab: PageTabId; label: string; words: string }[] = [
  { tab: "tasks", label: "Tasks", words: "what the agents here plan and hold" },
  { tab: "messages", label: "Messages", words: "what they send each other" },
  { tab: "access", label: "Agent access", words: "what they may do" },
];

export function ProjectAgentsCard({ project }: { project: ProjectInfo }): JSX.Element | null {
  const pools = usePools();
  const host = useActionHost();
  const pool = projectPool(pools?.pools, project.id);
  if (!pool || !host.openPool) return null;
  const apart = apartPool(pools?.pools, project.id);
  const open = (tab: PageTabId) => {
    rememberTab("pool", tab);
    host.openPool!(pool.id);
  };
  return (
    <Section title="Agents" icon={<IconLayers />} sub={agentsHereLine(pool)} testId="project-agents">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 pb-1" data-testid="project-agents-apart" data-apart={apart ? "yes" : "no"}>
        <p className="min-w-0 flex-1 text-body text-ink">
          <span className="font-medium">Kept apart: {apart ? "yes" : "no"}</span>
          <span className="text-meta text-muted">
            {apart ? ` · in “${apart.name}”, its sessions are in no other pool, and nothing leaves it through AI Juggler` : " · its sessions can be in your other pools too"}
          </span>
        </p>
        {apart ? (
          <Button size="sm" variant="secondary" onClick={() => host.openPool!(apart.id)} data-testid="project-agents-apart-open">
            Open it ›
          </Button>
        ) : (
          host.keepProjectApart && (
            <Button size="sm" variant="secondary" onClick={() => host.keepProjectApart!(project)} data-testid="project-agents-keep-apart">
              Keep it apart…
            </Button>
          )
        )}
      </div>
      <ul className="flex flex-col gap-1.5">
        {LINKS.map((link) => (
          <li key={link.tab}>
            <button
              type="button"
              onClick={() => open(link.tab)}
              className="flex w-full items-center gap-2.5 rounded-lg border border-hairline bg-surface-2 px-3 py-2 text-left text-body text-ink outline-none hover:border-accent/40 focus-visible:ring-2 focus-visible:ring-accent/60"
              data-testid={`project-agents-${link.tab}`}
            >
              <span className="font-medium">{link.label}</span>
              <span className="min-w-0 flex-1 truncate text-meta text-muted">{link.words}</span>
              <span aria-hidden="true" className="text-muted">
                ›
              </span>
            </button>
          </li>
        ))}
      </ul>
    </Section>
  );
}
