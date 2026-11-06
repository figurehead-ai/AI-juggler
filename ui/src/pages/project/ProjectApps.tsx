/**
 * E31 (AJ-E3104; relations-map §4) — a project's Apps tab: every App its
 * sessions ran in, each with what's open there and when it was last used,
 * one click from its page. Which Apps comes from the relation cache (all
 * its sessions, not only the ones the app holds now), so the tab's count
 * and the list agree; the design preview works them out from what it
 * shows.
 */
import { useMemo, type JSX } from "react";
import { hasLiveBackend } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { appsAmong, type AccountApp } from "../../lib/account-items";
import { useApps } from "../../lib/apps";
import { appByConversation } from "../../lib/rail-apps";
import { itemsOf } from "../../lib/rail-items";
import { useRelatedIds } from "../../lib/relation-counts";
import { useSeen } from "../../lib/seen";
import { useShownConversations } from "../../lib/shown-apps";
import { RelatedEmpty } from "../../components/related/RelatedEmpty";
import { AccountAppRows } from "../subscription/AccountApps";

/** The Apps `project`'s sessions ran in; `null` until the Apps are read. */
export function useProjectApps(project: ProjectInfo | null | undefined, sessions: InstanceInfo[], accounts: AccountInfo[], projects: ProjectInfo[]): AccountApp[] | null {
  const apps = useApps(accounts);
  const { conversations } = useShownConversations();
  const seen = useSeen();
  const live = hasLiveBackend();
  const ids = useRelatedIds(live && project ? "project" : null, project?.id, "app");
  const items = useMemo(
    () => (project ? itemsOf({ sessions, conversations, projects, seen }).filter((item) => item.projectId === project.id) : []),
    [project, sessions, conversations, projects, seen],
  );
  const byConversation = useMemo(() => appByConversation((apps ?? []).flatMap((app) => (app.place ? [app.place] : []))), [apps]);
  return useMemo(() => (apps && project ? appsAmong(apps, live ? ids : undefined, items, byConversation) : null), [apps, project, live, ids, items, byConversation]);
}

export function ProjectApps({ project, sessions, accounts, projects, now }: { project: ProjectInfo; sessions: InstanceInfo[]; accounts: AccountInfo[]; projects: ProjectInfo[]; now: number }): JSX.Element {
  const rows = useProjectApps(project, sessions, accounts, projects);
  if (rows === null) return <div className="h-24" data-testid="project-apps-loading" />;
  if (rows.length === 0)
    return <RelatedEmpty testId="project-apps-empty" teach={{ title: `No App has worked in ${project.name} yet`, body: "Start a session here and its App shows up, with what's open in it." }} />;
  return (
    <div className="w-full max-w-content rounded-xl border border-hairline bg-surface px-2 py-2" data-testid="project-apps-tab">
      <AccountAppRows apps={rows} now={now} />
    </div>
  );
}
