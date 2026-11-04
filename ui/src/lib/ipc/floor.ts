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
import { invoke, listen } from "./transport";
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

/** The company now, as the Floor draws it. */
export async function floorSnapshot(): Promise<FloorSnapshot> {
  if (!hasLiveBackend()) return (await import("../../preview/floor")).previewFloor();
  return invoke<FloorSnapshot>("floor_snapshot");
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
