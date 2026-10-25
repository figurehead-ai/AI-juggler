/**
 * A project in a few words: when it was last worked in, how many
 * conversations, and who — the rail row's second line, and the project
 * page's history line.
 */
import { agoWords } from "./time-words";
import type { ProjectInfo } from "./bridge";

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
  if (accounts > 1) parts.push(`${accounts} accounts`);
  else if (accounts === 1) parts.push(project.accounts_used[0]!);
  return parts;
}

/** A folder as the user knows it: under their home, `~/…` (never the
 * absolute path on screen: P24). */
export function homeRelative(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~");
}

/** Branches that name no piece of work. */
const PLAIN_BRANCHES = new Set(["main", "master", "trunk", "develop", "development", "dev", "head", "default"]);

/** A session's title from what's known when nothing named it (the review
 * of 0.1.4: rows read "main"): a telling branch ("fix/limits"), else
 * "Untitled session · <project or folder>". */
export function untitled(branch: string | null | undefined, project: string): string {
  if (branch && !PLAIN_BRANCHES.has(branch.toLowerCase())) return branch;
  return project ? `Untitled session · ${project}` : "Untitled session";
}
