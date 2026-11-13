/**
 * The embedded terminal (M1-4): spawn, attach, write, resize, kill and
 * its output events.
 */
import { call, listen, liveBackend as hasBackend } from "./transport";
import { orderedJoin, orderedLatest } from "./ordered";
import type { TerminalInfo } from "../../types/generated/TerminalInfo";


export const TERMINAL_EVENTS = {
  output: "terminal://output",
  exit: "terminal://exit",
} as const;

/** Spawns the guided login for an account inside an embedded terminal. */
export async function terminalSpawnLogin(accountId: string): Promise<TerminalInfo> {
  if (!hasBackend) return { id: `login-preview-${accountId}` };
  return call("terminal_spawn_login", { accountId });
}

/** Starts streaming a supervised session into the dock (idempotent). */
export async function terminalAttach(id: string): Promise<TerminalInfo> {
  return call("terminal_attach", { id });
}

/** Sends keystrokes to a terminal, in the order they were typed. */
export const terminalWrite = orderedJoin((id, data) => call("terminal_write", { id, data }));

const resizeTerminal = orderedLatest<{ cols: number; rows: number }>((id, size) => call("terminal_resize", { id, ...size }));

/** Resizes a terminal's PTY (the newest size wins). */
export async function terminalResize(id: string, cols: number, rows: number): Promise<void> {
  return resizeTerminal(id, { cols, rows });
}

/** The design preview's seams for its terminals. */
type Seams = { __AIJ_TERMINAL_KILL_CALLS__?: string[]; __AIJ_TERMINAL_KEPT__?: string[] };
function seams(): Seams {
  return window as unknown as Seams;
}

/** Stops a terminal's process (graceful ladder). */
export async function terminalKill(id: string): Promise<void> {
  if (!hasBackend) {
    // AJ-E1503: the seam that proves a held stop is (or isn't) sent.
    const w = seams();
    w.__AIJ_TERMINAL_KILL_CALLS__ = [...(w.__AIJ_TERMINAL_KILL_CALLS__ ?? []), id];
    return;
  }
  return call("terminal_kill", { id });
}

/** The retained output tail (base64) for replay on panel reopen. */
export async function terminalSnapshot(id: string): Promise<string> {
  return call("terminal_snapshot", { id });
}

/** P2: whether an ended session's terminal kept output to replay. The
 * tail is kept in memory while AI Juggler runs, so one that ended before
 * it last started has none (its snapshot is refused or empty). The
 * design preview keeps none, but for the ids its seam names
 * (`__AIJ_TERMINAL_KEPT__`). */
export async function terminalKept(id: string): Promise<boolean> {
  if (!hasBackend) return (seams().__AIJ_TERMINAL_KEPT__ ?? []).includes(id);
  try {
    return (await terminalSnapshot(id)).length > 0;
  } catch {
    return false;
  }
}

/** Live output stream for one terminal (base64 chunks). */
export function onTerminalOutput(
  id: string,
  handler: (base64: string) => void,
): () => void {
  return listenScoped(TERMINAL_EVENTS.output, (payload: { id: string; data: string }) => {
    if (payload.id === id) handler(payload.data);
  });
}

/** A terminal's process exited. */
export function onTerminalExit(handler: (id: string) => void): () => void {
  return listenScoped(TERMINAL_EVENTS.exit, (payload: { id: string }) =>
    handler(payload.id),
  );
}

export function listenScoped<T>(
  event: string,
  handler: (payload: T) => void,
): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<T>(event, (event_) => handler(event_.payload)).then((stop) => {
    if (disposed) {
      stop();
    } else {
      unlisten = stop;
    }
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}
