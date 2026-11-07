/**
 * AJ-E2018 — the one way a screen reads backend state.
 *
 *     const { data, error, reload } = useResource(`switch-space:${id}`, () => switchSpace(id));
 *
 * `key` names the read: it is read when the screen mounts and again when
 * the key changes (`null` reads nothing). Until the first answer `data`
 * is `undefined`; a new key keeps the last answer on screen until its own
 * comes (no flash of empty). A failed read keeps the last answer and sets
 * `error` (cleared by the next answer), so the screen decides what a
 * failure shows. An answer that arrives after the key changed, or after
 * the screen left, is dropped. `reload()` reads again (after a change the
 * user made); `pollMs` reads again on a timer while the screen is shown.
 *
 * A read several screens show and keep between visits (the runways, the
 * caps) is a `sharedRead` source shown with `useSharedRead`
 * (lib/shared-read.ts); useResource is a screen's own read.
 */
import { same } from "./same";
import { everyWhileShown } from "./shown-window";
import { useCallback, useEffect, useRef, useState } from "react";

export type Resource<T> = {
  /** The last answer (`undefined` before the first). */
  data: T | undefined;
  /** Why the last read failed, until the next answer. */
  error: unknown;
  /** True until an answer for this key has come (or failed). */
  loading: boolean;
  /** Reads again now. */
  reload: () => void;
};

type State<T> = { key: string | null; data: T | undefined; error: unknown };

export function useResource<T>(key: string | null, read: () => Promise<T>, { pollMs = 0 }: { pollMs?: number } = {}): Resource<T> {
  const [state, setState] = useState<State<T>>({ key: null, data: undefined, error: undefined });
  const [round, setRound] = useState(0);
  // The newest `read` (its arguments live in its closure; `key` says when
  // they changed).
  const latest = useRef(read);
  useEffect(() => {
    latest.current = read;
  });
  useEffect(() => {
    if (key === null) return undefined;
    let alive = true;
    const run = () =>
      void latest.current().then(
        // an equal answer keeps the state (0.4.1, idle CPU)
        (data) => alive && setState((was) => (was.key === key && was.error === undefined && same(was.data, data) ? was : { key, data, error: undefined })),
        (error: unknown) => alive && setState((was) => ({ key, data: was.data, error })),
      );
    run();
    // a screen's poll, only while the window is shown
    const stop = pollMs > 0 ? everyWhileShown(run, pollMs) : undefined;
    return () => {
      alive = false;
      stop?.();
    };
  }, [key, round, pollMs]);
  const reload = useCallback(() => setRound((was) => was + 1), []);
  return { data: state.data, error: state.error, loading: key !== null && state.key !== key, reload };
}
