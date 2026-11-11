/**
 * AJ-E1537 — what App gives every menu: opening a session, project,
 * account or tool, focusing a session, carrying it to another account,
 * stopping it, copying its resume command, and starting a new session in
 * a project. Screens below add their own (the rail, Accounts, History).
 */
import { useMemo, type JSX, type ReactNode } from "react";
import { ActionHostProvider } from "./ItemMenu";
import { showToast } from "./Toast";
import { showGoTo } from "../lib/go-to";
import type { ActionHost } from "../lib/actions";
import { desktopFocus } from "../lib/ipc/desktop";
import { errorText } from "../lib/bridge";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { sessionsResumeCommand } from "../lib/ipc/sessions";
import { conversationsArchive, conversationsContinue, conversationsGoTo, conversationsResumeCommand, switchSourceOf, type ConversationInfo } from "../lib/ipc/conversations";
import { stillListedWords } from "../lib/archive-words";
import { lastDesktopApp, useKnownPlaces } from "../lib/places";
import { refreshSessionLists } from "../lib/prefetch-data";
import { useConversations } from "../lib/conversations";
import { CONTINUED_HERE, openPoolSheet, requestNewPool } from "../lib/pools";
import { markSeen } from "../lib/seen";
import { useProjectManage } from "./ManageWork";
import { focusHandoverSoon } from "../lib/handover-focus";
import { continueWhy, resumeRefusalWords, type ResumeRefusalInfo } from "../lib/resume-refusal";

async function copy(text: Promise<string> | string, done: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(await text);
    showToast({ id: "copied", message: done });
  } catch (cause) {
    showToast({ id: "copied", message: `Couldn't copy: ${errorText(cause)}` });
  }
}

/** rc21: a resume command its tool won't open (a copy an older AI
 * Juggler made) is never handed out: the toast says why instead. */
function copyResume(refusal: ResumeRefusalInfo | null | undefined, command: () => Promise<string>): Promise<void> | void {
  const why = resumeRefusalWords(refusal);
  if (why) {
    showToast({ id: "copied", tone: "warning", message: "No resume command to copy", detail: why });
    return;
  }
  return copy(command(), "Copied the resume command. Paste it into a terminal.");
}

export function AppActions({
  selectConversation,
  selectSession,
  selectProject,
  selectAccount,
  selectTool,
  openToolSettings,
  selectPool,
  openAccounts,
  openHome,
  openPools,
  stopSession,
  newSessionIn,
  refresh,
  children,
}: {
  /** Its page, from its ids (one not loaded opens all the same). */
  selectConversation: (conversation: Pick<ConversationInfo, "account_id" | "id" | "project_id">) => void;
  selectSession: (id: string) => void;
  selectProject: (id: string) => void;
  selectAccount: (id: string) => void;
  selectTool: (id: string) => void;
  /** A tool's install, version and updates, in Settings. */
  openToolSettings: () => void;
  /** AJ-E1703: a pool's page. */
  selectPool: (id: string) => void;
  openAccounts: () => void;
  openHome: () => void;
  /** The Pools screen. */
  openPools?: () => void;
  stopSession: (row: InstanceInfo) => void;
  newSessionIn: (project: ProjectInfo) => void;
  refresh: () => void;
  children: ReactNode;
}): JSX.Element {
  const conversations = useConversations();
  const places = useKnownPlaces();
  // AJ-E2804 R4: one project menu everywhere (the Projects screen, a
  // project's page, ⌘K and the rail): rename, show in Finder, forget…
  // and their sheets live here, not in one surface.
  const projectManage = useProjectManage(refresh);
  const manageHost = projectManage.host;
  const host = useMemo<ActionHost>(() => {
    // The same as ⌘⇧H on the session: its switch list.
    const carry = (row: InstanceInfo) => {
      selectSession(row.id);
      focusHandoverSoon(row.id);
    };
    return {
      ...manageHost,
      ...(openPools
        ? {
            keepProjectApart: (project: ProjectInfo) => {
              requestNewPool({ template: "sealed-client", project: project.id });
              openPools();
            },
          }
        : {}),
      openProject: (project) => selectProject(project.id),
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
      copyResumeCommand: (row) => copyResume(row.resume_refusal, () => sessionsResumeCommand(row.id)),
      // AJ-E1529: conversations started outside AI Juggler.
      // AJ-E2107: opening or going to it is looking at it (the seen mark).
      openConversation: (c) => {
        markSeen(c.account_id, c.id);
        selectConversation(c);
      },
      // AJ-E3104: a notice's conversation not loaded yet (past the list).
      openConversationById: (accountId, id, projectId) => {
        markSeen(accountId, id);
        selectConversation({ account_id: accountId, id, project_id: projectId });
      },
      // AJ-E2102: feedback at once, then what really happened.
      goToConversation: async (c) => {
        markSeen(c.account_id, c.id);
        showToast({ id: "go-to", message: "Going to it…" });
        try {
          showGoTo(await conversationsGoTo(c.account_id, c.id), () => conversationsGoTo(c.account_id, c.id, null, true));
        } catch (cause) {
          showToast({ id: "go-to", message: `Couldn't go to it: ${errorText(cause)}` });
        }
      },
      // F8: another app's session too, from its row menu and ⌘K.
      continueConversationElsewhere: (c) => {
        markSeen(c.account_id, c.id);
        selectConversation(c);
        focusHandoverSoon(switchSourceOf(c));
      },
      archiveConversation: async (c, archived) => {
        try {
          await conversationsArchive(c.account_id, c.id, archived);
          conversations.refresh();
          refreshSessionLists();
          // The desktop app it was last open in (a closed one's row names
          // only its tool).
          const app = lastDesktopApp(places, c.account_id, c.tool, c.id);
          const still = stillListedWords(c, app);
          showToast({
            id: "archive-here",
            // Short, with its consequence under it (the rc8 walk: one long
            // line ran over Undo).
            message: archived ? "Archived in AI Juggler" : "Unarchived",
            detail: `${still.charAt(0).toUpperCase()}${still.slice(1)}.`,
            undo: {
              onUndo: async () => {
                await conversationsArchive(c.account_id, c.id, !archived);
                conversations.refresh();
                refreshSessionLists();
              },
            },
          });
        } catch (cause) {
          showToast({ id: "archive-here", message: `Couldn't ${archived ? "archive" : "unarchive"} it: ${errorText(cause)}` });
        }
      },
      copyConversationResume: (c) => copyResume(c.resume_refusal, () => conversationsResumeCommand(c.account_id, c.id)),
      continueConversation: async (c) => {
        // rc21: one its tool won't open by its id starts nothing; rc31: nor
        // one open in another session, and the toast says why.
        const refused = continueWhy(c);
        if (refused) {
          showToast({ id: "continue", tone: "warning", message: "Couldn't continue it here", detail: refused });
          return;
        }
        try {
          const launched = await conversationsContinue(c.account_id, c.id);
          refresh();
          conversations.refresh();
          selectSession(launched.session_id);
          showToast({ id: "continue", message: CONTINUED_HERE });
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
      openToolSettings,
      toolAccounts: () => openAccounts(),
      copyUpdateCommand: (tool) => copy(tool.install_command ?? "", `Copied ${tool.display_name}'s update command. Paste it into a terminal.`),
    };
  }, [manageHost, selectConversation, conversations, places, selectSession, selectProject, selectAccount, selectTool, openToolSettings, selectPool, openAccounts, openHome, openPools, stopSession, newSessionIn, refresh]);
  return (
    <ActionHostProvider host={host}>
      {children}
      {projectManage.element}
    </ActionHostProvider>
  );
}
