/**
 * 0.3 (the redesign, §3) — a project page's tabs: Overview · Sessions ·
 * Apps · Tasks · Pools · Subagents · Usage · History (AJ-E3104: Apps,
 * and Pools for Pool). The page keeps every tab
 * mounted (PageShell); the heavy ones (its pool's tasks, its pool, its
 * subagents, its past sessions) load the first time they're shown and
 * stay, so going back to one keeps its scroll and what was open.
 */
import { useCallback, useMemo, type JSX } from "react";
import type { PoolInfo } from "../../lib/ipc/pools";
import { usePagePools } from "../../components/RelatedPools";
import { lazyNamed } from "../../lib/lazy";
import { defaultPoolIdOf } from "../../lib/pools";
import { useProjectTasks } from "./useProjectTasks";
import { outside, useConversations } from "../../lib/conversations";
import { openTasks } from "../../lib/project-summary";
import { hasLiveBackend, type AccountInfo, type InstanceInfo, type ProjectInfo } from "../../lib/bridge";
import { ProjectApps, useProjectApps } from "./ProjectApps";
import { useRelationCounts } from "../../lib/relation-counts";
import type { PageTabId } from "../registry";
import { WhenShown } from "../PageShell";
import { ProjectOverview, ProjectSessions, type ProjectPanelProps } from "../ProjectPage";
import { ProjectUsage } from "../ProjectUsage";

const PoolTasks = lazyNamed(() => import("../tasks/PoolTasks"), "PoolTasks");
const ProjectPool = lazyNamed(() => import("./ProjectPool"), "ProjectPool");
// The subagents builder's ProjectSubagents mounts here (subagents-0.3).
const ProjectSubagents = lazyNamed(() => import("./ProjectSubagents"), "ProjectSubagents");
const History = lazyNamed(() => import("../../screens/History"), "History");

/** Mounted the first time it's shown, and kept (pages/PageShell). */
const Once = WhenShown;

/** The numbers beside a project's tabs: its sessions, as its Sessions
 * list holds them (AJ-E3102: the backend's relation counts, a moment
 * after the page shows), and its open tasks. The design preview, with no
 * backend, counts the rows its list shows (AJ-E3103: AI Juggler's and the
 * ones other apps run, so the tab says what the list does). */
export function useProjectTabCounts(project: ProjectInfo | null | undefined, sessions: InstanceInfo[], accounts: AccountInfo[] = [], projects: ProjectInfo[] = []): Partial<Record<PageTabId, number>> | undefined {
  const tasks = useProjectTasks(project?.id ?? null);
  // AJ-E3104: its own pool and the others its Pools tab lists.
  const own = useMemo(() => (project ? sessions.filter((s) => s.project_id === project.id) : []), [project, sessions]);
  const forIt = useCallback((pool: PoolInfo) => pool.kind === "user" && pool.project_id === project?.id, [project?.id]);
  const pools = usePagePools(project ? "project" : null, project?.id ?? "", own, forIt);
  const apps = useProjectApps(project, sessions, accounts, projects);
  const related = useRelationCounts(project ? "project" : null, project?.id);
  const { all } = useConversations();
  const live = hasLiveBackend();
  const shown = useMemo(
    () => (project && !live ? sessions.filter((s) => s.project_id === project.id).length + outside(all).filter((c) => c.project_id === project.id).length : 0),
    [project, sessions, all, live],
  );
  if (!project) return undefined;
  const ownPool = project ? defaultPoolIdOf(project.id) : "";
  return { sessions: live ? related?.sessions.all : shown, tasks: openTasks(tasks)?.open ?? 0, apps: apps?.length, pools: pools && 1 + pools.filter((pool) => pool.id !== ownPool).length };
}

export function ProjectTab(props: ProjectPanelProps & { projects: ProjectInfo[]; tab: PageTabId; visible: boolean }): JSX.Element | null {
  const { project, tab, visible } = props;
  switch (tab) {
    case "overview":
      return <ProjectOverview {...props} />;
    case "sessions":
      return <ProjectSessions {...props} />;
    case "apps":
      return <ProjectApps project={project} sessions={props.sessions} accounts={props.accounts} projects={props.projects} now={props.now} />;
    case "usage":
      return <ProjectUsage projectId={project.id} sessions={props.sessions} />;
    case "tasks":
      return (
        <Once visible={visible}>
          <PoolTasks poolId={defaultPoolIdOf(project.id)} sessions={props.sessions} projects={props.projects} now={props.now} />
        </Once>
      );
    case "pools":
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
