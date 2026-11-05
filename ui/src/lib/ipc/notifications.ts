/**
 * AJ-E1510 — system notifications over IPC (ENGINEERING-GUIDE U3: one thin
 * file per feature): which kinds may interrupt, and what a clicked one
 * asks the app to open. In the preview, the seam `__AIJ_NOTIFY__` holds
 * the kinds, and an `aij:notification-click` window event stands in for
 * a click.
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { SystemNotificationsInfo } from "../../types/generated/SystemNotificationsInfo";

export type { SystemNotificationsInfo };

/** Every kind starts off (Req.1). */
export const NOTIFY_DEFAULT: SystemNotificationsInfo = {
  waiting: false,
  stuck: false,
  limit: false,
  finished: false,
  this_mac: false,
  left_behind: false,
  quota: false,
  crash: false,
  reset: false,
};

/** What a clicked notification opens. */
export interface OpenRequest {
  session_id: string | null;
  /** `this_mac`, or (AJ-E2501) `notifications`: the centre; from the
   * Floor (AJ-E3510) also `app`, `conversation` (another app's session),
   * `subagents` and `usage`, each with its `id`. */
  page: "this_mac" | "notifications" | "app" | "conversation" | "subagents" | "usage" | "pool" | null;
  /** AJ-E1515: the paired Mac the session runs on (absent: this one). */
  machine_id?: string | null;
  /** What `page` opens: an App's id, `<account>/<id>` of another app's
   * session, the rail's key of a session (its subagents), an Account. */
  id?: string | null;
}

/** The preview's stand-in for a click. */
export const NOTIFICATION_CLICK_EVENT = "aij:notification-click";

const preview = () => window as unknown as { __AIJ_NOTIFY__?: SystemNotificationsInfo };

/** Which kinds may become a system notification. */
export async function notificationsGet(): Promise<SystemNotificationsInfo> {
  if (!hasLiveBackend()) return { ...NOTIFY_DEFAULT, ...preview().__AIJ_NOTIFY__ };
  return invoke<SystemNotificationsInfo>("notifications_get");
}

/** Saves which kinds may become a system notification. */
export async function notificationsSet(kinds: SystemNotificationsInfo): Promise<SystemNotificationsInfo> {
  if (!hasLiveBackend()) {
    preview().__AIJ_NOTIFY__ = { ...kinds };
    return { ...kinds };
  }
  return invoke<SystemNotificationsInfo>("notifications_set", { kinds });
}

/** AJ-E2904: Send test — one system notification now; what to check if
 * none appeared (the preview's seam `__AIJ_NOTIFY_TESTS__` counts them). */
export async function notificationsSendTest(): Promise<string> {
  if (!hasLiveBackend()) {
    const seams = window as unknown as { __AIJ_NOTIFY_TESTS__?: number };
    seams.__AIJ_NOTIFY_TESTS__ = (seams.__AIJ_NOTIFY_TESTS__ ?? 0) + 1;
    return "Sent. If nothing appeared, allow AI Juggler in System Settings › Notifications.";
  }
  return invoke<string>("notifications_send_test");
}

/** Called when a notification was clicked: open what it was about. */
export function onNotificationClick(handler: (open: OpenRequest) => void): () => void {
  if (!hasLiveBackend()) {
    const local = (event: Event) => handler((event as CustomEvent<OpenRequest>).detail);
    window.addEventListener(NOTIFICATION_CLICK_EVENT, local);
    return () => window.removeEventListener(NOTIFICATION_CLICK_EVENT, local);
  }
  let stop: (() => void) | null = null;
  let disposed = false;
  void listen<OpenRequest>("app://open", (event) => handler(event.payload)).then((unlisten) => {
    if (disposed) unlisten();
    else stop = unlisten;
  });
  return () => {
    disposed = true;
    stop?.();
  };
}
