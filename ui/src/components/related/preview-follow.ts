/**
 * The design preview (no backend): a related list's rows are the app's own
 * conversations, read again after the preview's change event. The list's
 * own read on that event (AJ-E805: at once) runs before they have changed,
 * so it reads again once they have (0.5.1 F49: sessions that started never
 * came). The app's list reads the backend, which has them by then.
 */
import { useEffect, useRef } from "react";
import { refreshPaged } from "../../lib/paged";

/** Reads `listKey` again whenever `keys` (its rows' keys) change. */
export function usePreviewFollow(on: boolean, listKey: string, keys: string): void {
  const read = useRef(keys);
  useEffect(() => {
    if (!on || keys === read.current) return;
    read.current = keys;
    refreshPaged(listKey);
  }, [on, listKey, keys]);
}
