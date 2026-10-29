/**
 * AJ-E1537 — what App gives every menu: opening a session, project,
 * account or tool, focusing a session, carrying it to another account,
 * stopping it, copying its resume command, and starting a new session in
 * a project. Screens below add their own (the rail, Accounts, History).
 */
import { useMemo, type JSX, type ReactNode } from "react";
import { ActionHostProvider } from "./ItemMenu";
import { showToast } from "./Toast";
import type { ActionHost } from "../lib/actions";
import { desktopFocus, errorText, type InstanceInfo, type ProjectInfo } from "../lib/bridge";
import { sessionsResumeCommand } from "../lib/ipc/sessions";
import { conversationsContinue, conversationsGoTo, conversationsResumeCommand, type ConversationInfo } from "../lib/ipc/conversations";
import { useConversations } from "../lib/conversations";
import { openPoolSheet } from "../lib/pools";
import { markSeen } from "../lib/seen";
import { focusHandoverSoon } from "../lib/handover-focus";

async function copy(text: Promise<string> | string, done: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(await text);
    showToast({ id: "copied", message: done });
  } catch (cause) {
    showToast({ id: "copied", message: `Couldn't copy: ${errorText(cause)}` });
  }
}

export function AppActions({
  selectConversation,
  selectSession,
  selectProject,
  selectAccount,
  selectTool,
  selectPool,
  openAccounts,
  openHome,
  stopSession,
  newSessionIn,
  refresh,
  children,
}: {
  selectConversation: (conversation: ConversationInfo) => void;
  selectSession: (id: string) => void;
  selectProject: (id: string) => void;
  selectAccount: (id: string) => void;
  selectTool: (id: string) => void;
  /** AJ-E1703: a pool's page. */
  selectPool: (id: string) => void;
  openAccounts: () => void;
  openHome: () => void;
  stopSession: (row: InstanceInfo) => void;
  newSessionIn: (project: ProjectInfo) => void;
  refresh: () => void;
  children: ReactNode;
}): JSX.Element {
  const conversations = useConversations();
  const host = useMemo<ActionHost>(() => {
    // The same as ⌘⇧H on the session: its switch list.
    const carry = (row: InstanceInfo) => {
      selectSession(row.id);
      focusHandoverSoon();
    };
    return {
      openSession: (row) => selectSession(row.id),
      openSessionById: selectSession,
      openProjectById: selectProject,
      openAccountById: selectAccount,
      openHome,
      openHistory: (row) => selectSession(row.id),
      // A desktop session's own window; a terminal one's terminal here.
      focusSession: async (row) => {
        if (row.kind === "desktop") await desktopFocus(row.id).finally(refresh);
        selectSession(row.id);
      },
      continueSession: carry,
      continueHistory: carry,
      copyResumeCommand: (row) => copy(sessionsResumeCommand(row.id), "Copied the resume command. Paste it into a terminal."),
      // AJ-E1529: conversations started outside AI Juggler.
      // AJ-E2107: opening or going to it is looking at it (the seen mark).
      openConversation: (c) => {
        markSeen(c.account_id, c.id);
        selectConversation(c);
      },
      // AJ-E2102: feedback at once, then what really happened.
      goToConversation: async (c) => {
        markSeen(c.account_id, c.id);
        showToast({ id: "go-to", message: "Going to it…" });
        try {
          showToast({ id: "go-to", message: (await conversationsGoTo(c.account_id, c.id)).outcome });
        } catch (cause) {
          showToast({ id: "go-to", message: `Couldn't go to it: ${errorText(cause)}` });
        }
      },
      // F8: another app's session too, from its row menu and ⌘K.
      continueConversationElsewhere: (c) => {
        markSeen(c.account_id, c.id);
        selectConversation(c);
        focusHandoverSoon();
      },
      copyConversationResume: (c) => copy(conversationsResumeCommand(c.account_id, c.id), "Copied the resume command. Paste it into a terminal."),
      continueConversation: async (c) => {
        try {
          const launched = await conversationsContinue(c.account_id, c.id);
          refresh();
          conversations.refresh();
          selectSession(launched.session_id);
        } catch (cause) {
          showToast({ id: "continue", message: `Couldn't continue it here: ${errorText(cause)}` });
        }
      },
      revealSessionProject: (row) => row.project_id && selectProject(row.project_id),
      // AJ-E1702/E1703: pools.
      moveSessionToPool: (row) => openPoolSheet({ mode: "move", sessionId: row.id }),
      openPool: selectPool,
      stopSession,
      newSessionIn,
      setProjectAccount: (project) => selectProject(project.id),
      openAccount: (account) => selectAccount(account.id),
      openTool: (tool) => selectTool(tool.id),
      toolAccounts: () => openAccounts(),
      copyUpdateCommand: (tool) => copy(tool.install_command ?? "", `Copied ${tool.display_name}'s update command. Paste it into a terminal.`),
    };
  }, [selectConversation, conversations, selectSession, selectProject, selectAccount, selectTool, selectPool, openAccounts, openHome, stopSession, newSessionIn, refresh]);
  return <ActionHostProvider host={host}>{children}</ActionHostProvider>;
}
