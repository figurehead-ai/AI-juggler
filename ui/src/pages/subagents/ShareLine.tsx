/**
 * AJ-E1520 (0.3), AJ-E2812 R1 — the Usage tab's line to the subagents:
 * "Subagents: 41% of this session's tokens · see which ›", which opens the
 * Subagents tab sorted by most tokens, and the three that used most, each
 * a link to its own page. Only when the session started some and their
 * tokens are recorded; their share is part of the total above, never
 * added to it, and each of the three by its own tokens (a parent's total
 * holds its children's).
 */
import { useEffect, useState, type JSX } from "react";
import { short } from "../../components/charts/scale";
import { NO_QUERY, subagentsList, type SubagentInfo } from "../../lib/ipc/subagents";
import { titleOf } from "./words";
import { IconSubtasks } from "../../components/icons";
import { openSubagents } from "../../lib/subagent-links";
import { useHeardSubagentCounts } from "../session/useSubagents";
import { shareWords } from "./words";

/** How many of the most expensive the line names. */
const MOST = 3;

/** The session's subagents that used most, by their own tokens. */
function useMostExpensive(profileId: string, conversationId: string | null, shown: boolean): SubagentInfo[] {
  const [most, setMost] = useState<SubagentInfo[]>([]);
  useEffect(() => {
    if (!shown || !conversationId) return undefined;
    let alive = true;
    subagentsList(profileId, conversationId, { cursor: null, limit: 100 }, { ...NO_QUERY, sort: "tokens" }).then(
      (info) => {
        if (!alive) return;
        const agents = info.page.items.flatMap((row) => (row.agent ? [row.agent] : []));
        const own = (agent: SubagentInfo) => agent.own_tokens ?? agent.tokens ?? 0;
        setMost(agents.filter((agent) => own(agent) > 0).sort((a, b) => own(b) - own(a)).slice(0, MOST));
      },
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [profileId, conversationId, shown]);
  return most;
}

export function SubagentsShareLine({ profileId, conversationId }: { profileId: string; conversationId: string | null }): JSX.Element | null {
  const counts = useHeardSubagentCounts(profileId, conversationId);
  const shown = !!counts && counts.total > 0 && counts.share != null;
  const most = useMostExpensive(profileId, conversationId, shown);
  if (!counts || counts.total === 0 || counts.share == null) return null;
  return (
    <div className="flex flex-col gap-1">
    <p className="flex flex-wrap items-center gap-x-1.5 text-body text-ink-2" data-testid="usage-subagents-line">
      <IconSubtasks aria-hidden="true" width={15} height={15} className="text-muted" />
      <span>
        Subagents: <span className="font-semibold tabular-nums text-ink">{shareWords(counts.share)}</span> of this session's tokens
      </span>
      <span aria-hidden="true">·</span>
      <button
        type="button"
        onClick={() => openSubagents({ sort: "tokens" })}
        className="rounded text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
        data-testid="usage-subagents-open"
      >
        see which ›
      </button>
    </p>
    {most.length > 0 && (
      <p className="flex flex-wrap items-baseline gap-x-1.5 pl-[21px] text-meta text-muted" data-testid="usage-subagents-most">
        <span>Used most:</span>
        {most.map((agent, n) => (
          <span key={agent.id} className="inline-flex min-w-0 items-baseline gap-x-1">
            {n > 0 && <span aria-hidden="true">·</span>}
            <button
              type="button"
              onClick={() => openSubagents({ subagent: agent.id })}
              className="max-w-[16rem] truncate rounded text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
              data-testid="usage-subagents-most-open"
            >
              {titleOf(agent)}
            </button>
            <span className="tabular-nums">{short(agent.own_tokens ?? agent.tokens ?? 0, "tokens")}</span>
          </span>
        ))}
      </p>
    )}
    </div>
  );
}
