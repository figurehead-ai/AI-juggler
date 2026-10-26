/**
 * The session page's tabs that other features own: E18's Messages and
 * E19's Tasks. Each shows only when the session has some (`useSessionTabs`
 * says which), and each tab's screen loads when it first opens.
 *
 * E19 × D-41: the session page holds the Processes tab, so it is what a
 * task's Processes link opens (`lib/task-links`): the link picks that tab,
 * and the task view opens the holder's session on it.
 */
import type { JSX } from "react";
import type { InstanceInfo } from "../../lib/bridge";
import { lazyNamed } from "../../lib/lazy";
import { provideProcessesLink } from "../../lib/task-links";
import { useSessionMessages } from "../messages/useSessionMessages";
import { useSessionTasks } from "../tasks/useSessionTasks";
import { rememberTab } from "../registry";

const SessionMessages = lazyNamed(() => import("../messages/PoolMessages"), "SessionMessages");
const SessionTasks = lazyNamed(() => import("../tasks/PoolTasks"), "SessionTasks");

provideProcessesLink(() => ({ label: "Its processes", open: () => rememberTab("session", "processes") }));

/** Which of these tabs a session's page shows (its `pageTabs` facts). */
export function useSessionTabs(sessionId: string | null): { hasMessages: boolean; hasTasks: boolean } {
  const messagesPool = useSessionMessages(sessionId);
  const hasTasks = useSessionTasks(sessionId);
  return { hasMessages: messagesPool !== null, hasTasks };
}

/** The Messages or Tasks tab of a session. */
export function SessionTab({ tab, session, now }: { tab: "messages" | "tasks"; session: InstanceInfo; now: number }): JSX.Element {
  return tab === "messages" ? <SessionMessages session={session} now={now} /> : <SessionTasks session={session} now={now} />;
}
