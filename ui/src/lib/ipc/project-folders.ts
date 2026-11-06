/**
 * Projects and folders: the list, backfill, the default account, and
 * picking or dropping a folder.
 */
import { call, liveBackend as hasBackend, pickPath, readOnlyBackend } from "./transport";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { previewHooks } from "../preview-hooks";

/** Tracked projects — the juggling anchor (which instance is on each directory). */
export async function projectsList(): Promise<ProjectInfo[]> {
  if (!hasBackend) {
    return [...previewHooks().fixtures.MOCK_PROJECTS];
  }
  return call("projects_list");
}


/** Backfills the projects rail from imported transcript history (idempotent). */
export async function projectsBackfill(): Promise<number> {
  if (!hasBackend) return 0;
  return call("projects_backfill");
}

/** Native macOS folder picker; null when cancelled. */
export async function pickDirectory(): Promise<string | null> {
  if (!hasBackend) {
    // AJ-E307 test seam: specs inject the "picked" folder.
    return (
      (window as unknown as { __AIJ_PICK_DIRECTORY__?: string })
        .__AIJ_PICK_DIRECTORY__ ?? null
    );
  }
  return pickPath("folder");
}

/** Folders dragged onto the window (Tauri drag-drop events). */
export function onFolderDropped(handler: (path: string) => void): () => void {
  // No window to drop onto through the read-only test bridge.
  if (!hasBackend || readOnlyBackend) return () => {};
  let disposed = false;
  let unlisten: (() => void) | null = null;
  import("@tauri-apps/api/webview")
    .then(({ getCurrentWebview }) =>
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "drop") {
          for (const path of event.payload.paths) handler(path);
        }
      }),
    )
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}
