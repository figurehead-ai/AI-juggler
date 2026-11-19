/**
 * E14 machine health (AJ-E1401): how the Mac is doing, for Home's This Mac
 * panel, the launch advisor and the overload alerts. The calls (and their
 * preview mocks) are in `ipc/this-mac`; this module holds the hooks and the
 * words.
 */
import { createContext, createElement, useContext, useEffect, useLayoutEffect, useState, useSyncExternalStore, type ReactNode } from "react";
import { hasLiveBackend } from "./bridge";
import { onThisMac, thisMac } from "./ipc/this-mac";
import { memorySize } from "./compact";
import type { MachineAdviceInfo } from "../types/generated/MachineAdviceInfo";
import type { ThisMacInfo } from "../types/generated/ThisMacInfo";
import type { OverloadAlertInfo } from "../types/generated/OverloadAlertInfo";
import type { ToolTreeInfo } from "../types/generated/ToolTreeInfo";



/** Memory in plain words ("3.2 GB", "640 MB", "under 1 MB"): the app's
 * memory rule (`lib/compact`), with This Mac's floor — as the backend's
 * `memory_words` says it. */
export function bytesWords(bytes: number): string {
  return bytes < 1024 ** 2 ? "under 1 MB" : memorySize(bytes);
}

// --- AJ-E1403: This Mac ------------------------------------------------------


export type { ThisMacInfo, ToolTreeInfo, OverloadAlertInfo };


/**
 * This Mac, kept current by the backend's 5 s `machine://this-mac`
 * emission (no UI timer). `refreshKey` re-reads it on demand — the
 * preview has no events, so callers pass something that changes with
 * the sessions on screen.
 */
function useThisMac(refreshKey?: unknown): ThisMacInfo | null {
  const [info, setInfo] = useState<ThisMacInfo | null>(null);
  useEffect(() => {
    let alive = true;
    void thisMac().then((data) => alive && setInfo(data), () => null);
    return () => {
      alive = false;
    };
    // Live, the event keeps it current; the preview re-reads with the
    // sessions on screen.
  }, [hasLiveBackend() ? 0 : refreshKey]);
  useEffect(() => {
    if (!hasLiveBackend()) return;
    let alive = true;
    const stop = onThisMac((payload) => alive && setInfo(payload));
    return () => {
      alive = false;
      stop();
    };
  }, []);
  return info;
}

/** "CPU 12% · 1.1 GB · 3 helpers" for a live session ("—" where unmeasured). */
export function resourceLine(cpu: number | null, rss: number | null, helpers: number | null): string {
  const parts = [
    `CPU ${cpu !== null ? `${Math.round(cpu)}%` : "—"}`,
    rss !== null ? bytesWords(rss) : "memory —",
    helpers === null ? "more processes —" : helpers === 1 ? "1 more process" : `${helpers} more processes`,
  ];
  return parts.join(" · ");
}

/** This Mac for components deep in the tree: the app provides its one
 * subscription here. Every reader draws again on each reading; a reader of
 * one part of it (a rail row's heavy note) uses the store below. */
export const ThisMacContext = createContext<ThisMacInfo | null>(null);

/** The same reading as a store: a reader of one part of it draws again
 * only when that part changes (AJ-E805: each 5 s reading drew every rail
 * row again for its heavy note). */
interface ThisMacStore {
  get: () => ThisMacInfo | null;
  subscribe: (listener: () => void) => () => void;
  set: (info: ThisMacInfo | null) => void;
}

function thisMacStore(first: ThisMacInfo | null): ThisMacStore {
  let current = first;
  const listeners = new Set<() => void>();
  return {
    get: () => current,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    set: (info) => {
      if (info === current) return;
      current = info;
      for (const listener of listeners) listener();
    },
  };
}

const ThisMacStoreContext = createContext<ThisMacStore>(thisMacStore(null));

/** Gives `info` to what reads This Mac below: whole (`ThisMacContext`) or
 * one part at a time (`useHeavyNote`). */
export function ThisMacValue({ info, children }: { info: ThisMacInfo | null; children?: ReactNode }): ReactNode {
  const [store] = useState(() => thisMacStore(info));
  useLayoutEffect(() => store.set(info), [store, info]);
  return createElement(ThisMacStoreContext.Provider, { value: store }, createElement(ThisMacContext.Provider, { value: info }, children));
}

/** This Mac as given (a test, the preview), else the app's one
 * subscription (AJ-E805). */
export function useThisMacInfo(given?: ThisMacInfo | null): ThisMacInfo | null {
  const shown = useContext(ThisMacContext);
  return given === undefined ? shown : given;
}

/** The app's one This Mac subscription, given to what reads it. AJ-E805:
 * it was the shell's state, so each 5 s reading (while an AI tool runs)
 * drew the whole app again, the rail included; now only what reads This
 * Mac draws again. `refreshKey`: as `useThisMac`'s. */
export function ThisMacProvider({ refreshKey, children }: { refreshKey?: unknown; children: ReactNode }): ReactNode {
  return createElement(ThisMacValue, { info: useThisMac(refreshKey) }, children);
}

/** How each level of the Mac's state looks: This Mac's glyph and tone,
 * the rail footer's dot. */
const MACHINE_LEVEL: Record<string, { glyph: string; tone: string; dot: string }> = {
  plenty: { glyph: "●", tone: "text-success", dot: "bg-success" },
  tight: { glyph: "▲", tone: "text-warning", dot: "bg-warning" },
  pressure: { glyph: "■", tone: "text-danger", dot: "bg-danger" },
};

/** The Mac's state, from one place: This Mac's summary (which the backend
 * aligns with its alerts). This Mac, the rail footer and the collapsed
 * rail all read it, so they never disagree (QA, P19: "under memory
 * pressure", "Getting tight", "healthy" and "All clear" at once). `null`
 * before the first sample. */
export function machineStatus(
  info: ThisMacInfo | null,
): { level: string; words: string; cause: string | null; glyph: string; tone: string; dot: string; calm: boolean } | null {
  if (!info || !info.machine.sampled) return null;
  const summary = info.machine.summary;
  const level = summary.level;
  const look = MACHINE_LEVEL[level] ?? MACHINE_LEVEL.plenty!;
  // The worst reason in a few words ("swap almost full"): a level is
  // never said without what put it there (D1 on rc29).
  const cause = level === "plenty" ? null : (summary.cause ?? null);
  return { level, words: summary.words, cause, calm: level === "plenty", ...look };
}

/** This Mac's summary level alone ("plenty", "tight", "pressure"; `null`
 * before the first reading), from `given` (a test, the preview) or the
 * app's one subscription. AJ-E813: Home reads only this, yet it drew
 * itself again on every 5 s reading; its reader draws again only when the
 * level changes. */
export function useMachineLevel(given?: ThisMacInfo | null): string | null {
  const store = useContext(ThisMacStoreContext);
  const level = () => levelOf(store.get());
  const fromStore = useSyncExternalStore(store.subscribe, level, level);
  return given === undefined ? fromStore : levelOf(given);
}

function levelOf(info: ThisMacInfo | null | undefined): string | null {
  return info?.machine.summary.level ?? null;
}

/** A session's heavy note ("CPU 95%", "3.2 GB"), only while it's heavy.
 * Its row draws again only when its own note changes. */
export function useHeavyNote(sessionId: string): string | null {
  const store = useContext(ThisMacStoreContext);
  const note = () => store.get()?.heavy.find((tree) => tree.session_id === sessionId)?.heavy_note ?? null;
  return useSyncExternalStore(store.subscribe, note, note);
}

/** Reasons joined as one sentence ("macOS …" keeps its lower-case m). */
export function reasonSentence(words: string[]): string {
  const joined = words.join("; ");
  return joined.startsWith("macOS") ? joined : joined.replace(/^./, (first) => first.toUpperCase());
}

/** The rail's short form of a heavy note: one figure (memory first). */
export function shortHeavy(note: string): string {
  const parts = note.split(" · ");
  return parts[parts.length - 1] ?? note;
}

// --- AJ-E1407: the machine launch advisor ---------------------------------

export type { MachineAdviceInfo };

// The alerts' words live with the memory parts they name.
export { alertNextStep, alertSubject, alertWords } from "./machine-memory";
