/**
 * AJ-E1408: the Mac's overload notices on screen — one shared list the
 * notification region reads and the alert poll writes (told once, kept
 * while their condition is on). Moved from NotificationRegion (the size
 * budget).
 */
import type { MachineNotice } from "./machine";

let notices: MachineNotice[] = [];
const listeners = new Set<(notices: MachineNotice[]) => void>();

/** The notices now. */
export function machineNotices(): MachineNotice[] {
  return notices;
}

export function publishMachineNotices(next: MachineNotice[]): void {
  notices = next;
  for (const listener of listeners) listener(next);
}

/** Hears every change, from the current list on; returns its undo. */
export function subscribeMachineNotices(listener: (notices: MachineNotice[]) => void): () => void {
  listeners.add(listener);
  listener(notices);
  return () => listeners.delete(listener);
}
