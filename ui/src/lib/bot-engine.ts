/**
 * AJ-E3825 — Bot's engine in the window: its status (the header's
 * "Apple on-device · Running"), its lazy start when Bot opens with an AI
 * chosen ("Starting Apple's on-device model…"), refreshed while Bot is
 * open and after each answer. One small store, so the header, the empty
 * state and the usage bar agree.
 */
import { useEffect, useSyncExternalStore } from "react";
import { aiEngineStart, aiEngineStatus, type AiEngineStatusInfo } from "./ipc/ai-engine";

interface EngineView {
  status: AiEngineStatusInfo | null;
  /** A start is under way (before the backend says so). */
  starting: boolean;
}

let view: EngineView = { status: null, starting: false };
const listeners = new Set<() => void>();

function set(next: Partial<EngineView>): void {
  view = { ...view, ...next };
  for (const listen of listeners) listen();
}

function subscribe(listen: () => void): () => void {
  listeners.add(listen);
  return () => listeners.delete(listen);
}

/** Reads the status again (starts nothing). */
export async function refreshEngine(): Promise<void> {
  try {
    set({ status: await aiEngineStatus("chat") });
  } catch {
    // The header keeps what it had; the next look tries again.
  }
}

/** Starts Bot's engine when it isn't running (lazily: Bot's first use). */
export async function startEngine(): Promise<void> {
  if (view.starting) return;
  set({ starting: true });
  try {
    set({ status: await aiEngineStart("chat"), starting: false });
  } catch {
    set({ starting: false });
  }
}

/** Tests: back to nothing known. */
export function resetEngineView(): void {
  view = { status: null, starting: false };
}

/** How often the open panel looks again (idle stop, a failure). */
const LOOK_MS = 10_000;

/** Bot's engine; with `ready` (an AI chosen), started once on open and
 * looked at again while open. */
export function useBotEngine(ready: boolean): EngineView {
  useEffect(() => {
    if (!ready) return () => undefined;
    void startEngine();
    const timer = window.setInterval(() => void refreshEngine(), LOOK_MS);
    return () => window.clearInterval(timer);
  }, [ready]);
  return useSyncExternalStore(subscribe, () => view, () => view);
}
