/**
 * The Go-to palette's small helpers (split from GoToPalette.tsx, the
 * size budget).
 */
import type { RemoteMachineInfo } from "../lib/bridge";

/** A remote focus is offered only where that machine allows control and
 * is connected now (it decides again when the request arrives). */
export function canFocusRemote(machine: RemoteMachineInfo): boolean {
  return machine.permission === "control" && machine.connection === "connected";
}

/** Option ids must be valid DOM ids. */
export function domId(id: string): string {
  return id.replace(/[^A-Za-z0-9_-]/g, "_");
}
