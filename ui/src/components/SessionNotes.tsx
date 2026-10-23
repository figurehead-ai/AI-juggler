/**
 * The session page's notes under its header: its worktree and git state,
 * whether it's parked, and the subagents it started (AJ-E1520).
 */
import type { JSX } from "react";
import type { InstanceInfo } from "../lib/bridge";
import { ParkCard } from "./ParkCard";
import { SubagentsSection } from "./SubagentsSection";
import { WorktreeGit } from "./WorktreeGit";

export function SessionNotes({ session, live, now }: { session: InstanceInfo; live: boolean; now: number }): JSX.Element {
  return (
    <>
      <div className="flex flex-col gap-1 text-[11px] text-muted">
        <WorktreeGit sessionId={session.id} live={live} />
        <ParkCard session={session} now={now} />
      </div>
      <SubagentsSection sessionId={session.id} running={session.subtask_count} live={live} />
    </>
  );
}
