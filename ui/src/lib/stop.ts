/**
 * AJ-E1503 Req.2 — stopping one session is held for the undo window,
 * then sent: Undo in time and the stop never reaches the session. Every
 * single-session Stop (the inspector's button and Actions row, ⌘., and a
 * paired Mac's session's "Stop on <Mac>") comes through here. "Stop all" is different: it asks first and cannot be
 * undone (the maintainer's plan, 2026-10-21).
 */
import { useEffect, useState, useSyncExternalStore } from "react";

import { holdAction, isHeld, toastStore } from "../components/Toast";
import { terminalKill } from "./ipc/terminal-io";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { cliAppName } from "./app-name";

/** The undo window for a held stop, in seconds. */
export const STOP_HOLD_SECONDS = 5;

/** A session in words: its project and account ("api · Work Max"). */
export function sessionLabel(session: InstanceInfo, projects: ProjectInfo[]): string {
  const project = projects.find((row) => row.id === session.project_id)?.name;
  // Its App by its full name (the 0.4 review: "ai-juggler · Work Max").
  const app = cliAppName(session.provider, session.account_label);
  return project ? `${project} · ${app}` : app;
}

/** Holds the stop of one session; `after` runs once it is sent. `send`
 * replaces this Mac's own stop (a paired Mac's session is stopped there). */
export function holdStop(session: { id: string; label: string; send?: () => Promise<unknown> }, after?: () => void): void {
  holdAction({
    id: `stop:${session.id}`,
    message: `Stopping ${session.label}.`,
    detail: "Undo keeps it running.",
    seconds: STOP_HOLD_SECONDS,
    testId: "stop-undo",
    undone: `${session.label} keeps running.`,
    failed: (cause) => `Couldn’t stop ${session.label}: ${cause}`,
    run: async () => {
      // Before it's sent: its ended row can arrive before the call returns.
      stoppedAt.set(session.id, Date.now());
      await (session.send ? session.send() : terminalKill(session.id));
      after?.();
    },
  });
}

/** How long the button that takes Stop's place (Resume) stays unarmed
 * after a stop is sent from here. */
export const RESUME_SETTLE_MS = 2_000;

/** When each stop was sent from here, by session id. */
const stoppedAt = new Map<string, number>();

const subscribe = (listener: () => void) => toastStore.subscribe(listener);

/** The rc33 R3 walk: a Grok session came back 6 s after its Stop. The
 * held stop left the button reading "Stop" on a working session for its
 * 5 s, and once it landed Resume took that very place, so a second press
 * on Stop resumed it. While held, it says so and can't be pressed again. */
export function useStopHeld(sessionId: string): boolean {
  const read = () => isHeld(`stop:${sessionId}`);
  return useSyncExternalStore(subscribe, read, read);
}

/** True for [`RESUME_SETTLE_MS`] after a stop was sent from here: the
 * press meant for Stop never resumes what it stopped. */
export function useJustStopped(sessionId: string): boolean {
  const left = () => RESUME_SETTLE_MS - (Date.now() - (stoppedAt.get(sessionId) ?? -Infinity));
  const [settling, setSettling] = useState(() => left() > 0);
  useEffect(() => {
    const wait = left();
    setSettling(wait > 0);
    if (wait <= 0) return undefined;
    const timer = setTimeout(() => setSettling(false), wait);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- read once per session
  }, [sessionId]);
  return settling;
}
