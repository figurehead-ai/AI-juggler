/**
 * ADR-0035 §2 (AJ-E2023) — `useNotices()`: the remote configuration's
 * notices to show now (louder first, the dismissed left out), and
 * `dismiss(id)`, remembered on this Mac. Not the notification centre's
 * notices (`notices-store.ts`): these come from the signed file.
 *
 * Lazy, cached and kept current like `useRemote` (no timer): read when a
 * view first asks, again on `remote-config://changed`. `null` until the
 * first answer. The screens that show them come with the D-126 Alert.
 */
import { useCallback, useEffect } from "react";
import { sharedRead, useSharedRead, whenForgotten } from "./shared-read";
import { noticesDismiss, noticesList, onRemoteChanged, type RemoteNoticeInfo } from "./ipc/remote";



const NOTICES = sharedRead<RemoteNoticeInfo[]>(noticesList);

let stop: (() => void) | null = null;
function listen(): void {
  if (stop) return;
  stop = onRemoteChanged(() => void NOTICES.refresh());
  whenForgotten(() => {
    stop?.();
    stop = null;
  });
}

export type Notices = {
  /** The notices to show (`null` until read). */
  notices: RemoteNoticeInfo[] | null;
  /** Dismisses one; the list is read again. Rejects with the backend's
   * words for one that can't be dismissed. */
  dismiss: (id: string) => Promise<void>;
};

export function useNotices(): Notices {
  useEffect(listen, []);
  const notices = useSharedRead(NOTICES, { maxAgeMs: Number.POSITIVE_INFINITY }) ?? null;
  const dismiss = useCallback(async (id: string) => {
    await noticesDismiss(id);
    await NOTICES.refresh();
  }, []);
  return { notices, dismiss };
}
