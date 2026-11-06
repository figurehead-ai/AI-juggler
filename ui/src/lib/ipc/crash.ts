/**
 * Crash reports: UI errors, the setting and the answer.
 */
import { call, liveBackend as hasBackend } from "./transport";
import type { CrashInfo as CrashInfoData } from "../../types/generated/CrashInfo";

function previewCrash(): CrashInfoData {
  const seams = window as unknown as { __AIJ_CRASH__?: CrashInfoData };
  seams.__AIJ_CRASH__ ??= { setting: "never", effective: "never", reason: null, asking: [], queued: 0, can_send: false };
  return seams.__AIJ_CRASH__;
}

/** A screen's error, from its error boundary: kept like a crash (scrubbed,
 * only if the user's crash-report setting allows). Only the screen's id,
 * the error's message and its stack are sent. */
export async function crashUiError(screen: string, message: string, stack: string): Promise<boolean> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_UI_ERRORS__?: { screen: string; message: string }[] };
    seams.__AIJ_UI_ERRORS__ = [...(seams.__AIJ_UI_ERRORS__ ?? []), { screen, message }];
    return false;
  }
  return call("crash_ui_error", { screen, message, stack });
}

/** AJ-E1107: the crash-report setting and the reports waiting. */
export async function crashInfo(): Promise<CrashInfoData> {
  if (!hasBackend) return previewCrash();
  return call("crash_info");
}

/** AJ-E1107: `never` (deletes every kept report) | `ask` | `always`. */
export async function crashSet(setting: "never" | "ask" | "always"): Promise<CrashInfoData> {
  if (!hasBackend) {
    const current = previewCrash();
    const next: CrashInfoData = {
      ...current,
      setting,
      effective: current.reason ? "never" : setting,
      ...(setting === "never" ? { asking: [], queued: 0 } : {}),
    };
    (window as unknown as { __AIJ_CRASH__?: CrashInfoData }).__AIJ_CRASH__ = next;
    return next;
  }
  return call("crash_set", { setting });
}

/** AJ-E1107: the answer to "Send a crash report?" for one report. */
export async function crashAnswer(id: string, answer: "send" | "dont_send" | "always_send"): Promise<CrashInfoData> {
  if (!hasBackend) {
    const current = previewCrash();
    const asking = current.asking.filter((report) => report.id !== id);
    const next: CrashInfoData = {
      ...current,
      asking,
      queued: current.queued + (answer === "dont_send" ? 0 : 1),
      setting: answer === "always_send" ? "always" : current.setting,
    };
    (window as unknown as { __AIJ_CRASH__?: CrashInfoData; __AIJ_CRASH_ANSWERS__?: string[] }).__AIJ_CRASH__ = next;
    const w = window as unknown as { __AIJ_CRASH_ANSWERS__?: string[] };
    w.__AIJ_CRASH_ANSWERS__ = [...(w.__AIJ_CRASH_ANSWERS__ ?? []), `${id}:${answer}`];
    return next;
  }
  return call("crash_answer", { id, answer });
}
