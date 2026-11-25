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
 * AJ-E1552: `state` says which of lib/loading's states the read is in
 * (a skeleton only while `loading`), `slow` turns on after 3 s, and a
 * read with no answer by its deadline (`timeoutMs`, 12 s) fails with
 * `TookTooLong`, logged: nothing stays a skeleton for ever. `isEmpty`
 * says when an answer has nothing in it (by default an empty list or
 * `null`).
 *
 * A read several screens show and keep between visits (the runways, the
 * caps) is a `sharedRead` source shown with `useSharedRead`
 * (lib/shared-read.ts); useResource is a screen's own read.
 */
import { same } from "./same";
import { everyWhileShown } from "./shown-window";
import { useCallback, useEffect, useRef, useState } from "react";
import { DEADLINE_MS, emptyAnswer, loadStateOf, logLoadFailure, useLoadDeadline, withDeadline, type LoadState } from "./loading";

export type Resource<T> = {
  /** The last answer (`undefined` before the first). */
  data: T | undefined;
  /** Why the last read failed, until the next answer. */
  error: unknown;
  /** True until an answer for this key has come (or failed). */
  loading: boolean;
  /** Reads again now. */
  reload: () => void;
  /** AJ-E1552: loading, ready, empty, error or stale. */
  state: LoadState;
  /** Still loading after 3 s: say so. */
  slow: boolean;
};

type State<T> = { key: string | null; data: T | undefined; error: unknown };

export function useResource<T>(
  key: string | null,
  read: () => Promise<T>,
  { pollMs = 0, timeoutMs = DEADLINE_MS, isEmpty = emptyAnswer }: { pollMs?: number; timeoutMs?: number; isEmpty?: (data: T) => boolean } = {},
): Resource<T> {
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
      void withDeadline(key, latest.current(), timeoutMs).then(
        // an equal answer keeps the state (0.4.1, idle CPU)
        (data) => alive && setState((was) => (was.key === key && was.error === undefined && same(was.data, data) ? was : { key, data, error: undefined })),
        (error: unknown) => {
          if (!alive) return;
          logLoadFailure(key, error);
          setState((was) => ({ key, data: was.data, error }));
        },
      );
    run();
    // a screen's poll, only while the window is shown
    const stop = pollMs > 0 ? everyWhileShown(run, pollMs) : undefined;
    return () => {
      alive = false;
      stop?.();
    };
  }, [key, round, pollMs, timeoutMs]);
  const reload = useCallback(() => setRound((was) => was + 1), []);
  const loading = key !== null && state.key !== key;
  const { slow } = useLoadDeadline(loading && state.data === undefined, key ?? "", { timeoutMs: Number.POSITIVE_INFINITY, round });
  const answer = state.data;
  const answered = answer !== undefined;
  return {
    data: state.data,
    error: state.error,
    loading,
    reload,
    state: key === null ? "empty" : loadStateOf({ hasData: answered, loading, error: loading ? undefined : state.error, empty: answer !== undefined && isEmpty(answer) }),
    slow,
  };
}
