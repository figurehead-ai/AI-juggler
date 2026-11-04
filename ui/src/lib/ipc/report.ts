/**
 * AJ-E2911 / AJ-E2912 (D-75): Report a bug and debug mode.
 *
 * - The report is gathered and scrubbed by the backend; the window shows
 *   exactly what the backend composes and asks it to open the bug form
 *   (the window never passes an address).
 * - Debug mode keeps a bounded local log of switch and launch steps.
 *
 * In the preview the seams `__AIJ_REPORT__` (the report),
 * `__AIJ_REPORT_OPENED__` (the composed report the form was opened with)
 * and `__AIJ_DEBUG__` (debug mode) answer instead.
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { BugContextArgs } from "../../types/generated/BugContextArgs";
import type { BugReportInfo } from "../../types/generated/BugReportInfo";
import type { BugComposedInfo } from "../../types/generated/BugComposedInfo";
import type { BugFieldInfo } from "../../types/generated/BugFieldInfo";
import type { BugPartInfo } from "../../types/generated/BugPartInfo";
import type { DebugModeInfo } from "../../types/generated/DebugModeInfo";

export type { BugContextArgs, BugReportInfo, BugComposedInfo, BugFieldInfo, BugPartInfo, DebugModeInfo };

const seams = () =>
  window as unknown as {
    __AIJ_REPORT__?: BugReportInfo;
    __AIJ_REPORT_OPENED__?: BugComposedInfo;
    __AIJ_DEBUG__?: DebugModeInfo;
    __AIJ_DEBUG_LOG__?: string;
  };

const SAMPLE: BugReportInfo = {
  id: "preview",
  title: "A switch failed",
  parts: [
    {
      key: "failure",
      title: "What AI Juggler showed",
      about: "the message and its detail",
      text: "Shown: The switch stopped before the new session started.\nDetail: stage: launch; claude exited with code 1\nCode: adapter",
    },
    {
      key: "app",
      title: "AI Juggler and this computer",
      about: "versions and the system",
      text: "AI Juggler 0.4.0 (AI Juggler, Beta)\nmacOS 26.1, Apple silicon\nclaude 2.1.31\ncodex 0.98.0",
    },
    {
      key: "switch",
      title: "The last switch",
      about: "its steps, sizes and timings",
      text: "1793831000000 switch start pair=claude>codex carries=everything\n1793831000420 switch stage files=3 bytes=48211 ms=420\n1793831001900 switch launch exit=1 ms=1480",
    },
    {
      key: "errors",
      title: "Recent errors",
      about: "the last lines of the error log",
      text: "1793831001900 adapter claude exited with code 1",
    },
  ],
};

/** A report's parts, scrubbed, for `context` (none from Help or Settings). */
export async function bugReportPreview(context?: BugContextArgs): Promise<BugReportInfo> {
  if (!hasLiveBackend()) return seams().__AIJ_REPORT__ ?? SAMPLE;
  return invoke<BugReportInfo>("app_bug_report_preview", { context: context ?? null });
}

/** The preview's composer: the backend's shape, enough to show. */
function composeHere(report: BugReportInfo, title: string, words: string, keep: string[]): BugComposedInfo {
  const kept = report.parts.filter((p) => keep.includes(p.key));
  const failure = kept.find((p) => p.key === "failure")?.text;
  const what = [words.trim(), failure].filter(Boolean).join("\n\n");
  const fields: BugFieldInfo[] = [{ id: "what-happened", label: "What happened?", value: what }];
  if (kept.some((p) => p.key === "app")) fields.push({ id: "version", label: "AI Juggler version", value: "0.4.0" });
  const logs = kept.filter((p) => ["switch", "errors", "debug"].includes(p.key));
  if (logs.length > 0) fields.push({ id: "logs", label: "Logs or screenshots", value: logs.map((p) => `# ${p.title}\n${p.text}`).join("\n\n") });
  const body = fields.map((f) => `### ${f.label}\n\n${f.value}\n`).join("\n");
  return { title: title.trim() || report.title, fields, body, cut: false };
}

/** What goes: the bug form's fields and the whole text for Copy. */
export async function bugReportCompose(report: BugReportInfo, title: string, words: string, keep: string[]): Promise<BugComposedInfo> {
  if (!hasLiveBackend()) return composeHere(report, title, words, keep);
  return invoke<BugComposedInfo>("app_bug_report_compose", { id: report.id, title, words, keep });
}

/** Opens the bug form in the browser, filled in with what was shown. */
export async function bugReportOpen(report: BugReportInfo, title: string, words: string, keep: string[]): Promise<void> {
  if (!hasLiveBackend()) {
    seams().__AIJ_REPORT_OPENED__ = composeHere(report, title, words, keep);
    return;
  }
  return invoke<void>("app_bug_report_open", { id: report.id, title, words, keep });
}

const OFF: DebugModeInfo = { on: false, until_ms: null, bytes: 0, lines: 0 };

/** Debug mode now. */
export async function debugModeGet(): Promise<DebugModeInfo> {
  if (!hasLiveBackend()) return seams().__AIJ_DEBUG__ ?? OFF;
  return invoke<DebugModeInfo>("app_debug_mode_get");
}

/** Turns debug mode on (for seven days) or off. */
export async function debugModeSet(on: boolean): Promise<DebugModeInfo> {
  if (!hasLiveBackend()) {
    const now = seams().__AIJ_DEBUG__ ?? OFF;
    const next = { ...now, on, until_ms: on ? Date.now() + 7 * 86_400_000 : null };
    seams().__AIJ_DEBUG__ = next;
    return next;
  }
  return invoke<DebugModeInfo>("app_debug_mode_set", { on });
}

/** Clears the debug log. */
export async function debugLogClear(): Promise<DebugModeInfo> {
  if (!hasLiveBackend()) {
    const next = { ...(seams().__AIJ_DEBUG__ ?? OFF), bytes: 0, lines: 0 };
    seams().__AIJ_DEBUG__ = next;
    return next;
  }
  return invoke<DebugModeInfo>("app_debug_log_clear");
}

/** The debug log's newest lines, scrubbed. */
export async function debugLogShow(): Promise<string> {
  if (!hasLiveBackend()) return seams().__AIJ_DEBUG_LOG__ ?? "";
  return invoke<string>("app_debug_log_show");
}

/** Help › Report a bug… in the menu bar. */
export const REPORT_BUG_ASKED = "app://report-bug";

/** Calls `on` when the menu bar's Report a bug… is chosen. */
export function onReportBugAsked(on: () => void): () => void {
  if (!hasLiveBackend()) return () => {};
  let disposed = false;
  let stop: (() => void) | null = null;
  void listen<null>(REPORT_BUG_ASKED, () => on()).then((unlisten) => {
    if (disposed) unlisten();
    else stop = unlisten;
  });
  return () => {
    disposed = true;
    stop?.();
  };
}
