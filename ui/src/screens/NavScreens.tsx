/**
 * 0.3 (the redesign, §1) — the header's new doors, as one global screen
 * App renders (its size budget): Projects, Pools and Tasks.
 */
import { lazyNamed } from "../lib/lazy";
// AJ-E3512: loaded when it opens.
const FloorList = lazyNamed(() => import("../pages/floor/FloorList"), "FloorList");
import type { JSX } from "react";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { ProjectsScreen } from "./projects/ProjectsScreen";
import { PoolsScreen } from "./pools/PoolsScreen";
import { TasksScreen } from "./tasks/TasksScreen";

export type NavScreenId = "projects" | "pools" | "tasks" | "floor";

export function NavScreen({
  id,
  projects,
  sessions,
  accounts,
  loading,
  now,
  onNewProject,
}: {
  id: NavScreenId;
  projects: ProjectInfo[];
  sessions: InstanceInfo[];
  accounts: AccountInfo[];
  loading: boolean;
  now: number;
  onNewProject: () => void;
}): JSX.Element {
  if (id === "projects") return <ProjectsScreen projects={projects} sessions={sessions} loading={loading} now={now} onNewProject={onNewProject} />;
  if (id === "floor") return <FloorList />;
  if (id === "pools") return <PoolsScreen sessions={sessions} projects={projects} accounts={accounts} />;
  return <TasksScreen projects={projects} sessions={sessions} accounts={accounts} now={now} />;
}
