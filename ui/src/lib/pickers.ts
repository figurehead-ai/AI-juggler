/**
 * 0.3.2 — the pickers rule's top groups (docs/plan/QUALITY.md "Finding
 * things"): what each kind of picker shows before All. Each returns ids in
 * order; the shared Picker draws them (`groups`).
 */
import type { InstanceInfo, ProjectInfo } from "./bridge";

/** At most this many in a top group. */
export const TOP = 5;

/** The projects worked in lately, most recent first. */
export function recentProjects(projects: readonly ProjectInfo[]): string[] {
  const when = (project: ProjectInfo) => Math.max(project.last_activity_ms ?? 0, project.history_last_used_ms ?? 0);
  return projects
    .filter((project) => when(project) > 0)
    .sort((a, b) => when(b) - when(a))
    .slice(0, TOP)
    .map((project) => project.id);
}

const OPEN = new Set(["running", "waiting_user", "stuck", "starting", "quota_blocked"]);

/** The sessions open now, the most recently active first. */
export function openSessions(sessions: readonly InstanceInfo[]): string[] {
  const when = (session: InstanceInfo) => session.last_activity_ms ?? session.started_at_ms ?? 0;
  return sessions
    .filter((session) => OPEN.has(session.status))
    .sort((a, b) => when(b) - when(a))
    .slice(0, TOP)
    .map((session) => session.id);
}
