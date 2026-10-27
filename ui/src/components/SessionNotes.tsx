/**
 * The session page's notes under its header: its worktree and git state,
 * whether it's parked, and — when it started any — one quiet line to its
 * subagents, which opens their tab (AJ-E1520, 0.2: the tab lists them;
 * wireframe screen 05's inspector names the count). The line's counts
 * come from the page's own read for its tabs (`useHasSubagents`).
 */
import type { JSX } from "react";
import type { InstanceInfo } from "../lib/bridge";
import { showTab } from "../pages/PageShell";
import { useHeardSubagentCounts, type SubagentCounts } from "../pages/session/useSubagents";
import { ParkCard } from "./ParkCard";
import { WorktreeGit } from "./WorktreeGit";

export function SessionNotes({ session, live, now }: { session: InstanceInfo; live: boolean; now: number }): JSX.Element {
  const subagents = useHeardSubagentCounts(session.account_id, session.conversation_id);
  return (
    <div className="flex flex-col gap-1 text-label text-muted">
      <WorktreeGit sessionId={session.id} live={live} />
      <ParkCard session={session} now={now} />
      {subagents && subagents.total > 0 && <SubagentsLine counts={subagents} />}
    </div>
  );
}

/** `3 subagents · 1 running` (the line's words, before its link). */
export function subagentsLineWords({ total, running }: SubagentCounts): string {
  const all = `${total} ${total === 1 ? "subagent" : "subagents"}`;
  return running > 0 ? `${all} · ${running} running` : all;
}

function SubagentsLine({ counts }: { counts: SubagentCounts }): JSX.Element {
  const words = subagentsLineWords(counts);
  return (
    <p className="flex items-center gap-1" data-testid="session-subagents-line">
      <span>{words}</span>
      <button
        type="button"
        data-testid="session-subagents-open"
        aria-label={`Open its subagents: ${words}`}
        onClick={() => showTab("subagents")}
        className="min-h-6 rounded px-0.5 text-muted outline-none hover:bg-surface-3 hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        <span aria-hidden="true">› </span>Subagents
      </button>
    </p>
  );
}
