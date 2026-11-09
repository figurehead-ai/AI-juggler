/**
 * E14 machine health (AJ-E1401): how the Mac is doing, for Home's This Mac
 * panel, the launch advisor and the overload alerts. The calls (and their
 * preview mocks) are in `ipc/this-mac`; this module holds the hooks and the
 * words.
 */
import { createContext, useContext, useEffect, useState } from "react";
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
export function useThisMac(refreshKey?: unknown): ThisMacInfo | null {
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
    let unlisten: (() => void) | null = null;
    let alive = true;
    void onThisMac((payload) => alive && setInfo(payload)).then((stop) => {
      if (alive) unlisten = stop;
      else stop();
    });
    return () => {
      alive = false;
      unlisten?.();
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

/** This Mac for components deep in the tree (the rail rows): the app
 * provides its one subscription here. */
export const ThisMacContext = createContext<ThisMacInfo | null>(null);

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
): { level: string; words: string; glyph: string; tone: string; dot: string; calm: boolean } | null {
  if (!info || !info.machine.sampled) return null;
  const level = info.machine.summary.level;
  const look = MACHINE_LEVEL[level] ?? MACHINE_LEVEL.plenty!;
  return { level, words: info.machine.summary.words, calm: level === "plenty", ...look };
}

/** A session's heavy note ("CPU 95%", "3.2 GB"), only while it's heavy. */
export function useHeavyNote(sessionId: string): string | null {
  const info = useContext(ThisMacContext);
  return info?.heavy.find((tree) => tree.session_id === sessionId)?.heavy_note ?? null;
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
