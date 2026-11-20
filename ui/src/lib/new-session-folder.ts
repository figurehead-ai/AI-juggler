/**
 * D-62 (AJ-E3207) — New session's usual folder, with a short why, and the
 * folder picker's top groups. Pure, so the tests pin every case.
 *
 * The folder: the one it was opened for (a project's page, a drop, a
 * handover), else the project worked in last (here or in the tool's own
 * history), else, with no time on any, a project the user has (walk N4,
 * rc26: one without history once opened on a temporary folder), else the
 * App's own folder from its launch settings, else none.
 */
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { recentProjects } from "./pickers";

/** `used-before`: no time, but the tool's history has sessions there;
 * `yours` / `one-of-yours`: no history at all, the only project or one of
 * several (never said as work in it: adding a folder isn't). */
export type FolderWhy = "asked" | "latest" | "used-before" | "yours" | "one-of-yours" | "app-folder";

export interface FolderChoice {
  root: string;
  project: ProjectInfo | null;
  why: FolderWhy;
}

/** When a project was last worked in, here or in the tool's history. */
export function workedAt(project: ProjectInfo): number {
  return Math.max(project.last_activity_ms ?? 0, project.history_last_used_ms ?? 0);
}

/** The projects New session offers: neither moved nor forgotten. */
export function offeredProjects(projects: readonly ProjectInfo[]): ProjectInfo[] {
  return projects.filter((project) => !project.missing && !project.forgotten);
}

/** The folder it opens on (`null`: none known; the App's own folder is
 * the caller's, once the App is chosen). */
export function chooseFolder(projects: readonly ProjectInfo[], asked: string | null): FolderChoice | null {
  if (asked) return { root: asked, project: projects.find((project) => project.root === asked) ?? null, why: "asked" };
  const offered = offeredProjects(projects);
  const latest = recentProjects(offered)[0];
  const project = projects.find((row) => row.id === latest);
  if (project) return { root: project.root, project, why: "latest" };
  return firstFolder(offered);
}

/** With no project worked in at any known time (walk N4, rc26: since the
 * backfill stopped stamping every found folder "now", 1e51970b0, a
 * newcomer's project has no time and New session opened on a temporary
 * folder): the project the tool's history used most, said as "used here
 * before"; else a project the user added (pinned first, then by name),
 * said as theirs. Nothing here claims a time. */
function firstFolder(offered: readonly ProjectInfo[]): FolderChoice | null {
  const used = offered.filter((row) => row.history_sessions > 0 || row.instance_count > 0 || row.accounts_used.length > 0);
  const byName = (a: ProjectInfo, b: ProjectInfo) => a.name.localeCompare(b.name);
  const best = [...used].sort((a, b) => b.history_sessions + b.instance_count - (a.history_sessions + a.instance_count) || byName(a, b))[0];
  if (best) return { root: best.root, project: best, why: "used-before" };
  const mine = [...offered].sort((a, b) => Number(!!b.pinned) - Number(!!a.pinned) || byName(a, b))[0];
  if (!mine) return null;
  return { root: mine.root, project: mine, why: offered.length === 1 ? "yours" : "one-of-yours" };
}

/** Why it's the folder, after its path, when no time says it. */
export function folderWhyWords(why: FolderWhy): string | null {
  switch (why) {
    case "used-before":
      return "used here before";
    case "yours":
      return "your project";
    case "one-of-yours":
      return "one of your projects";
    case "app-folder":
      return "its launch settings' folder";
    default:
      return null;
  }
}
/** The project a pool's page opens New session on (the existing order's
 * "the one it was opened for"): a project's own pool's, else its scope's
 * only project; `null` for a pool over several, none or every project. */
export function poolProject(pool: { project_id: string | null; scope: readonly (string | null)[]; scope_all: boolean } | null | undefined): string | null {
  if (!pool) return null;
  if (pool.project_id) return pool.project_id;
  if (pool.scope_all || pool.scope.length !== 1) return null;
  return pool.scope[0] ?? null;
}
/** D-96 F: the project "Start another" offers after a start: the next
 * the user works in, by the order New session opens on (worked in last,
 * else used by the tool's history), never one started in this run
 * (`started`, their roots) and never a guess (a project without work in
 * it); `null` when none is left. */
export function nextFolder(projects: readonly ProjectInfo[], started: readonly string[]): FolderChoice | null {
  const left = projects.filter((project) => !started.includes(project.root));
  const next = chooseFolder(left, null);
  return next && (next.why === "latest" || next.why === "used-before") ? next : null;
}

/** The folder picker's groups before All: worked in lately, then pinned. */
export function folderGroups(projects: readonly ProjectInfo[]): { label: string; values: string[] }[] {
  const offered = offeredProjects(projects);
  const recent = recentProjects(offered);
  const pinned = offered.filter((project) => project.pinned && !recent.includes(project.id)).map((project) => project.id);
  return [
    { label: "Recent", values: recent },
    { label: "Pinned", values: pinned },
  ];
}
