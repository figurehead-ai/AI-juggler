/**
 * AJ-E3505 — the Floor's snapshot over IPC (ENGINEERING-GUIDE U3: one thin
 * file per feature). The backend builds one snapshot of the company for
 * the Floor window and the main window's Floor list, and says when it
 * changed (`floor://changed`, with the new one). Its first read starts
 * the backend keeping it current.
 *
 * In the preview (no backend) the same calls answer from `previewFloor`,
 * and an `aij:floor-changed` window event stands in for the backend's.
 */
import { call, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { FloorAccount } from "../../types/generated/FloorAccount";
import type { FloorApp } from "../../types/generated/FloorApp";
import type { FloorAppKind } from "../../types/generated/FloorAppKind";
import type { FloorMachine } from "../../types/generated/FloorMachine";
import type { FloorPool } from "../../types/generated/FloorPool";
import type { FloorSession } from "../../types/generated/FloorSession";
import type { FloorSnapshot } from "../../types/generated/FloorSnapshot";
import type { FloorState } from "../../types/generated/FloorState";

export type { FloorAccount, FloorApp, FloorAppKind, FloorMachine, FloorPool, FloorSession, FloorSnapshot, FloorState };

export const FLOOR_CHANGED = "floor://changed";
/** The Floor couldn't open (⌘⇧M or the tray): its words. */
export const FLOOR_UNAVAILABLE = "floor://unavailable";

/** The company now, as the Floor draws it. */
export async function floorSnapshot(): Promise<FloorSnapshot> {
  if (!hasLiveBackend()) return (await import("../../preview/floor")).previewFloor();
  return call("floor_snapshot");
}

/** Calls `on` with each new snapshot; returns the disposer. */
export function onFloorChanged(on: (snapshot: FloorSnapshot) => void): () => void {
  if (!hasLiveBackend()) {
    const handler = (event: Event) => on((event as CustomEvent<FloorSnapshot>).detail);
    window.addEventListener("aij:floor-changed", handler);
    return () => window.removeEventListener("aij:floor-changed", handler);
  }
  let disposed = false;
  let stop: (() => void) | null = null;
  void listen<FloorSnapshot>(FLOOR_CHANGED, (event) => on(event.payload)).then((unlisten) => {
    if (disposed) unlisten();
    else stop = unlisten;
  });
  return () => {
    disposed = true;
    stop?.();
  };
}

/** Opens the Floor's own window, or brings it forward (AJ-E3501). In the
 * preview there's no window to open: it says so. */
export async function floorOpen(): Promise<void> {
  if (!hasLiveBackend()) throw new Error("The Floor opens in the app, not in the preview.");
  return call("floor_open");
}

/** Calls `on` when the Floor couldn't open from ⌘⇧M or the tray. */
export function onFloorUnavailable(on: (words: string) => void): () => void {
  if (!hasLiveBackend()) return () => {};
  let disposed = false;
  let stop: (() => void) | null = null;
  void listen<string>(FLOOR_UNAVAILABLE, (event) => on(event.payload)).then((unlisten) => {
    if (disposed) unlisten();
    else stop = unlisten;
  });
  return () => {
    disposed = true;
    stop?.();
  };
}

/** Hands the backend a floor logo the window drew for an App (AJ-E3508). */
export async function floorLogoPut(appId: string, print: string, png: Uint8Array): Promise<void> {
  if (!hasLiveBackend()) return;
  return call("floor_logo_put", { appId, print, png: Array.from(png) });
}
