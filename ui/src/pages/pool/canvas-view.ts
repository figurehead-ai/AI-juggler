/**
 * Where a pool's canvas looks: the view kept per pool while the app runs
 * (back to a pool: where you left it), fitted to its members the first
 * time and refitted as its room or its members' places change until the
 * person moves it or a member (the P9 walk: fitted before the inspector, a
 * question and 0.3's kept places came in, a member sat half under the
 * inspector).
 */
import { useCallback, useEffect, useRef, useState } from "react";

import type { View } from "./layout";

const views = new Map<string, View>();
/** Tests: every pool back to its first view. */
export const forgetCanvasViews = () => views.clear();

export function useCanvasView(
  poolId: string,
  /** The view that shows every member in the room there is now. */
  fitted: () => View,
  /** The members and the room are known. */
  ready: boolean,
  /** What changes the room: a refit when one changes. */
  room: readonly unknown[],
): { view: View | null; latest: () => View | undefined; setView: (next: View) => void; fit: () => void; forget: () => void; hold: () => void } {
  const [view, setState] = useState<View | null>(views.get(poolId) ?? null);
  const auto = useRef(false);
  const compute = useRef(fitted);
  compute.current = fitted;
  const put = useCallback(
    (next: View, fit: boolean) => {
      auto.current = fit;
      views.set(poolId, next);
      setState(next);
    },
    [poolId],
  );
  const setView = useCallback((next: View) => put(next, false), [put]);
  const fit = useCallback(() => put(compute.current(), true), [put]);
  const forget = useCallback(() => {
    views.delete(poolId);
    setState(null);
  }, [poolId]);
  // The person arranged the members: the view stays as they see it.
  const hold = useCallback(() => {
    auto.current = false;
  }, []);
  // The newest view, between renders (a burst of wheel events).
  const latest = useCallback(() => views.get(poolId), [poolId]);
  useEffect(() => {
    if (ready && (view === null || auto.current)) fit();
  }, [ready, view === null, fit, ...room]); // eslint-disable-line react-hooks/exhaustive-deps
  return { view, latest, setView, fit, forget, hold };
}
