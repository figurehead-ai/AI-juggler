/**
 * The session page's tabs that other features own: E18's Messages,
 * E19's Tasks, AJ-E1520's Subagents and wf09's History. Each shows only
 * when the session has some (`useSessionTabs` says which), and each tab's
 * screen loads when it first opens.
 *
 * E19 × D-41: the session page holds the Processes tab, so it is what a
 * task's Processes link opens (`lib/task-links`): the link picks that tab,
 * and the task view opens the holder's session on it.
 */
import { useMemo, type JSX } from "react";
import type { InstanceInfo } from "../../lib/bridge";
import { lazyNamed } from "../../lib/lazy";
import { provideProcessesLink } from "../../lib/task-links";
import { useSessionMessages } from "../messages/useSessionMessages";
import { useSessionTasks } from "../tasks/useSessionTasks";
import { rememberTab } from "../registry";
import { useHasSubagents } from "./useSubagents";
import { useSessionHasHistory } from "./useSessionHasHistory";
import { fromInstance } from "./model";
import { sessionTitle } from "../../components/ManageWork";

const SessionMessages = lazyNamed(() => import("../messages/PoolMessages"), "SessionMessages");
const SessionTasks = lazyNamed(() => import("../tasks/PoolTasks"), "SessionTasks");
const SubagentsTab = lazyNamed(() => import("./SubagentsTab"), "SubagentsTab");
const SessionHistoryTab = lazyNamed(() => import("./HistoryTab"), "SessionHistoryTab");

provideProcessesLink(() => ({ label: "Its processes", open: () => rememberTab("session", "processes") }));

/** Which of these tabs a session's page shows (its `pageTabs` facts). */
export function useSessionTabs(session: InstanceInfo | null): { hasMessages: boolean; hasTasks: boolean; hasSubagents: boolean; hasHistory: boolean } {
  const sessionId = session?.id ?? null;
  const messagesPool = useSessionMessages(sessionId);
  const hasTasks = useSessionTasks(sessionId);
  const live = session ? ["starting", "running", "waiting_user", "stuck", "quota_blocked"].includes(session.status) : false;
  const hasSubagents = useHasSubagents(session?.account_id ?? null, session?.conversation_id ?? null, live);
  // wf09: its History, once it has something recorded.
  const hasHistory = useSessionHasHistory(sessionId);
  return { hasMessages: messagesPool !== null, hasTasks, hasSubagents, hasHistory };
}

type SessionTabId = "messages" | "tasks" | "subagents" | "history";

/** The Messages, Tasks, Subagents or History tab of a session. */
export function SessionTab({ tab, session, now, visible = true, onOpenProject }: { tab: SessionTabId; session: InstanceInfo; now: number; visible?: boolean; onOpenProject: (id: string) => void }): JSX.Element {
  if (tab === "history") return <SessionHistoryTab session={session} onOpenProject={onOpenProject} />;
  if (tab === "subagents") return <InstanceSubagents session={session} now={now} visible={visible} />;
  return tab === "messages" ? <SessionMessages session={session} now={now} /> : <SessionTasks session={session} now={now} />;
}

/** AJ-E1520: the Subagents tab of a session AI Juggler runs. */
function InstanceSubagents({ session, now, visible }: { session: InstanceInfo; now: number; visible: boolean }): JSX.Element {
  // The same object while nothing it shows has changed.
  const json = JSON.stringify(fromInstance(session));
  const model = useMemo(() => JSON.parse(json) as ReturnType<typeof fromInstance>, [json]);
  return <SubagentsTab model={model} title={sessionTitle(session)} visible={visible} now={now} />;
}
