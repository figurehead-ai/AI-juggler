/**
 * This Mac's memory as a whole (the 0.1.4 review: "Under memory pressure"
 * beside "AI tools: about 16 GB of 103 GB" read as plenty left). What's
 * in use, split into the AI tools and everything else; what macOS keeps
 * compressed; memory pressure in plain words; and the overload alerts'
 * words, which name the other apps when they hold most of it. Moved out
 * of lib/machine.ts (the size budget).
 */
import { bytesWords, reasonSentence } from "./machine";
import type { MachineHealthInfo } from "../types/generated/MachineHealthInfo";
import type { OtherAppInfo } from "../types/generated/OtherAppInfo";
import type { OverloadAlertInfo } from "../types/generated/OverloadAlertInfo";
import type { ThisMacInfo } from "../types/generated/ThisMacInfo";

export type { OtherAppInfo };

/** Memory pressure in one line: shown beside the figures whenever macOS
 * reports it, so a free figure never stands alone against it. */
export const PRESSURE_SHORT = "Memory pressure is macOS's own measure of how hard it is working to make room.";

/** The rest, behind a disclosure. */
export const PRESSURE_MORE =
  "To make room, macOS compresses memory and moves some of it to disk (swap). Pressure can be high while some memory still looks free, and everything slows down while it lasts. Closing the apps that use the most memory eases it.";

/** Whether macOS reports memory pressure now. */
export function underPressure(machine: MachineHealthInfo): boolean {
  return machine.pressure === "warning" || machine.pressure === "critical";
}

/** The Mac's memory in parts: in use (the AI tools, what macOS keeps
 * compressed, everything else) and what's left. The AI tools' figure is
 * a sum over helpers, so it's capped at what's in use; compressed memory
 * belongs to every app, so it's its own part. `used` is `null` when it
 * wasn't read. */
export function memoryParts(info: ThisMacInfo): {
  total: number;
  used: number | null;
  tools: number;
  compressed: number;
  other: number;
  available: number;
} {
  const machine = info.machine;
  const used = machine.memory_used;
  const tools = used === null ? info.tools.rss_bytes : Math.min(info.tools.rss_bytes, used);
  const compressed = used === null ? 0 : Math.min(machine.memory_compressed ?? 0, used - tools);
  return {
    total: machine.memory_total,
    used,
    tools,
    compressed,
    other: used === null ? 0 : used - tools - compressed,
    available: used === null ? machine.memory_available : Math.max(0, machine.memory_total - used),
  };
}

/** An other app's memory: "6.4 GB", or "about 4.0 GB" when the figure
 * sums several processes (memory they share is counted more than once). */
export function otherAppBytes(app: OtherAppInfo): string {
  return `${app.processes > 1 ? "about " : ""}${bytesWords(app.rss_bytes)}`;
}

/** "next-server 6.4 GB", "Docker about 4.0 GB". */
export function otherAppWords(app: OtherAppInfo): string {
  return `${app.name} ${otherAppBytes(app)}`;
}

/** Who an alert is about, and what of theirs is high ("feat/read-model is
 * using 3.2 GB", "The Claude app is using CPU 140%"): the engine sends only
 * the figures that are high, so a CPU-heavy session isn't told by its
 * memory. While other apps hold most of the memory, it names them and no
 * AI tool (the 0.1.4 review). */
export function alertSubject(alert: OverloadAlertInfo, nameOf: (sessionId: string) => string | null): string | null {
  if (alert.other_apps.length > 0) return `Other apps are using most of the memory: ${alert.other_apps.map(otherAppWords).join(", ")}`;
  const who = alert.session_id
    ? (nameOf(alert.session_id) ?? "a session")
    : alert.place
      ? alert.place === "desktop"
        ? "The Claude app"
        : alert.place === "terminal"
          ? "claude in a terminal"
          : (alert.subject_name ?? "Another app")
      : null;
  if (!who) return null;
  const figures = [
    alert.subject_cpu !== null ? `CPU ${Math.round(alert.subject_cpu)}%` : null,
    alert.subject_bytes !== null
      ? // A tool AI Juggler didn't start: its helpers share memory the
        // sum counts more than once (AJ-E1402).
        `${alert.subject_estimate ? "about " : ""}${bytesWords(alert.subject_bytes)}`
      : null,
  ].filter((figure): figure is string => figure !== null);
  if (figures.length === 0) return null;
  const what = figures.join(" and ");
  // A tool AI Juggler didn't start is named as a whole: its windows'
  // figure, and how many it sums (the 0.0.12 report).
  // Another app's group sums its sessions (AJ-E2407).
  const unit = alert.subject_name ? "sessions" : "windows";
  const windows = (alert.subject_windows ?? 0) > 1 ? ` across ${alert.subject_windows} ${unit}` : "";
  // A session's name keeps its own case ("feat/read-model").
  return `${who} is using ${what}${windows}`;
}

/** The one source of a machine alert's words — the banner on other
 * screens and Home's Needs you say exactly this (the 0.0.12 report). */
export function alertWords(
  alerts: OverloadAlertInfo[],
  nameOf: (sessionId: string) => string | null,
): { title: string; detail: string } | null {
  const first = alerts[0];
  if (!first) return null;
  return {
    title: first.title,
    detail: [reasonSentence(alerts.map((alert) => alert.detail)), alertSubject(first, nameOf)].filter(Boolean).join(". "),
  };
}
