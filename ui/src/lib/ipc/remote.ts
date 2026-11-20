/**
 * ADR-0035 (AJ-E2023) — the signed remote configuration, as the window
 * reads it: a section's values in effect (`features::remote_config`),
 * the notices to show and dismissing one (`features::notices`), and
 * "About this configuration". Every answer comes from the backend's
 * memory; it says when something changed (`remote-config://changed`).
 *
 * The preview and tests set `window.__AIJ_REMOTE__`: the flags, the
 * notices and About, each optional (the built-in values otherwise).
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import { listenWhileShown } from "../shown-window";
import type { RemoteAboutInfo } from "../../types/generated/RemoteAboutInfo";
import type { RemoteNoticeInfo } from "../../types/generated/RemoteNoticeInfo";
import type { RemoteSection } from "../../types/generated/RemoteSection";
import type { RemoteValuesInfo } from "../../types/generated/RemoteValuesInfo";

export type { RemoteAboutInfo, RemoteNoticeInfo, RemoteSection, RemoteValuesInfo };

/** A section's values, typed by its name. */
export type RemoteValuesOf<S extends RemoteSection> = Extract<RemoteValuesInfo, { section: S }>;

/** Whether `values` are section `section`'s (a type guard, no cast). */
export function isSection<S extends RemoteSection>(values: RemoteValuesInfo, section: S): values is RemoteValuesOf<S> {
  return values.section === section;
}

/** The event the backend sends when the configuration in effect changed
 * or a notice was dismissed. */
const REMOTE_CHANGED = "remote-config://changed";

type RemotePreview = {
  flags?: Record<string, boolean>;
  notices?: RemoteNoticeInfo[];
  about?: RemoteAboutInfo;
};

declare global {
  interface Window {
    __AIJ_REMOTE__?: RemotePreview;
  }
}

const preview = (): RemotePreview => (window.__AIJ_REMOTE__ ??= {});

/** The built-in flags (the backend's `flags::DECLARED` defaults). */
const BUILT_IN_FLAGS: Record<string, boolean> = {
  notices_on: true,
  ai_feed_on: false,
  ai_review_on: true,
  ai_chat_on: true,
  ai_digest_on: false,
  ai_watches_on: false,
};

/** `section`'s values in effect (narrow them with `isSection`). */
export async function remoteValues(section: RemoteSection): Promise<RemoteValuesInfo> {
  if (!hasLiveBackend()) {
    switch (section) {
      case "flags":
        return { section: "flags", flags: { ...BUILT_IN_FLAGS, ...preview().flags } };
    }
  }
  return call("remote_config_values", { section });
}

/** The notices to show now, louder first, the dismissed left out. */
export async function noticesList(): Promise<RemoteNoticeInfo[]> {
  if (!hasLiveBackend()) return [...(preview().notices ?? [])];
  return call("notices_list");
}

/** Dismisses notice `id` (remembered on this Mac); the notices left. */
export async function noticesDismiss(id: string): Promise<RemoteNoticeInfo[]> {
  if (!hasLiveBackend()) {
    const host = preview();
    const notice = host.notices?.find((row) => row.id === id);
    if (notice && !notice.dismissable) throw new Error("This notice stays until the problem it's about is fixed.");
    host.notices = (host.notices ?? []).filter((row) => row.id !== id);
    window.dispatchEvent(new Event(REMOTE_CHANGED));
    return [...host.notices];
  }
  return call("notices_dismiss", { id });
}

/** "About this configuration": each value's layer, the file in use, when
 * it was last checked, whether it is verified. */
export async function remoteAbout(): Promise<RemoteAboutInfo> {
  if (!hasLiveBackend()) {
    return (
      preview().about ?? {
        checking: true,
        keys_pinned: false,
        channel: "stable",
        verified: false,
        file: null,
        last_checked_ms: null,
        values: [],
        records: [],
      }
    );
  }
  return call("remote_config_about");
}

/** Calls `on` whenever the configuration changes: the backend's word
 * while the window can be seen (read again once when it shows), and the
 * preview's own changes. Returns the stop. */
export function onRemoteChanged(on: () => void): () => void {
  const heard = () => on();
  window.addEventListener(REMOTE_CHANGED, heard);
  const stop = hasLiveBackend() ? listenWhileShown<null>(REMOTE_CHANGED, heard, heard) : null;
  return () => {
    window.removeEventListener(REMOTE_CHANGED, heard);
    stop?.();
  };
}
