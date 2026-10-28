/**
 * AJ-E2502 — answers the menu-bar item (a notice to open, the centre, New
 * session…). Draws nothing. Needs you itself is the backend's (D-49): the
 * window sends nothing for it. In the preview (no backend) it loads the
 * stand-in that works the list out over the preview's own data.
 */
import { lazy, Suspense, useEffect, useRef, type JSX } from "react";
import { notificationsPage, onNewSessionAsked, onNoticeOpen } from "../lib/ipc/notices";
import { setCentreOpen } from "../lib/notices-store";
import { hasLiveBackend, type AccountInfo, type InstanceInfo, type ProjectInfo } from "../lib/bridge";
import type { ThisMacInfo } from "../lib/machine";
import { useNoticeActions } from "../lib/notice-actions";

const PreviewNeeds = lazy(() => import("../preview/needs"));

export function NoticeOpeners({
  ready,
  sessions,
  accounts,
  projects,
  thisMac,
}: {
  /** The first load has answered (the preview's stand-in waits for it). */
  ready: boolean;
  sessions: InstanceInfo[];
  accounts: AccountInfo[];
  projects: ProjectInfo[];
  thisMac: ThisMacInfo | null;
}): JSX.Element | null {
  const actions = useNoticeActions(sessions);
  // The menu-bar item: a notice (its subject), the centre, or New session….
  const opener = useRef(actions.primary);
  opener.current = actions.primary;
  useEffect(() => {
    const stopOpen = onNoticeOpen((id) => {
      if (id === null) return setCentreOpen(true);
      void notificationsPage("needs", { cursor: null, limit: 200 }).then((page) => {
        const notice = page.items.find((row) => row.id === id);
        if (notice) opener.current(notice);
        else setCentreOpen(true);
      }, () => setCentreOpen(true));
    });
    const stopNew = onNewSessionAsked(() => window.dispatchEvent(new Event("aij:new-session")));
    return () => {
      stopOpen();
      stopNew();
    };
  }, []);
  if (hasLiveBackend()) return null;
  return (
    <Suspense fallback={null}>
      <PreviewNeeds ready={ready} sessions={sessions} accounts={accounts} projects={projects} thisMac={thisMac} />
    </Suspense>
  );
}
