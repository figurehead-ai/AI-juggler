/**
 * AJ-E1010: updates and going back.
 */
import { invoke, listen, liveBackend as hasBackend } from "./transport";
import { EVENTS } from "../bridge";
import type { GoingBackInfo } from "../../types/generated/GoingBackInfo";
import type { UpdateStatusInfo } from "../../types/generated/UpdateStatusInfo";
import { previewHooks } from "../preview-hooks";


interface UpdateSeams {
  __AIJ_UPDATE_STATUS__?: UpdateStatusInfo;
  __AIJ_UPDATE_CALLS__?: string[];
  __AIJ_UPDATE_AFTER_CHECK__?: UpdateStatusInfo;
  __AIJ_UPDATE_AFTER_INSTALL__?: UpdateStatusInfo;
  __AIJ_UPDATE_RESTART_ERROR__?: string;
}
export const updateSeams = () => window as unknown as UpdateSeams;
const updateCall = (call: string) => {
  const seams = updateSeams();
  seams.__AIJ_UPDATE_CALLS__ = [...(seams.__AIJ_UPDATE_CALLS__ ?? []), call];
};
/** Where updating stands. */
export async function updateStatus(): Promise<UpdateStatusInfo> {
  if (!hasBackend) return { ...previewHooks().appState.mockUpdateStatus() };
  return invoke<UpdateStatusInfo>("update_status");
}

/** Turns "check when AI Juggler starts" on or off. */
export async function updateSetCheckOnLaunch(enabled: boolean): Promise<UpdateStatusInfo> {
  if (!hasBackend) {
    updateCall(`check-on-launch:${enabled}`);
    const next = { ...previewHooks().appState.mockUpdateStatus(), check_on_launch: enabled };
    updateSeams().__AIJ_UPDATE_STATUS__ = next;
    return next;
  }
  return invoke<UpdateStatusInfo>("update_set_check_on_launch", { enabled });
}

/** D-20: the update channel this install follows. AI Juggler offers
 * stable and beta; AI Juggler Nightly follows nightly (D-23). */
export type UpdateChannel = "stable" | "beta" | "nightly";

/** Chooses the update channel; forgets what the previous feed announced. */
export async function updateSetChannel(channel: UpdateChannel): Promise<UpdateStatusInfo> {
  if (!hasBackend) {
    updateCall(`channel:${channel}`);
    const current = previewHooks().appState.mockUpdateStatus();
    const next: UpdateStatusInfo = {
      ...current,
      channel,
      ...(current.state === "installed" ? {} : { state: "idle", available: null, error: null }),
    };
    updateSeams().__AIJ_UPDATE_STATUS__ = next;
    return next;
  }
  return invoke<UpdateStatusInfo>("update_set_channel", { channel });
}

/** Checks for a newer version now (never installs). */
export async function updateCheck(): Promise<UpdateStatusInfo> {
  if (!hasBackend) {
    updateCall("check");
    const next = updateSeams().__AIJ_UPDATE_AFTER_CHECK__ ?? previewHooks().appState.mockUpdateStatus();
    updateSeams().__AIJ_UPDATE_STATUS__ = next;
    return { ...next };
  }
  return invoke<UpdateStatusInfo>("update_check");
}

/** Installs the announced update: backup, download, verify, install. */
export async function updateInstall(): Promise<UpdateStatusInfo> {
  if (!hasBackend) {
    updateCall("install");
    const next = updateSeams().__AIJ_UPDATE_AFTER_INSTALL__ ?? previewHooks().appState.mockUpdateStatus();
    updateSeams().__AIJ_UPDATE_STATUS__ = next;
    return { ...next };
  }
  return invoke<UpdateStatusInfo>("update_install");
}

/** Restarts into the installed version (refused while it isn't safe: a
 * session running, a switch in flight, a bring-in under way). */
export async function updateRestart(): Promise<void> {
  if (!hasBackend) {
    updateCall("restart");
    const error = updateSeams().__AIJ_UPDATE_RESTART_ERROR__;
    if (error) throw new Error(error);
    return;
  }
  return invoke<void>("update_restart");
}

/** AJ-E1010: the earlier versions this install can go back to (the
 * copies kept before upgrading the data). */
export async function goingBackOptions(): Promise<GoingBackInfo[]> {
  if (!hasBackend) {
    return [...((window as unknown as { __AIJ_GOING_BACK__?: GoingBackInfo[] }).__AIJ_GOING_BACK__ ?? [])];
  }
  return invoke<GoingBackInfo[]>("going_back_options");
}

/** AJ-E1010: puts the data back as the version reading `schema` left it,
 * keeps the data as it is now beside it, and quits. Refused while a
 * switch, a bring-in or a session would be cut short. */
export async function goingBackPrepare(schema: number): Promise<void> {
  if (!hasBackend) {
    updateCall(`going-back:${schema}`);
    const error = (window as unknown as { __AIJ_GOING_BACK_ERROR__?: string }).__AIJ_GOING_BACK_ERROR__;
    if (error) throw new Error(error);
    return;
  }
  return invoke<void>("going_back_prepare", { schema });
}

/** 0.0.11 walk (Q6): opens the page to install `version` from (its
 * release, or the releases list). */
export async function goingBackOpenReleases(version: string | null): Promise<void> {
  if (!hasBackend) {
    updateCall(`open-releases:${version ?? ""}`);
    return;
  }
  return invoke<void>("going_back_open_releases", { version });
}

/** Update status changes pushed by the backend. */
export function onUpdateStatus(handler: (status: UpdateStatusInfo) => void): () => void {
  if (!hasBackend) return () => {};
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<UpdateStatusInfo>(EVENTS.updateStatus, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {});
  return () => {
    disposed = true;
    unlisten?.();
  };
}
