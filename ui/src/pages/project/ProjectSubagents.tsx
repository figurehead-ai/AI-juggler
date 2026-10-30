/**
 * 0.3 — a project's Subagents tab (AJ-E1520): every subagent its newest
 * sessions started, as a session's Subagents tab shows them — what's
 * running, what failed, what it cost, grouped under the message that
 * started them, each group saying its session; a subagent's own page
 * one click away. The view and its data are the subagents feature's
 * (`pages/subagents/ProjectSubagentsView`); the backend finds the
 * project's sessions itself, so `sessions` isn't needed here.
 */
import type { JSX } from "react";
import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";
import { ProjectSubagentsView } from "../subagents/ProjectSubagentsView";

export function ProjectSubagents({ project }: { project: ProjectInfo; sessions?: InstanceInfo[] }): JSX.Element {
  return <ProjectSubagentsView projectId={project.id} />;
}
