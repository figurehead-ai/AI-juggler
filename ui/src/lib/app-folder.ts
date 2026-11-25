/**
 * AJ-E2327 — an App's tool folder in words: what Add's confirmation says
 * it brought in (where it is, never copied) and what told which folder it
 * is; an App whose sessions can't be shown says why, and offers "Choose
 * its folder…"; the calm grey of an App's icon (D-117) says what it means
 * wherever it's drawn.
 */
import type { PlaceFolderInfo } from "../types/generated/PlaceFolderInfo";

/** The tool whose folder it is ("Claude Code"), else plain "tool". */
function toolOf(folder: PlaceFolderInfo | null | undefined): string {
  return folder?.tool ?? "tool";
}

/** Whether an App has no folder linked, and says why. */
export function unlinked(folder: PlaceFolderInfo | null | undefined): folder is PlaceFolderInfo & { why: string } {
  return !!folder && folder.name === null && typeof folder.why === "string" && folder.why !== "";
}

/** What told which folder it is, as a clause ("its open sessions write
 * there"). */
export function folderByWords(by: string | null | undefined): string | null {
  switch (by) {
    case "running":
      return "its open sessions write there";
    case "records":
      return "its app's own list of its sessions is there";
    case "recorded":
      return "AI Juggler starts it with this folder";
    case "declared":
      return "it's its app's own folder";
    case "named":
      return "it's named like the profile: a guess, checked when it runs";
    default:
      return null;
  }
}

/** Add's confirmation: the App added, and the folder that came with it
 * (or why none did, and what to do). */
export function addedNote(label: string, folder: PlaceFolderInfo | null | undefined): { message: string; detail: string | null; unlinked: boolean } {
  const message = `Added ${label}`;
  if (folder?.name) {
    const by = folderByWords(folder.by);
    return {
      message,
      detail: `Also uses its ${toolOf(folder)} folder ${folder.name}, where it is, so its sessions show here${by ? ` (${by})` : ""}.`,
      unlinked: false,
    };
  }
  if (unlinked(folder)) {
    return { message, detail: `${folder.why} Choose its folder to see its sessions.`, unlinked: true };
  }
  return { message, detail: null, unlinked: false };
}

/** The App's row when its sessions can't be shown: its short words. */
export const NOT_SHOWN = "sessions not shown";

/** D-117's calm tone, said (the icon's tooltip): an App whose sessions
 * can't be shown says why; a muted one at rest, what muted means. `null`
 * when it's in full colour (its name is its tooltip then). */
export function toneTip(lit: boolean, folder?: PlaceFolderInfo | null): string | null {
  if (unlinked(folder)) return `Its sessions can't be shown: ${folder.why}`;
  if (lit) return null;
  return "Muted while nothing here needs you; in full colour when something does, or when you point at it.";
}
