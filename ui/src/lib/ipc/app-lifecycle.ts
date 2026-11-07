/**
 * AI Juggler's own life: its version, ready, relaunch and quitting with
 * live sessions (AJ-E1503).
 */
import { call, listen, liveBackend as hasBackend } from "./transport";
import { EVENTS } from "../bridge";
import { osAccessSeams } from "./settings";
import { listenScoped } from "./terminal-io";
import type { AppInfo } from "../../types/generated/AppInfo";
import type { QuitPlanInfo } from "../../types/generated/QuitPlanInfo";
import type { ReopenBlockInfo } from "../../types/generated/ReopenBlockInfo";


export async function fetchAppInfo(): Promise<AppInfo | null> {
  if (!hasBackend) {
    // Gallery seam: a Nightly or Dev window (D-23).
    const seeded = (window as unknown as { __AIJ_APP_INFO__?: AppInfo }).__AIJ_APP_INFO__;
    return seeded ?? { version: "0.0.0-preview", name: "AI Juggler", edition: "main", reserved_keys: [] };
  }
  try {
    return await call("app_info");
  } catch {
    return null;
  }
}

/** Reopens AI Juggler (`app_relaunch`; macOS shows a new Screen
 * Recording answer only then). Resolves with why it has to be confirmed
 * or wait; when nothing stands in the way (or `anyway` after live
 * sessions) the app relaunches and this never resolves. */
export async function appRelaunch(anyway: boolean): Promise<ReopenBlockInfo | null> {
  if (!hasBackend) {
    const seams = osAccessSeams();
    seams.__AIJ_OS_ACCESS_CALLS__ = [...(seams.__AIJ_OS_ACCESS_CALLS__ ?? []), `reopen:${anyway ? "anyway" : "ask"}`];
    const block = seams.__AIJ_REOPEN_BLOCK__ ?? null;
    return anyway && block?.can_reopen_anyway ? null : block;
  }
  return call("app_relaunch", { anyway });
}

/** Tells the backend the window has painted its first frame (two
 * animation frames on: the first one's paint is done), so the start's
 * heavy work (a first conversation list read) begins only then. A window
 * started hidden never paints: the backend starts within 5 s anyway. */
export function sayPainted(): void {
  if (!hasBackend || typeof requestAnimationFrame !== "function") return;
  requestAnimationFrame(() => requestAnimationFrame(() => void call("app_painted").catch(() => undefined)));
}

export function onAppReady(handler: (info: AppInfo) => void): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<AppInfo>(EVENTS.appReady, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {});
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** What quitting now would do to the live sessions. */
export async function appQuitPlan(): Promise<QuitPlanInfo> {
  if (!hasBackend) {
    const seam = (window as unknown as { __AIJ_QUIT_PLAN__?: QuitPlanInfo }).__AIJ_QUIT_PLAN__;
    return seam ?? { live: 0, end_on_quit: 0, keep_running: 0, come_back: 0 };
  }
  return call("app_quit_plan");
}

/** Quits now; with `stopFirst` each live session is stopped cleanly first. */
export async function appQuit(stopFirst: boolean): Promise<void> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_QUIT_CALLS__?: string[] };
    w.__AIJ_QUIT_CALLS__ = [...(w.__AIJ_QUIT_CALLS__ ?? []), stopFirst ? "stop-first" : "quit"];
    return;
  }
  return call("app_quit", { stopFirst });
}

/** The quit sheet is shown: the request is in a person's hands (without
 * this within 3 s, the app quits as if Quit anyway was chosen). */
export async function appQuitAck(): Promise<void> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_QUIT_CALLS__?: string[] };
    w.__AIJ_QUIT_CALLS__ = [...(w.__AIJ_QUIT_CALLS__ ?? []), "ack"];
    return;
  }
  return call("app_quit_ack");
}

/** Cancel on the quit sheet: the next ⌘Q asks again. */
export async function appQuitCancel(): Promise<void> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_QUIT_CALLS__?: string[] };
    w.__AIJ_QUIT_CALLS__ = [...(w.__AIJ_QUIT_CALLS__ ?? []), "cancel"];
    return;
  }
  return call("app_quit_cancel");
}

/** ⌘Q or the close button while sessions are live: show the quit sheet. */
export function onQuitRequested(handler: (plan: QuitPlanInfo) => void): () => void {
  if (!hasBackend) {
    // The preview's seam: `aij:quit-requested` with the plan as detail.
    const follow = (event: Event) => handler((event as CustomEvent<QuitPlanInfo>).detail);
    window.addEventListener("aij:quit-requested", follow);
    return () => window.removeEventListener("aij:quit-requested", follow);
  }
  return listenScoped("app://quit-requested", handler);
}
