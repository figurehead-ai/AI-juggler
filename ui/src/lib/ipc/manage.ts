/**
 * AJ-E1502 — naming, pinning and tidying sessions and projects, and the
 * editor projects open in (ENGINEERING-GUIDE U3: one IPC module per
 * feature). In preview mode each call changes the preview's own rows.
 */
import { invoke } from "@tauri-apps/api/core";
import { hasLiveBackend, MOCK_PROJECTS, MOCK_SESSIONS, pickDirectory, refused } from "../bridge";
import type { EditorChoicesInfo } from "../../types/generated/EditorChoicesInfo";

/** AJ-E1502: name a session (`null` or blank: its own title again). */
export async function sessionRename(sessionId: string, label: string | null): Promise<void> {
  if (!hasLiveBackend()) {
    const row = MOCK_SESSIONS.find((session) => session.id === sessionId);
    if (!row) throw { code: "not_found", retryable: false, user_message: "That session isn't there any more." };
    row.label = label && label.trim() !== "" ? label.trim() : undefined;
    return;
  }
  return invoke<void>("session_rename", { sessionId, label });
}

/** AJ-E1502: pin a session to the top of its group, or unpin it. */
export async function sessionSetPinned(sessionId: string, pinned: boolean): Promise<void> {
  if (!hasLiveBackend()) {
    const row = MOCK_SESSIONS.find((session) => session.id === sessionId);
    if (row) row.pinned = pinned ? true : undefined;
    return;
  }
  return invoke<void>("session_set_pinned", { sessionId, pinned });
}

/** AJ-E1502: rename a project — its label; the folder is never renamed.
 * Blank: the folder's name again. */
export async function projectRename(projectId: string, name: string): Promise<void> {
  if (!hasLiveBackend()) {
    const project = MOCK_PROJECTS.find((row) => row.id === projectId);
    if (project) project.name = name.trim() !== "" ? name.trim() : (project.root.split("/").pop() ?? project.root);
    return;
  }
  return invoke<void>("project_rename", { projectId, name });
}

/** AJ-E1502: pin a project to the top of the rail, or unpin it. */
export async function projectSetPinned(projectId: string, pinned: boolean): Promise<void> {
  if (!hasLiveBackend()) {
    const project = MOCK_PROJECTS.find((row) => row.id === projectId);
    if (project) project.pinned = pinned ? true : undefined;
    return;
  }
  return invoke<void>("project_set_pinned", { projectId, pinned });
}

/** AJ-E1502: forget a project (hidden until it is used again), or bring
 * it back. Refused while a session there is open. */
export async function projectForget(projectId: string, forgotten: boolean): Promise<void> {
  if (!hasLiveBackend()) {
    const project = MOCK_PROJECTS.find((row) => row.id === projectId);
    if (!project) return;
    const open = MOCK_SESSIONS.filter(
      (row) => row.project_id === projectId && ["starting", "running", "waiting_user", "stuck", "quota_blocked"].includes(row.status),
    );
    if (forgotten && open.length > 0) {
      const names = [...new Set(open.map((row) => row.git_branch ?? ((row.cwd ?? "").split("/").pop() || row.account_label)))].sort().join(", ");
      throw refused(
        `${project.name} has ${open.length === 1 ? "a session" : `${open.length} sessions`} open: ${names}. Stop ${open.length === 1 ? "it" : "them"} first, then forget the project`,
      );
    }
    project.forgotten = forgotten ? true : undefined;
    if (forgotten) project.pinned = undefined;
    return;
  }
  return invoke<void>("project_forget", { projectId, forgotten });
}

/** AJ-E1502: point a project at the folder it moved to. The backend
 * asks for the folder in the system's own dialog (the path never comes
 * from here, SECURITY C1); `false` when the user cancelled. */
export async function projectRelocate(projectId: string): Promise<boolean> {
  if (!hasLiveBackend()) {
    const fail = (window as unknown as { __AIJ_RELOCATE_FAIL__?: string }).__AIJ_RELOCATE_FAIL__;
    if (fail) throw refused(fail);
    const folder = await pickDirectory();
    if (folder === null) return false;
    const project = MOCK_PROJECTS.find((row) => row.id === projectId);
    if (project) {
      project.root = folder;
      project.missing = false;
    }
    return true;
  }
  return invoke<boolean>("project_relocate", { projectId });
}

/** AJ-E1502: open a project in the editor chosen in Settings (else
 * Finder), or show it in Finder. */
export async function projectOpen(projectId: string, how: "editor" | "finder"): Promise<void> {
  if (!hasLiveBackend()) {
    const w = window as unknown as { __AIJ_OPENED__?: { projectId: string; how: string }[] };
    (w.__AIJ_OPENED__ ??= []).push({ projectId, how });
    return;
  }
  return invoke<void>("project_open", { projectId, how });
}

/** AJ-E1502: the editors this Mac has, and the one projects open in. */
export async function editorChoices(): Promise<EditorChoicesInfo> {
  if (!hasLiveBackend()) {
    const w = window as unknown as { __AIJ_EDITORS__?: EditorChoicesInfo };
    return (w.__AIJ_EDITORS__ ??= { installed: ["Visual Studio Code", "Zed"], chosen: null });
  }
  return invoke<EditorChoicesInfo>("editor_choices");
}

/** AJ-E1502: choose the editor projects open in (`null`: Finder). */
export async function editorSet(editor: string | null): Promise<void> {
  if (!hasLiveBackend()) {
    const current = await editorChoices();
    current.chosen = editor;
    return;
  }
  return invoke<void>("editor_set", { editor });
}
