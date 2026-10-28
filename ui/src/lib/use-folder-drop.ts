/**
 * wf10, §4.8 — a folder dropped onto an open sheet (New session, New
 * project) is that sheet's folder. The window's own drop (the app's
 * native one, which carries full paths) reaches the open sheet; a drop
 * the page itself sees takes a `file://` address when the browser gives
 * one. The highlight shows only while something is over the sheet. Both
 * sheets use this one hook (the audit of 2026-10-27: the sheet lit up
 * and dropped nothing).
 */
import { useEffect, useRef, useState, type DragEvent } from "react";
import { onFolderDropped } from "./bridge";

/** A dropped item's path, from the page's own drop (`text/uri-list`). */
export function droppedPath(data: Pick<DataTransfer, "getData"> | null): string | null {
  const list = data?.getData("text/uri-list") ?? "";
  const first = list
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => line.length > 0 && !line.startsWith("#"));
  if (!first?.startsWith("file://")) return null;
  try {
    const path = decodeURIComponent(new URL(first).pathname);
    return path.length > 1 ? path.replace(/\/$/, "") : null;
  } catch {
    return null;
  }
}

export function useFolderDrop(onFolder: (path: string) => void) {
  const [dragging, setDragging] = useState(false);
  // The app's own drop: while the sheet is open, its folder.
  useEffect(
    () =>
      onFolderDropped((path) => {
        setDragging(false);
        onFolder(path);
      }),
    // The sheet's setter is stable for its life.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );
  return {
    dragging,
    zone: {
      onDragOver: (event: DragEvent) => {
        event.preventDefault();
        setDragging(true);
      },
      onDragLeave: () => setDragging(false),
      onDrop: (event: DragEvent) => {
        event.preventDefault();
        setDragging(false);
        const path = droppedPath(event.dataTransfer);
        if (path) onFolder(path);
      },
    },
  };
}

/** A folder dropped on the window itself: `onFolder` takes it unless an
 * open sheet does (`sheetOpen`; that sheet's own `useFolderDrop` has it). */
export function useWindowFolderDrop(onFolder: (path: string) => void, sheetOpen: boolean): void {
  const latest = useRef({ onFolder, sheetOpen });
  latest.current = { onFolder, sheetOpen };
  useEffect(() => onFolderDropped((path) => !latest.current.sheetOpen && latest.current.onFolder(path)), []);
}
