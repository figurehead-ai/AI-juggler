/**
 * AJ-E1553 — the sign-ins, one list for the whole window: the setup
 * wizard's sign-in step, the rail's Sign-ins and a sign-in's own page read
 * the same entries, so a sign-in that finishes ticks the wizard's row,
 * leaves the rail and says "✓ signed in" at once, everywhere.
 *
 * Read once when first used, then kept by the backend's change events;
 * `refreshSignIns` reads again (a sign-in just started, the preview).
 */
import { useSyncExternalStore } from "react";
import { onSignInsChanged, signInsList, type SignInInfo } from "./ipc/sign-ins";

let current: SignInInfo[] = [];
let started = false;
const listeners = new Set<() => void>();

function set(next: SignInInfo[]): void {
  current = next;
  for (const listener of listeners) listener();
}

/** Reads the list again from the backend. */
export async function refreshSignIns(): Promise<void> {
  try {
    set(await signInsList());
  } catch (cause) {
    console.warn("AI Juggler: the sign-ins couldn't be read", cause);
  }
}

function start(): void {
  if (started) return;
  started = true;
  void onSignInsChanged(set);
  void refreshSignIns();
}

function subscribe(listener: () => void): () => void {
  start();
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Every sign-in, kept current. */
export function useSignIns(): SignInInfo[] {
  return useSyncExternalStore(subscribe, () => current);
}

/** The tests' reset: forgets what was read (the next use reads again). */
export function resetSignInsForTests(): void {
  current = [];
  started = false;
  listeners.clear();
}
