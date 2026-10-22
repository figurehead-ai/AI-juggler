/**
 * A project in a few words: when it was last worked in, how many
 * conversations, and who — the rail row's second line, and the project
 * page's history line.
 */
import { ago } from "./time-words";
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
  if (at > 0) parts.push(ago(at, now) === "now" ? "just now" : `${ago(at, now)} ago`);
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
