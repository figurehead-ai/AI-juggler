/**
 * The 0.2 walk (F10): a project's Tasks and Messages, one click from its
 * page. Every project has its own pool, holding all its sessions (D-33);
 * its Tasks, Messages and Agent access are that pool's page's tabs. Before
 * this, they opened only from a Needs you card, or from a pool you made.
 */
import type { JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { useActionHost } from "../../components/ItemMenu";
import { IconLayers } from "../../components/usage/controls";
import { levelLabel, usePools } from "../../lib/pools";
import { rememberTab, type PageTabId } from "../registry";
import type { PoolInfo } from "../../lib/ipc/pools";

/** The project's own pool, once the pools are read. */
export function projectPool(pools: readonly PoolInfo[] | undefined, projectId: string): PoolInfo | null {
  return pools?.find((pool) => pool.kind === "default" && pool.project_id === projectId) ?? null;
}

const LINKS: { tab: PageTabId; label: string; words: string }[] = [
  { tab: "tasks", label: "Tasks", words: "what the agents here plan and hold" },
  { tab: "messages", label: "Messages", words: "what they send each other" },
  { tab: "access", label: "Agent access", words: "what they may do" },
];

export function ProjectAgentsCard({ projectId }: { projectId: string }): JSX.Element | null {
  const pools = usePools();
  const host = useActionHost();
  const pool = projectPool(pools?.pools, projectId);
  if (!pool || !host.openPool) return null;
  const open = (tab: PageTabId) => {
    rememberTab("pool", tab);
    host.openPool!(pool.id);
  };
  return (
    <Section title="Agents" icon={<IconLayers />} sub={`its own pool · agents may: ${levelLabel(pool.access.level)}`} testId="project-agents">
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
