/**
 * AJ-E1520 (0.3) — the Usage tab's line to the subagents: "Subagents
 * used 41% of these tokens · see which ›", which opens the Subagents tab
 * sorted by most tokens. Only when the session started some and their
 * tokens are recorded; their share is part of the total above, never
 * added to it.
 */
import type { JSX } from "react";
import { IconSubtasks } from "../../components/icons";
import { openSubagents } from "../../lib/subagent-links";
import { useHeardSubagentCounts } from "../session/useSubagents";
import { shareWords } from "./words";

export function SubagentsShareLine({ profileId, conversationId }: { profileId: string; conversationId: string | null }): JSX.Element | null {
  const counts = useHeardSubagentCounts(profileId, conversationId);
  if (!counts || counts.total === 0 || counts.share == null) return null;
  return (
    <p className="flex flex-wrap items-center gap-x-1.5 text-body text-muted" data-testid="usage-subagents-line">
      <IconSubtasks aria-hidden="true" width={15} height={15} className="text-muted" />
      <span>
        Subagents used <span className="font-semibold tabular-nums text-ink">{shareWords(counts.share)}</span> of these tokens
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
  );
}
