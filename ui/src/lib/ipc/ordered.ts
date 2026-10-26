/**
 * Sends that must arrive in order (a terminal's keystrokes, its size).
 * Every command runs off the app's main thread now, so two calls in
 * flight at once may be handled in either order: one call per key is in
 * flight at a time, and what comes meanwhile waits for it, joined into
 * the next call.
 */

type Lane<T> = { flight: Promise<void> | null; waiting: T | null };

/** Keystrokes, by terminal: what's typed while one write is in flight
 * goes, joined, in the next. */
export function orderedJoin(send: (key: string, data: string) => Promise<void>): (key: string, data: string) => Promise<void> {
  return ordered<string>(send, (waiting, next) => (waiting ?? "") + next);
}

/** The latest value wins (a terminal's size): one in flight, and only
 * the newest of what came meanwhile follows it. */
export function orderedLatest<T>(send: (key: string, value: T) => Promise<void>): (key: string, value: T) => Promise<void> {
  return ordered<T>(send, (_, next) => next);
}

function ordered<T>(send: (key: string, value: T) => Promise<void>, merge: (waiting: T | null, next: T) => T): (key: string, value: T) => Promise<void> {
  const lanes = new Map<string, Lane<T>>();
  const drain = (key: string, lane: Lane<T>): Promise<void> => {
    const value = lane.waiting as T;
    lane.waiting = null;
    const flight = send(key, value)
      .catch(() => undefined)
      .then(() => {
        if (lane.waiting !== null) return drain(key, lane);
        lane.flight = null;
        lanes.delete(key);
      });
    lane.flight = flight;
    return flight;
  };
  return (key, value) => {
    const lane = lanes.get(key) ?? { flight: null, waiting: null };
    lanes.set(key, lane);
    lane.waiting = merge(lane.waiting, value);
    return lane.flight ?? drain(key, lane);
  };
}
