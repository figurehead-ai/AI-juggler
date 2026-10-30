/**
 * 0.3 — a project's Subagents tab: the mount point. The subagents builder
 * (branch subagents-0.3) provides the real `ProjectSubagents` (every
 * subagent its sessions ran, what each did and cost); until it lands this
 * lists the project's sessions, each opening its own Subagents tab.
 */
import type { JSX } from "react";
import { useActionHost } from "../../components/ItemMenu";
import { sessionTitle } from "../../components/ManageWork";
import { sessionState } from "../../lib/rail-items";
import { rememberTab } from "../registry";
import { useHeardSubagentCounts } from "../session/useSubagents";
import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";

function Row({ session }: { session: InstanceInfo }): JSX.Element {
  const host = useActionHost();
  const counts = useHeardSubagentCounts(session.launch_recipe_id, session.conversation_id);
  const state = sessionState(session);
  return (
    <li>
      <button
        type="button"
        onClick={() => (rememberTab("session", "subagents"), host.openSessionById?.(session.id))}
        className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
        data-testid="project-subagents-session"
      >
        <span aria-hidden="true" className={"size-2 shrink-0 rounded-full " + state.dot} />
        <span className="min-w-0 flex-1 truncate text-body text-ink">{sessionTitle(session)}</span>
        <span className="shrink-0 text-meta text-muted">{counts ? `${counts.total} subagents${counts.running > 0 ? ` · ${counts.running} running` : ""}` : state.word}</span>
        <span aria-hidden="true" className="text-muted">›</span>
      </button>
    </li>
  );
}

export function ProjectSubagents({ project, sessions }: { project: ProjectInfo; sessions: InstanceInfo[] }): JSX.Element {
  const rows = sessions.filter((session) => session.project_id === project.id && session.lifecycle !== "archived").slice(0, 50);
  return (
    <div className="flex w-full max-w-content flex-col gap-3" data-testid="project-subagents">
      <p className="text-body text-muted">The subagents each session here started: open a session to see what each one did, how long it ran and what it found.</p>
      {rows.length === 0 ? (
        <p className="rounded-lg border border-dashed border-hairline px-4 py-6 text-center text-body text-muted">No session here has run yet.</p>
      ) : (
        <ul className="flex flex-col divide-y divide-hairline/70" aria-label="Sessions and their subagents">
          {rows.map((session) => (
            <Row key={session.id} session={session} />
          ))}
        </ul>
      )}
    </div>
  );
}
