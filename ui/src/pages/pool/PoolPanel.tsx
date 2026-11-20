/**
 * A pool page's other tabs, Tasks and Messages (D-55), over the canvas:
 * open, they take the whole width beside nothing — beside the inspector
 * their list and their open task or thread were squeezed into a column a
 * few words wide at 1280 px (the 0.3 re-render). Each loads only when
 * first opened. Back to the canvas, or Escape, closes it.
 */
import type { JSX } from "react";

import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { lazyNamed } from "../../lib/lazy";
import { ListSkeleton } from "../../components/ListSkeleton";
import type { Panel } from "./PoolHeader";

// SK-8: a skeleton while their code loads (Tasks opened on an empty body
// for ~7 s on a busy Mac), as their lists show while read.
const PoolMessages = lazyNamed(() => import("../messages/PoolMessages"), "PoolMessages", false, () => <ListSkeleton label="Loading the messages" rows={4} />);
const PoolTasks = lazyNamed(() => import("../tasks/PoolTasks"), "PoolTasks", false, () => <ListSkeleton label="Loading the tasks" rows={4} />);

export function PoolPanel({
  panel,
  poolId,
  taskKey,
  sessions,
  projects,
  members,
  now,
  onClose,
}: {
  panel: NonNullable<Panel>;
  poolId: string;
  /** Bumped to open a task again (a link to one). */
  taskKey: number;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  /** Who a message can go to. */
  members: { session_id: string; name: string; live: boolean }[];
  now: number;
  onClose: () => void;
}): JSX.Element {
  return (
    <section aria-label={panel === "tasks" ? "Tasks" : "Messages"} className="absolute inset-3 z-30 flex min-h-0 flex-col overflow-hidden rounded-xl border border-hairline bg-surface shadow-lg" data-testid={`pool-panel-${panel}-open`}>
      <div className="flex h-11 shrink-0 items-center gap-2 border-b border-hairline px-4">
        <h3 className="flex-1 text-section text-ink">{panel === "tasks" ? "Tasks" : "Messages"}</h3>
        <button type="button" onClick={onClose} className="rounded-md px-2 py-1 text-meta text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="pool-panel-close">
          Back to the canvas
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-4" data-canvas-scroll>
        {panel === "tasks" ? <PoolTasks key={taskKey} poolId={poolId} sessions={sessions} projects={projects} now={now} /> : <PoolMessages poolId={poolId} members={members} now={now} />}
      </div>
    </section>
  );
}
