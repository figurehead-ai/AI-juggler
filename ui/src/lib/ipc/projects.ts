/**
 * AJ-E1533 — the projects feature's calls (ENGINEERING-GUIDE U3).
 */
import { invoke } from "@tauri-apps/api/core";
import { hasLiveBackend } from "../bridge";
import type { GitStatusInfo } from "../../types/generated/GitStatusInfo";

/** A project's git state for its page's header (by its id; the folder is
 * the one AI Juggler recorded), `null` when it isn't a git checkout. */
export async function projectsGitStatus(projectId: string): Promise<GitStatusInfo | null> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_PROJECT_GIT__?: Record<string, GitStatusInfo> }).__AIJ_PROJECT_GIT__;
    return seeded?.[projectId] ?? null;
  }
  return invoke<GitStatusInfo | null>("projects_git_status", { projectId });
}
