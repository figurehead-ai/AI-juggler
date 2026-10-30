/**
 * 0.3 (the redesign: every door obvious) — a session page's doors in its
 * header's facts line ("Claude Code · Work Max · started 8h ago · Pool";
 * the 0.3 re-render moved them there from the chips, which they pushed
 * into ellipses at 1280 px): its pool, opening the pool's page (a pool of
 * yours, or its project's own pool once another session shares it; a
 * session alone there has nothing to see), and its tool, which opens
 * the tool's details in Settings › Accounts (moved here from the window's
 * top bar, which lost its title).
 */
import type { JSX } from "react";
import { useActionHost } from "../../components/ItemMenu";
import { IconPool } from "../../components/icons-actions";
import { poolById, usePoolPlace, usePools } from "../../lib/pools";
import { useTools } from "../../lib/tools";
import { ToolMark } from "../../components/ToolMark";

/** A door on the header's facts line: its icon and its words, in the
 * line's own size, ink on hover (the facts are muted). */
const DOOR =
  "inline-flex min-w-0 max-w-full items-center gap-1 whitespace-nowrap rounded text-meta text-ink/85 outline-none decoration-hairline underline-offset-2 transition-colors hover:text-ink hover:underline focus-visible:ring-2 focus-visible:ring-accent/60";

/** The facts line's separator. */
export const Dot = (): JSX.Element => (
  <span aria-hidden="true" className="text-muted">
    ·
  </span>
);

export function SessionPoolChip({ sessionId }: { sessionId: string }): JSX.Element | null {
  const host = useActionHost();
  const pools = usePools();
  const place = usePoolPlace(sessionId);
  const pool = place ? poolById(pools, place.pool_id) : undefined;
  if (!pool || !host.openPool || (pool.kind === "default" && pool.members < 2)) return null;
  return (
    <>
      <Dot />
      <button type="button" onClick={() => host.openPool?.(pool.id)} title={`Open the pool ${pool.name}`} className={DOOR} data-testid="session-pool-chip">
        <IconPool width={13} height={13} aria-hidden="true" className="shrink-0 text-muted" />
        {/* Its project's own pool is just "Pool". */}
        <span className="truncate">{pool.kind === "default" ? "Pool" : `Pool: ${pool.name}`}</span>
      </button>
    </>
  );
}

export function SessionToolChip({ tool }: { tool: string }): JSX.Element | null {
  const host = useActionHost();
  const spec = useTools().find((entry) => entry.id === tool);
  if (!spec || !host.openTool) return null;
  return (
    // Its mark and its name: the chips name the app it runs in, never the
    // tool, and a bare mark read as an unlabelled icon (the 0.3
    // re-render's review).
    <>
      <button
        type="button"
        onClick={() => host.openTool?.(spec)}
        aria-label={`About ${spec.display_name}`}
        title={`About ${spec.display_name}`}
        className={DOOR}
        data-testid="context-tool"
      >
        <ToolMark tool={tool} size={13} />
        <span className="truncate">{spec.display_name}</span>
      </button>
      <Dot />
    </>
  );
}
