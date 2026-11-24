/**
 * AJ-E1552 — nothing loads forever. One loading model for every read a
 * screen waits on:
 *
 *   loading  nothing to show yet: the only state that draws a skeleton;
 *   ready    an answer with something in it;
 *   empty    an answer with nothing in it (its own words and next step,
 *            never a skeleton);
 *   error    no answer, and the read failed or ran past its deadline
 *            (plain words and Retry);
 *   stale    an earlier answer shown while a new one is read, or after a
 *            re-read failed.
 *
 * A read that is still loading says "Still loading…" after SLOW_MS, and
 * after its deadline (DEADLINE_MS unless it says otherwise) it is an
 * error, its cause logged here and in the console. `useResource` applies
 * all of it to a screen's own read; `useLoadDeadline` gives the same
 * clock to a loading state a component derives itself.
 */
import { useCallback, useEffect, useState } from "react";

export type LoadState = "loading" | "ready" | "empty" | "error" | "stale";

/** "Still loading…" after this long. */
export const SLOW_MS = 3_000;
/** A read with no answer after this long has failed. */
export const DEADLINE_MS = 12_000;

/** A read that ran past its deadline. */
export class TookTooLong extends Error {
  constructor(what: string, ms: number) {
    super(`${what} took longer than ${Math.round(ms / 1000)} s`);
    this.name = "TookTooLong";
  }
}

/** The state of a read: whether an answer is shown (`hasData`), whether a
 * read is under way, its last failure, and whether the answer is empty. */
export function loadStateOf({ hasData, loading, error, empty }: { hasData: boolean; loading: boolean; error: unknown; empty: boolean }): LoadState {
  if (hasData) return loading || error !== undefined ? "stale" : empty ? "empty" : "ready";
  if (error !== undefined) return "error";
  return loading ? "loading" : "empty";
}

/** An answer with nothing in it: an empty list, or nothing at all. */
export function emptyAnswer(data: unknown): boolean {
  return data === null || data === undefined || (Array.isArray(data) && data.length === 0);
}

/** A read past its deadline, kept in this window for Report a problem. */
export interface MissedDeadline {
  what: string;
  cause: string;
  at: number;
}

let missed: MissedDeadline[] = [];

/** Notes that `what` failed to load (a deadline or an error), with why. */
export function logLoadFailure(what: string, cause: unknown): void {
  const words = cause instanceof Error ? `${cause.name}: ${cause.message}` : String(cause);
  missed = [{ what, cause: words, at: Date.now() }, ...missed].slice(0, 30);
  console.warn(`AI Juggler couldn't load ${what}: ${words}`);
}

/** The reads that failed to load lately, newest first. */
export function missedDeadlines(): readonly MissedDeadline[] {
  return missed;
}

/** `work`, or a `TookTooLong` once `ms` pass without its answer. */
export function withDeadline<T>(what: string, work: Promise<T>, ms = DEADLINE_MS): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<never>((_, fail) => {
    timer = setTimeout(() => fail(new TookTooLong(what, ms)), ms);
  });
  return Promise.race([work, late]).finally(() => clearTimeout(timer));
}

/** The clock of one loading state: `slow` after SLOW_MS, `timedOut`
 * after `timeoutMs` (logged once), both reset when loading ends or
 * `round` changes (a Retry). */
export function useLoadDeadline(loading: boolean, what: string, { timeoutMs = DEADLINE_MS, round = 0 }: { timeoutMs?: number; round?: number } = {}): { slow: boolean; timedOut: boolean } {
  // The clock of this wait: a new `what`, deadline or Retry is a new one.
  const clock = `${round}|${timeoutMs}|${what}`;
  const [phase, setPhase] = useState<{ clock: string; slow: boolean; timedOut: boolean }>({ clock: "", slow: false, timedOut: false });
  useEffect(() => {
    if (!loading) return undefined;
    const slow = setTimeout(() => setPhase({ clock, slow: true, timedOut: false }), SLOW_MS);
    // No deadline of its own (`Infinity`): the read has one already.
    const out = Number.isFinite(timeoutMs)
      ? setTimeout(() => {
          logLoadFailure(what, new TookTooLong(what, timeoutMs));
          setPhase({ clock, slow: true, timedOut: true });
        }, timeoutMs)
      : undefined;
    return () => {
      clearTimeout(slow);
      clearTimeout(out);
      // the wait ended: the next one starts its clock afresh
      setPhase({ clock: "", slow: false, timedOut: false });
    };
  }, [loading, what, timeoutMs, clock]);
  const current = loading && phase.clock === clock;
  return { slow: current && phase.slow, timedOut: current && phase.timedOut };
}

/** AJ-E1552: the state of a read a component keeps itself (`null` or
 * `undefined` until its answer, `error` once it failed): loading until
 * then, with "Still loading…" after 3 s and an error at the deadline;
 * then ready, or empty by `isEmpty`. `round` restarts the clock (a
 * Retry). */
export function useReadState<T>(
  data: T | null | undefined,
  what: string,
  { error, isEmpty = emptyAnswer, round = 0, timeoutMs = DEADLINE_MS }: { error?: unknown; isEmpty?: (data: T) => boolean; round?: number; timeoutMs?: number } = {},
): { state: LoadState; slow: boolean } {
  const waiting = data === null || data === undefined;
  const failed = error !== undefined && error !== null;
  const { slow, timedOut } = useLoadDeadline(waiting && !failed, what, { round, timeoutMs });
  if (!waiting) return { state: isEmpty(data) ? "empty" : "ready", slow: false };
  return { state: failed || timedOut ? "error" : "loading", slow };
}

/** A screen's own read failure and its Retry (AJ-E1552): `failed` is the
 * last read's cause (cleared with `setFailed(undefined)` on an answer),
 * `round` a count a read's effect depends on, so `retry` clears the
 * failure and reads again. */
export function useRetry(): { failed: unknown; setFailed: (cause: unknown) => void; round: number; retry: () => void } {
  const [failed, setFailed] = useState<unknown>(undefined);
  const [round, setRound] = useState(0);
  const retry = useCallback(() => {
    setFailed(undefined);
    setRound((was) => was + 1);
  }, []);
  return { failed, setFailed, round, retry };
}

/** A screen's own read of `data` with its Retry: `useRetry` and
 * `useReadState` together, for a read that knows its emptiness up front. */
export function useRetriedRead<T>(
  data: T | null | undefined,
  what: string,
  { isEmpty }: { isEmpty?: (data: T) => boolean } = {},
): { state: LoadState; slow: boolean; setFailed: (cause: unknown) => void; round: number; retry: () => void } {
  const { failed, setFailed, round, retry } = useRetry();
  const { state, slow } = useReadState(data, what, { error: failed, round, isEmpty });
  return { state, slow, setFailed, round, retry };
}
