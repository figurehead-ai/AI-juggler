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
  /** `this_mac`, or (AJ-E2501) `notifications`: the centre. */
  page: "this_mac" | "notifications" | null;
  /** AJ-E1515: the paired Mac the session runs on (absent: this one). */
  machine_id?: string | null;
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
