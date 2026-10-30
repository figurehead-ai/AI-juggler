/**
 * 0.3 (the redesign, §3) — a project page's tabs: Overview · Sessions ·
 * Tasks · Pool · Subagents · Usage · History. The page keeps every tab
 * mounted (PageShell); the heavy ones (its pool's tasks, its pool, its
 * subagents, its past sessions) load the first time they're shown and
 * stay, so going back to one keeps its scroll and what was open.
 */
import { useEffect, useMemo, useState, type JSX, type ReactNode } from "react";
import { lazyNamed } from "../../lib/lazy";
import { defaultPoolIdOf } from "../../lib/pools";
import { useProjectTasks } from "./useProjectTasks";
import { isActiveStatus } from "../../components/indicator";
import { openTasks } from "../../lib/project-summary";
import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";
import type { PageTabId } from "../registry";
import { ProjectOverview, ProjectSessions, type ProjectPanelProps } from "../ProjectPage";
import { ProjectUsage } from "../ProjectUsage";

const PoolTasks = lazyNamed(() => import("../tasks/PoolTasks"), "PoolTasks");
const ProjectPool = lazyNamed(() => import("./ProjectPool"), "ProjectPool");
// The subagents builder's ProjectSubagents mounts here (subagents-0.3).
const ProjectSubagents = lazyNamed(() => import("./ProjectSubagents"), "ProjectSubagents");
const History = lazyNamed(() => import("../../screens/History"), "History");

/** Mounted the first time it's shown, and kept. */
function Once({ visible, children }: { visible: boolean; children: ReactNode }): JSX.Element | null {
  const [shown, setShown] = useState(visible);
  useEffect(() => {
    if (visible) setShown(true);
  }, [visible]);
  return shown || visible ? <>{children}</> : null;
}

/** The numbers beside a project's tabs: its open sessions, open tasks. */
export function useProjectTabCounts(project: ProjectInfo | null | undefined, sessions: InstanceInfo[]): Partial<Record<PageTabId, number>> | undefined {
  const tasks = useProjectTasks(project?.id ?? null);
  const open = useMemo(() => (project ? sessions.filter((s) => s.project_id === project.id && isActiveStatus(s.status)).length : 0), [project, sessions]);
  if (!project) return undefined;
  return { sessions: open, tasks: openTasks(tasks)?.open ?? 0 };
}

export function ProjectTab(props: ProjectPanelProps & { projects: ProjectInfo[]; tab: PageTabId; visible: boolean }): JSX.Element | null {
  const { project, tab, visible } = props;
  switch (tab) {
    case "overview":
      return <ProjectOverview {...props} />;
    case "sessions":
      return <ProjectSessions {...props} />;
    case "usage":
      return <ProjectUsage projectId={project.id} sessions={props.sessions} />;
    case "tasks":
      return (
        <Once visible={visible}>
          <PoolTasks poolId={defaultPoolIdOf(project.id)} sessions={props.sessions} projects={props.projects} now={props.now} />
        </Once>
      );
    case "pool":
      return (
        <Once visible={visible}>
          <ProjectPool project={project} sessions={props.sessions} now={props.now} />
        </Once>
      );
    case "subagents":
      return (
        <Once visible={visible}>
          <ProjectSubagents project={project} sessions={props.sessions} />
        </Once>
      );
    case "history":
      return (
        <Once visible={visible}>
          <History projectId={project.id} sessions={props.sessions} projects={props.projects} accounts={props.accounts} onChanged={props.onStopped} />
        </Once>
      );
    default:
      return null;
  }
}
