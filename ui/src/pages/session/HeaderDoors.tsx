/**
 * 0.3 (the redesign: every door obvious) — a session page's doors in its
 * header: its pool, as a chip that opens the pool's page (a pool of
 * yours, or its project's own pool once another session shares it; a
 * session alone there has nothing to see), and its tool, whose chip opens
 * the tool's details in Settings › Accounts (moved here from the window's
 * top bar, which lost its title).
 */
import type { JSX } from "react";
import { useActionHost } from "../../components/ItemMenu";
import { IconPool } from "../../components/icons-actions";
import { poolById, usePoolPlace, usePools } from "../../lib/pools";
import { useTools } from "../../lib/tools";
import { ToolChip } from "../../components/ToolMark";

export function SessionPoolChip({ sessionId }: { sessionId: string }): JSX.Element | null {
  const host = useActionHost();
  const pools = usePools();
  const place = usePoolPlace(sessionId);
  const pool = place ? poolById(pools, place.pool_id) : undefined;
  if (!pool || !host.openPool || (pool.kind === "default" && pool.members < 2)) return null;
  return (
    <button
      type="button"
      onClick={() => host.openPool?.(pool.id)}
      title={`Open the pool ${pool.name}`}
      className="inline-flex min-w-0 max-w-full shrink items-center gap-1.5 whitespace-nowrap rounded-full border border-hairline bg-surface-2 px-2.5 py-0.5 text-meta leading-5 text-ink outline-none transition-colors hover:border-accent/40 hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60"
      data-testid="session-pool-chip"
    >
      <IconPool width={13} height={13} aria-hidden="true" className="shrink-0 text-muted" />
      <span className="truncate">Pool: {pool.name}</span>
    </button>
  );
}

export function SessionToolChip({ tool }: { tool: string }): JSX.Element | null {
  const host = useActionHost();
  const spec = useTools().find((entry) => entry.id === tool);
  if (!spec || !host.openTool) return null;
  return (
    <button type="button" onClick={() => host.openTool?.(spec)} aria-label={`About ${spec.display_name}`} title={`About ${spec.display_name}`} className="shrink-0 rounded-full outline-none focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="context-tool">
      <ToolChip tool={tool} tone="accent" />
    </button>
  );
}
