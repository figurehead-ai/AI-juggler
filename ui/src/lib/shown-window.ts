/**
 * 0.4.1, idle CPU: the window's own polling follows whether it's seen.
 * A poll runs every `everyMs` while the window is shown and focused,
 * `unfocusedTimes` slower while it's shown but another app is in front,
 * and not at all while it's hidden or minimised; when it's shown again it
 * runs once at once, then on its cadence. Events from the backend still
 * arrive while hidden: this is only the window's own asking.
 */

import { listen } from "./ipc/transport";

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

/** D-112: runs `run` each time the window comes back on screen (shown
 * again after a close hid it, restored, or uncovered); never while it's
 * hidden, and not at once. Returns the stop. */
export function whenShownAgain(run: () => void): () => void {
  const onVisibility = () => {
    if (windowShown()) run();
  };
  document.addEventListener("visibilitychange", onVisibility);
  return () => document.removeEventListener("visibilitychange", onVisibility);
}

/**
 * 0.4.5 (AJ-E805, round 3): a read asked for on every change event runs
 * at most once per `everyMs`. The first ask after a quiet spell runs at
 * once (a change still shows at once); every ask while it is too soon
 * makes one read at the spell's end, never more. With 10 sessions at
 * work, the shown window read each list on every event: the relation
 * counts 24–36 times a minute, the shell's lists and the conversation
 * list 8–10 each.
 */
export function coalesced(run: () => void, everyMs = 1000): { ask: () => void; stop: () => void } {
  let last = Number.NEGATIVE_INFINITY;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let stopped = false;
  const fire = () => {
    timer = null;
    if (stopped) return;
    last = Date.now();
    run();
  };
  return {
    ask: () => {
      if (stopped || timer !== null) return;
      const wait = last + everyMs - Date.now();
      if (wait <= 0) fire();
      else timer = setTimeout(fire, wait);
    },
    stop: () => {
      stopped = true;
      if (timer !== null) clearTimeout(timer);
      timer = null;
    },
  };
}

/**
 * 0.4.5 (AJ-E805, round 3): a backend event the page listens to only
 * while the window can be seen. Every event reaches the page as a script
 * WebKit runs at foreground priority, so in a hidden or minimised window
 * each one raised the web content, GPU and networking processes to the
 * foreground and dropped them back (about 60 events a minute with 10
 * sessions at work: the hidden window's web content at 0.6–0.7 % and its
 * GPU process at 0.3–0.4 % of a core, drawing nothing). While the window
 * is hidden the page holds no listener for the event, so the backend
 * sends it nothing; when it shows, it listens again and then runs
 * `caughtUp` once, to read what it may have missed (it may hand that to
 * `heard`, which it is given). Returns the stop.
 */
export function listenWhileShown<T>(event: string, heard: (payload: T) => void, caughtUp: (heard: (payload: T) => void) => void): () => void {
  let unlisten: (() => void) | null = null;
  // The listen under way, by number (0: none); a stop or a hide meanwhile
  // makes it stop as soon as it lands.
  let asking = 0;
  let asked = 0;
  let gone = false;
  const attach = (then?: () => void) => {
    if (gone || unlisten !== null || asking !== 0) return;
    const mine = (asking = ++asked);
    const landed = (stop: (() => void) | null) => {
      if (asking !== mine) return stop?.();
      asking = 0;
      unlisten = stop;
      then?.();
    };
    listen<T>(event, (message) => heard(message.payload)).then(landed, () => landed(null));
  };
  const detach = () => {
    asking = 0;
    unlisten?.();
    unlisten = null;
  };
  // Read after the listener is in place: a change after the read is heard.
  const onVisibility = () => (windowShown() ? attach(() => caughtUp(heard)) : detach());
  document.addEventListener("visibilitychange", onVisibility);
  if (windowShown()) attach();
  return () => {
    gone = true;
    detach();
    document.removeEventListener("visibilitychange", onVisibility);
  };
}
