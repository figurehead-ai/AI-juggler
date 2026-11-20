/**
 * D-62 (AJ-E3207) — New session's usual folder, with a short why, and the
 * folder picker's top groups. Pure, so the tests pin every case.
 *
 * The folder: the one it was opened for (a project's page, a drop, a
 * handover; from the header or ⌘N, the project of the page you're on),
 * else the first of Recent: the project you last started a session in
 * here, else the one worked in last (in the tool's own history), else,
 * with no time on any, a project the user has (walk N4, rc26: one without
 * history once opened on a temporary folder), else the App's own folder
 * from its launch settings, else none.
 *
 * AJ-E2311 item 2 (the 0.4.6 walks: every New session opened on another
 * of the maintainer's projects, and Recent reshuffled): the tools'
 * histories move whenever any of their sessions writes, so they come
 * after the starts made here, which move only when the person starts one;
 * and the sheet holds Recent's order while it's open (`steadyRecent`).
 */
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { recentProjects, TOP } from "./pickers";
import type { UsedSession } from "./new-session-history";

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

/** Recent's order (AJ-E2311 item 2): the projects you started a session
 * in here (AI Juggler's sessions, `sessions`), newest start first; then
 * the others worked in lately, here or in the tool's history; at most
 * `TOP`. Without sessions, as before: by when each was worked in. */
export function recentFolders(projects: readonly ProjectInfo[], sessions: readonly UsedSession[] = []): string[] {
  const offered = offeredProjects(projects);
  const started = new Map<string, number>();
  for (const row of sessions) if (row.project_id && (started.get(row.project_id) ?? -1) < (row.started_at_ms ?? 0)) started.set(row.project_id, row.started_at_ms ?? 0);
  const here = offered
    .filter((project) => started.has(project.id))
    .sort((a, b) => started.get(b.id)! - started.get(a.id)!)
    .map((project) => project.id);
  return [...here, ...recentProjects(offered).filter((id) => !here.includes(id))].slice(0, TOP);
}

/** Recent as the sheet shows it while it's open: the order it first
 * showed (`held`), less a project no longer offered, then any new one to
 * fill it; `fresh` (`recentFolders`) when nothing is held yet. */
export function steadyRecent(held: readonly string[] | null, fresh: readonly string[], projects: readonly ProjectInfo[]): string[] {
  if (!held || held.length === 0) return [...fresh];
  const offered = new Set(offeredProjects(projects).map((project) => project.id));
  const kept = held.filter((id) => offered.has(id));
  return [...kept, ...fresh.filter((id) => !kept.includes(id))].slice(0, TOP);
}

/** The folder it opens on (`null`: none known; the App's own folder is
 * the caller's, once the App is chosen): `asked`, else the first of
 * `recent` (`recentFolders`). */
export function chooseFolder(projects: readonly ProjectInfo[], asked: string | null, recent?: readonly string[]): FolderChoice | null {
  if (asked) return { root: asked, project: projects.find((project) => project.root === asked) ?? null, why: "asked" };
  const offered = offeredProjects(projects);
  const latest = (recent ?? recentFolders(offered))[0];
  const project = offered.find((row) => row.id === latest);
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
export function poolProject(pool: PoolScope | null | undefined): string | null {
  if (!pool) return null;
  if (pool.project_id) return pool.project_id;
  if (pool.scope_all || pool.scope.length !== 1) return null;
  return pool.scope[0] ?? null;
}

/** What a pool's scope says of its projects. */
type PoolScope = { project_id: string | null; scope: readonly (string | null)[]; scope_all: boolean };

/** What `hereProject` reads of the page you're on (the shell's selection). */
export type HerePage = { kind: string; id?: string; projectId?: string | null };

/** AJ-E2311 item 2: the folder of the page you're on, for New session
 * from the header or ⌘N: a project's page, a session's (its project's
 * root, else its own folder), another app's session's (its project), a
 * pool's (`poolProject`); `null` anywhere else. */
export function hereProject(
  page: HerePage,
  { pools, projects, sessions }: { pools: readonly ({ id: string } & PoolScope)[]; projects: readonly ProjectInfo[]; sessions: readonly { id: string; project_id: string | null; cwd: string }[] },
): string | null {
  const rootOf = (id: string | null | undefined) => (id ? (projects.find((project) => project.id === id)?.root ?? null) : null);
  switch (page.kind) {
    case "project":
      return rootOf(page.id);
    case "session": {
      const session = sessions.find((row) => row.id === page.id);
      return session ? (rootOf(session.project_id) ?? session.cwd) : null;
    }
    case "conversation":
      return rootOf(page.projectId);
    case "pool":
      return rootOf(poolProject(pools.find((pool) => pool.id === page.id)));
    default:
      return null;
  }
}

/** D-96 F: the project "Start another" offers after a start: the next
 * the user works in, by the order New session opens on (Recent's, from
 * `sessions`, else used by the tool's history), never one started in this
 * run (`started`, their roots) and never a guess (a project without work
 * in it); `null` when none is left. */
export function nextFolder(projects: readonly ProjectInfo[], started: readonly string[], sessions: readonly UsedSession[] = []): FolderChoice | null {
  const left = projects.filter((project) => !started.includes(project.root));
  const next = chooseFolder(left, null, recentFolders(left, sessions));
  return next && (next.why === "latest" || next.why === "used-before") ? next : null;
}

/** The folder picker's groups before All: Recent (`recent`, as the sheet
 * holds it; else `recentFolders`), then pinned. */
export function folderGroups(projects: readonly ProjectInfo[], recent: readonly string[] = recentFolders(projects)): { label: string; values: string[] }[] {
  const offered = offeredProjects(projects);
  const pinned = offered.filter((project) => project.pinned && !recent.includes(project.id)).map((project) => project.id);
  return [
    { label: "Recent", values: [...recent] },
    { label: "Pinned", values: pinned },
  ];
}
