/**
 * 0.4.1, idle CPU: the window's own polling follows whether it's seen.
 * A poll runs every `everyMs` while the window is shown and focused,
 * `unfocusedTimes` slower while it's shown but another app is in front,
 * and not at all while it's hidden or minimised; when it's shown again it
 * runs once at once, then on its cadence. Events from the backend still
 * arrive while hidden: this is only the window's own asking.
 */

/** The window can be seen (not hidden, minimised, or a background tab). */
function windowShown(): boolean {
  return typeof document === "undefined" || document.visibilityState !== "hidden";
}

function windowFocused(): boolean {
  return typeof document === "undefined" || typeof document.hasFocus !== "function" || document.hasFocus();
}

/** How long until the next run: `null` while hidden. */
export function nextDelay(everyMs: number, shown: boolean, focused: boolean, unfocusedTimes: number): number | null {
  if (!shown) return null;
  return focused ? everyMs : everyMs * unfocusedTimes;
}

/** Runs `run` on the window's cadence (above); returns the stop. With
 * `now`, it also runs at once. */
export function everyWhileShown(run: () => void, everyMs: number, { unfocusedTimes = 4, now = false }: { unfocusedTimes?: number; now?: boolean } = {}): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let stopped = false;
  const schedule = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    if (stopped) return;
    const delay = nextDelay(everyMs, windowShown(), windowFocused(), unfocusedTimes);
    if (delay === null) return;
    timer = setTimeout(() => {
      timer = null;
      run();
      schedule();
    }, delay);
  };
  const back = () => {
    if (stopped) return;
    if (windowShown()) run();
    schedule();
  };
  const onVisibility = () => (windowShown() ? back() : schedule());
  document.addEventListener("visibilitychange", onVisibility);
  window.addEventListener("focus", schedule);
  window.addEventListener("blur", schedule);
  if (now) run();
  schedule();
  return () => {
    stopped = true;
    if (timer !== null) clearTimeout(timer);
    document.removeEventListener("visibilitychange", onVisibility);
    window.removeEventListener("focus", schedule);
    window.removeEventListener("blur", schedule);
  };
}

/**
 * 0.4.5 (AJ-E805): a backend event's handler that runs only while the
 * window can be seen. Heard while it's hidden, it runs once when the
 * window shows again, with what those events said merged by `merge` (the
 * latest, by default). With 10 sessions at work, the hidden window read
 * its lists again about 90 times a minute (each a read of every
 * conversation), and drew them, for nobody.
 */
export function whileShown<T>(handler: (value: T) => void, merge: (was: T, now: T) => T = (_was, now) => now): { heard: (value: T) => void; stop: () => void } {
  let pending: { value: T } | null = null;
  const onVisibility = () => {
    if (pending === null || !windowShown()) return;
    const { value } = pending;
    pending = null;
    handler(value);
  };
  if (typeof document !== "undefined") document.addEventListener("visibilitychange", onVisibility);
  return {
    heard: (value) => {
      if (windowShown()) handler(value);
      else pending = pending === null ? { value } : { value: merge(pending.value, value) };
    },
    stop: () => {
      pending = null;
      if (typeof document !== "undefined") document.removeEventListener("visibilitychange", onVisibility);
    },
  };
}
