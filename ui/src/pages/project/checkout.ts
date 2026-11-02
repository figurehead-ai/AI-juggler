/**
 * 0.3.4 — a project's checkout as its page last read it: git's state and
 * whether its rules files agree, per project. The header's line and the
 * Overview's callout share one read and show the last answer in their
 * first frame (both drew nothing until it came, then pushed the page
 * down). Each is read again whenever a page of the project opens; the
 * rules also when the rules sheet closes (`PROJECT_SYNC_EVENT`). At most
 * 32 projects are kept.
 */
import { PROJECT_SYNC_EVENT } from "../../components/ProjectSyncSheet";
import { projectSyncRules } from "../../lib/ipc/projectSync";
import { projectsGitStatus } from "../../lib/ipc/projects";
import { sharedReads, useSharedRead } from "../../lib/shared-read";
import type { GitStatusInfo } from "../../types/generated/GitStatusInfo";

export type RulesSummary = { words: string; inSync: boolean };

/** `null`: not a git checkout (or git couldn't be read). */
const gitOf = sharedReads<GitStatusInfo | null>((projectId) => projectsGitStatus(projectId).catch(() => null));
/** `null`: nothing to compare (or the rules couldn't be read). */
const rulesOf = sharedReads<RulesSummary | null>((projectId) =>
  projectSyncRules(projectId).then(
    (rules) => (rules.summary ? { words: rules.summary, inSync: rules.in_sync } : null),
    () => null,
  ),
);

let watching = false;
function watchRules(): void {
  if (watching || typeof window === "undefined") return;
  watching = true;
  window.addEventListener(PROJECT_SYNC_EVENT, (event) => {
    const projectId = (event as CustomEvent<string>).detail;
    if (typeof projectId === "string") void rulesOf(projectId).refresh();
  });
}

/** Git's state of the project's root; `undefined` until first read. */
export function useProjectGit(projectId: string): GitStatusInfo | null | undefined {
  return useSharedRead(gitOf(projectId));
}

/** Whether the project's rules files agree; `undefined` until first read. */
export function useProjectRules(projectId: string): RulesSummary | null | undefined {
  watchRules();
  return useSharedRead(rulesOf(projectId));
}
