/**
 * 0.3 (the redesign, §1) — the header's new doors, as one global screen
 * App renders (its size budget): Projects, Pools and Tasks.
 */
import type { JSX } from "react";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import { ProjectsScreen } from "./projects/ProjectsScreen";
import { PoolsScreen } from "./pools/PoolsScreen";
import { TasksScreen } from "./tasks/TasksScreen";

export type NavScreenId = "projects" | "pools" | "tasks";

export function NavScreen({ id, projects, sessions, loading, now, onNewProject }: { id: NavScreenId; projects: ProjectInfo[]; sessions: InstanceInfo[]; loading: boolean; now: number; onNewProject: () => void }): JSX.Element {
  if (id === "projects") return <ProjectsScreen projects={projects} sessions={sessions} loading={loading} now={now} onNewProject={onNewProject} />;
  if (id === "pools") return <PoolsScreen sessions={sessions} projects={projects} />;
  return <TasksScreen projects={projects} sessions={sessions} now={now} />;
}
