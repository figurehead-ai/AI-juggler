/**
 * A project in a few words: when it was last worked in, how many
 * conversations, and who — the rail row's second line, and the project
 * page's history line.
 */
import { agoWords } from "./time-words";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

/** When a project was last worked in: here or, before that, elsewhere. */
export function lastWorked(project: ProjectInfo): number {
  return project.history_last_used_ms ?? project.last_activity_ms ?? 0;
}

/** The second line of a project row: what was done there, in a few words. */
export function projectStats(project: ProjectInfo, now: number): string[] {
  // When first (a narrow rail cuts from the end), then how much, then who.
  const parts: string[] = [];
  const at = lastWorked(project);
  if (at > 0) parts.push(agoWords(at, now));
  const conversations = project.history_sessions + project.instance_count;
  if (conversations > 0) parts.push(conversations === 1 ? "1 conversation" : `${conversations} conversations`);
  const accounts = project.accounts_used.length;
  if (accounts > 1) parts.push(`${accounts} apps`);
  else if (accounts === 1) parts.push(project.accounts_used[0]!);
  return parts;
}

/** A folder as the user knows it: under their home, `~/…` (never the
 * absolute path on screen: P24). */
export function homeRelative(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~");
}

/** Projects as a picker lists them: by name, and where two share a name
 * their folders under it, so each can be told apart (the go sitting's
 * 7b walk: New pool's "Add a project" listed "web" twice). */
export function projectChoices(projects: readonly { id: string; name: string; root?: string }[]): { value: string; label: string; hint?: string }[] {
  const seen = new Map<string, number>();
  for (const project of projects) seen.set(project.name, (seen.get(project.name) ?? 0) + 1);
  return projects.map((project) => {
    const shared = (seen.get(project.name) ?? 0) > 1 && project.root;
    return shared ? { value: project.id, label: project.name, hint: homeRelative(project.root!) } : { value: project.id, label: project.name };
  });
}

/** A session's place: project name (or folder) and branch. */
export function placeOf(session: InstanceInfo, projects: ProjectInfo[]): { title: string; detail: string } {
  const project = projects.find((p) => p.id === session.project_id);
  const cwd = session.cwd ?? "";
  const folder = cwd.replace(/\/+$/, "").split("/").pop() || cwd || session.account_label;
  const title = project?.name ?? folder;
  const detail = [session.git_branch, session.account_label].filter(Boolean).join(" · ");
  return { title, detail };
}

/** Branches that name no piece of work. */
const PLAIN_BRANCHES = new Set(["main", "master", "trunk", "develop", "development", "dev", "head", "default"]);

/** The branch when it says what the work was ("fix/limits"), never one
 * that says nothing ("main"). */
export function tellingBranch(branch: string | null | undefined): string | null {
  return branch && !PLAIN_BRANCHES.has(branch.toLowerCase()) ? branch : null;
}
