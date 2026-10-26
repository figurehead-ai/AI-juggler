/**
 * D-25: the rail row's quiet notes, after its state: uncommitted work,
 * subagents still running (AJ-E1520, I-08) and a sign-in to check. Each
 * shows only when there's something to say; the session view has the
 * rest.
 */
import type { JSX } from "react";
import type { InstanceInfo } from "../lib/bridge";
import { identityWords } from "../lib/identity";
import { subagentsRunning } from "../lib/subagents";
import { gitWords } from "./indicator";

export function RailQuietNotes({ session }: { session: InstanceInfo }): JSX.Element {
  const identity = identityWords(session);
  const identityMark = identity && identity.state !== "verified" ? identity : null;
  const dirty = session.git_dirty_count ? `${session.git_dirty_count} changed` : null;
  const subagents = session.subtask_count ? session.subtask_count : null;
  return (
    <>
      {dirty && (
        <span title={gitWords(session)} data-testid="git-badge" className="shrink-0 whitespace-nowrap">
          · {dirty}
        </span>
      )}
      {subagents && (
        <span title={subagentsRunning(subagents)} data-testid="rail-subagents" className="shrink-0 whitespace-nowrap">
          · {subagents === 1 ? "1 subagent" : `${subagents} subagents`}
        </span>
      )}
      {identityMark && (
        <span title={identityMark.text} aria-label={identityMark.text} data-testid="rail-identity" data-state={identityMark.state} className="shrink-0 whitespace-nowrap">
          · {identityMark.state === "signed_out" ? "not signed in" : identityMark.state === "first_seen" ? "first sign-in" : "identity unchecked"}
        </span>
      )}
    </>
  );
}
