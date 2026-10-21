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

async function copy(text: Promise<string> | string, done: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(await text);
    showToast({ id: "copied", message: done });
  } catch (cause) {
    showToast({ id: "copied", message: `Couldn't copy: ${errorText(cause)}` });
  }
}

export function AppActions({
  selectSession,
  selectProject,
  selectAccount,
  selectTool,
  openAccounts,
  stopSession,
  newSessionIn,
  refresh,
  children,
}: {
  selectSession: (id: string) => void;
  selectProject: (id: string) => void;
  selectAccount: (id: string) => void;
  selectTool: (id: string) => void;
  openAccounts: () => void;
  stopSession: (row: InstanceInfo) => void;
  newSessionIn: (project: ProjectInfo) => void;
  refresh: () => void;
  children: ReactNode;
}): JSX.Element {
  const host = useMemo<ActionHost>(() => {
    // The same as ⌘⇧H on the session: its hand-over section.
    const carry = (row: InstanceInfo) => {
      selectSession(row.id);
      window.setTimeout(() => window.dispatchEvent(new Event("aij:focus-handover")), 0);
    };
    return {
      openSession: (row) => selectSession(row.id),
      openSessionById: selectSession,
      openHistory: (row) => selectSession(row.id),
      // A desktop session's own window; a terminal one's terminal here.
      focusSession: async (row) => {
        if (row.kind === "desktop") await desktopFocus(row.id).finally(refresh);
        selectSession(row.id);
      },
      continueSession: carry,
      continueHistory: carry,
      copyResumeCommand: (row) => copy(sessionsResumeCommand(row.id), "Copied the resume command. Paste it into a terminal."),
      revealSessionProject: (row) => row.project_id && selectProject(row.project_id),
      stopSession,
      newSessionIn,
      setProjectAccount: (project) => selectProject(project.id),
      openAccount: (account) => selectAccount(account.id),
      openTool: (tool) => selectTool(tool.id),
      toolAccounts: () => openAccounts(),
      copyUpdateCommand: (tool) => copy(tool.install_command ?? "", `Copied ${tool.display_name}'s update command. Paste it into a terminal.`),
    };
  }, [selectSession, selectProject, selectAccount, selectTool, openAccounts, stopSession, newSessionIn, refresh]);
  return <ActionHostProvider host={host}>{children}</ActionHostProvider>;
}
