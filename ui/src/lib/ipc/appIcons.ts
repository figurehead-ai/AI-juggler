/**
 * D-38 — an app's icon, as installed on this Mac (ENGINEERING-GUIDE U3:
 * one IPC module per feature). The backend reads it from the app's own
 * bundle at run time, for an app it already knows (the host registry's
 * id, or the process of a place), and serves it as a PNG `data:` URL.
 * Nothing is shipped in the repository: in the preview and the tests,
 * icons come only from the seam `__AIJ_APP_ICONS__` (by app id), which
 * the specs fill with made-up test images, else from the preview's own
 * made-up set (`preview/app-icons`).
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import { PREVIEW_APP_ICONS } from "../../preview/app-icons";

/** The pixel sizes the backend renders. */
export type AppIconPixels = 32 | 64 | 128;

/** Which app: the host registry's id (`PlaceInfo.app_id`, a tool's
 * `host_app`, or AI Juggler's own place), and the app's process when a
 * place knows it (an app the registry doesn't list). */
export interface AppIconAsk {
  appId: string | null;
  appPid: number | null;
}

/** The app's icon as a `data:` URL, or `null` when it can't be read (not
 * installed, no icon, or nothing to ask for). */
export async function appIconRead(ask: AppIconAsk, px: AppIconPixels): Promise<string | null> {
  if (!hasLiveBackend()) {
    if (!ask.appId) return null;
    const seams = window as unknown as { __AIJ_APP_ICONS__?: Record<string, string> };
    if (seams.__AIJ_APP_ICONS__) return seams.__AIJ_APP_ICONS__[ask.appId] ?? null;
    // The dev preview and the gallery: the made-up test icons, so every
    // capture shows the intended look (unit tests keep the letters).
    if (import.meta.env.MODE === "test") return null;
    return PREVIEW_APP_ICONS[ask.appId] ?? null;
  }
  if (ask.appId === null && ask.appPid === null) return null;
  return invoke<string | null>("app_icons_read", { appId: ask.appId, process: ask.appPid, px });
}
