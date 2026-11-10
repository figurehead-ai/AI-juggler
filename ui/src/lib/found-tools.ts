/**
 * D-97 (1): what first run's scan found, kept for Home's "Next steps" —
 * each tool it found, and whether it found a setup of it or only the
 * tool (`not_set_up`, AJ-E3306). Home lists the found ones that have no
 * App in AI Juggler yet (a tool left unticked no longer just disappears),
 * with one click to its add or sign-in flow. Kept in this computer's
 * local storage (the local-cache ruling): never a second scan.
 */
import type { ScanReportData } from "./ipc/providers";

const KEY = "aij.scan.found-tools";

/** A tool the scan found: with a setup of its own, or only installed. */
export interface FoundTool {
  tool: string;
  setUp: boolean;
}

/** The scan's tools, by id, each once, in the report's order. */
export function foundToolsOf(report: Pick<ScanReportData, "installs" | "not_set_up" | "cancelled">): FoundTool[] {
  const out: FoundTool[] = [];
  for (const install of report.installs) {
    if (!out.some((found) => found.tool === install.harness)) out.push({ tool: install.harness, setUp: true });
  }
  for (const tool of report.not_set_up ?? []) {
    if (!out.some((found) => found.tool === tool)) out.push({ tool, setUp: false });
  }
  return out;
}

/** Keeps what a finished scan found (a cancelled one says too little). */
export function rememberFound(report: ScanReportData): void {
  if (report.cancelled) return;
  try {
    localStorage.setItem(KEY, JSON.stringify(foundToolsOf(report)));
  } catch {
    // No storage: Home just has nothing of the scan's to list.
  }
}

/** A kept row, as it was written. */
function isFound(row: unknown): row is FoundTool {
  return typeof row === "object" && row !== null && "tool" in row && typeof row.tool === "string" && "setUp" in row && typeof row.setUp === "boolean";
}

/** What the last finished scan found; empty when none is kept. */
export function rememberedFound(): FoundTool[] {
  try {
    const raw = localStorage.getItem(KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter(isFound).map((row) => ({ tool: row.tool, setUp: row.setUp })) : [];
  } catch {
    return [];
  }
}

/** The found tools with no App in AI Juggler, among the tools first run
 * brings in. */
export function notBroughtIn(found: FoundTool[], appTools: string[], firstRunTools: string[]): FoundTool[] {
  return found.filter((row) => firstRunTools.includes(row.tool) && !appTools.includes(row.tool));
}

/** The event the shell opens a tool's Add flow on (its found step, or a
 * fresh App and its sign-in when nothing of it is set up). */
export const ADD_APP = "aij:add-app";

/** Opens Add an app for `tool`, by its id. */
export function addAppFor(tool: string): void {
  window.dispatchEvent(new CustomEvent<{ tool: string }>(ADD_APP, { detail: { tool } }));
}
