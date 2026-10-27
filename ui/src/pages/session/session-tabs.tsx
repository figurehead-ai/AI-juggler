/**
 * The session page's tabs that other features own: E18's Messages,
 * E19's Tasks and AJ-E1520's Subagents. Each shows only when the session
 * has some (`useSessionTabs` says which), and each tab's screen loads
 * when it first opens.
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
import { fromInstance } from "./model";

const SessionMessages = lazyNamed(() => import("../messages/PoolMessages"), "SessionMessages");
const SessionTasks = lazyNamed(() => import("../tasks/PoolTasks"), "SessionTasks");
const SubagentsTab = lazyNamed(() => import("./SubagentsTab"), "SubagentsTab");

provideProcessesLink(() => ({ label: "Its processes", open: () => rememberTab("session", "processes") }));

/** Which of these tabs a session's page shows (its `pageTabs` facts). */
export function useSessionTabs(session: InstanceInfo | null): { hasMessages: boolean; hasTasks: boolean; hasSubagents: boolean } {
  const sessionId = session?.id ?? null;
  const messagesPool = useSessionMessages(sessionId);
  const hasTasks = useSessionTasks(sessionId);
  const live = session ? ["starting", "running", "waiting_user", "stuck", "quota_blocked"].includes(session.status) : false;
  const hasSubagents = useHasSubagents(session?.account_id ?? null, session?.conversation_id ?? null, live);
  return { hasMessages: messagesPool !== null, hasTasks, hasSubagents };
}

/** The Messages, Tasks or Subagents tab of a session. */
export function SessionTab({ tab, session, now, visible = true }: { tab: "messages" | "tasks" | "subagents"; session: InstanceInfo; now: number; visible?: boolean }): JSX.Element {
  if (tab === "subagents") return <InstanceSubagents session={session} now={now} visible={visible} />;
  return tab === "messages" ? <SessionMessages session={session} now={now} /> : <SessionTasks session={session} now={now} />;
}

/** AJ-E1520: the Subagents tab of a session AI Juggler runs. */
function InstanceSubagents({ session, now, visible }: { session: InstanceInfo; now: number; visible: boolean }): JSX.Element {
  // The same object while nothing it shows has changed.
  const json = JSON.stringify(fromInstance(session));
  const model = useMemo(() => JSON.parse(json) as ReturnType<typeof fromInstance>, [json]);
  return <SubagentsTab model={model} visible={visible} now={now} />;
}
