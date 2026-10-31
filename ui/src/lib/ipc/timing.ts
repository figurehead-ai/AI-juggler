/**
 * 0.3.2 — how long each backend call takes, kept in this window only (the
 * maintainer's 0.3.0 walk: a banner's close took about a minute, and
 * nothing said which call was slow). Every invoke is timed by the
 * transport; per command the count, the slowest and the latest are kept,
 * and calls slower than SLOW_MS go into a short list. Command names and
 * times only: never the arguments or the answer. Settings › Advanced ›
 * Report a problem shows them.
 */

/** A call slower than this is listed on its own. */
export const SLOW_MS = 300;
const KEEP_SLOW = 30;

export interface CommandTiming {
  command: string;
  calls: number;
  slowestMs: number;
  lastMs: number;
  failed: number;
}

export interface SlowCall {
  command: string;
  ms: number;
  at: number;
}

const byCommand = new Map<string, CommandTiming>();
let slow: SlowCall[] = [];

/** Records one finished call. */
export function recordCall(command: string, ms: number, ok: boolean): void {
  const row = byCommand.get(command) ?? { command, calls: 0, slowestMs: 0, lastMs: 0, failed: 0 };
  row.calls += 1;
  row.lastMs = ms;
  row.slowestMs = Math.max(row.slowestMs, ms);
  if (!ok) row.failed += 1;
  byCommand.set(command, row);
  if (ms >= SLOW_MS) slow = [{ command, ms, at: Date.now() }, ...slow].slice(0, KEEP_SLOW);
}

/** Times a call; the answer and any error pass through unchanged. */
export function timed<T>(command: string, call: () => Promise<T>): Promise<T> {
  const start = performance.now();
  return call().then(
    (value) => (recordCall(command, performance.now() - start, true), value),
    (cause: unknown) => {
      recordCall(command, performance.now() - start, false);
      throw cause;
    },
  );
}

/** The commands, slowest first. */
export function commandTimings(): CommandTiming[] {
  return [...byCommand.values()].sort((a, b) => b.slowestMs - a.slowestMs);
}

/** The slow calls, newest first. */
export function slowCalls(): SlowCall[] {
  return slow;
}

/** Test seam. */
export function resetTimings(): void {
  byCommand.clear();
  slow = [];
}
