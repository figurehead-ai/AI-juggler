/**
 * AJ-E3301: each step of a walk timed as the person feels it — click to
 * feedback (the first change on screen after a click or ↩) and click to
 * done (no backend call in flight and nothing changing for a moment).
 * Off unless asked for (`localStorage["aij.walk-timing"] = "1"`, or a
 * Dev window opened with `?walk`): a walk sheet's ms column and L3's
 * journeys read the steps from `window.__AIJ_WALK__`, and each is logged
 * as "[walk] feedback 42 ms · done 180 ms". Times only — never what was
 * clicked or shown.
 */
import { inFlight } from "./ipc/timing";

export interface WalkStep {
  at: number;
  feedbackMs: number | null;
  doneMs: number | null;
}

/** Quiet for this long with no call in flight: the step is done. */
const SETTLED_MS = 300;
const KEEP = 100;

export function walkTimingWanted(): boolean {
  try {
    if (localStorage.getItem("aij.walk-timing") === "1") return true;
  } catch {
    // No storage: only the address can ask.
  }
  return import.meta.env.DEV && new URL(window.location.href).searchParams.has("walk");
}

/** Starts timing; returns the stop. */
export function startWalkTiming(target: Window = window): () => void {
  const steps: WalkStep[] = [];
  (target as unknown as { __AIJ_WALK__?: WalkStep[] }).__AIJ_WALK__ = steps;
  let current: { start: number; step: WalkStep; lastChange: number } | null = null;
  let timer = 0;
  const settle = () => {
    if (!current) return;
    const now = performance.now();
    if (inFlight() === 0 && now - current.lastChange >= SETTLED_MS) {
      current.step.doneMs = Math.round(current.lastChange - current.start);
      console.info(`[walk] feedback ${current.step.feedbackMs ?? "—"} ms · done ${current.step.doneMs} ms`);
      current = null;
      return;
    }
    timer = target.setTimeout(settle, 50);
  };
  const begin = () => {
    const start = performance.now();
    const step: WalkStep = { at: Date.now(), feedbackMs: null, doneMs: null };
    steps.push(step);
    if (steps.length > KEEP) steps.shift();
    current = { start, step, lastChange: start };
    target.clearTimeout(timer);
    timer = target.setTimeout(settle, 50);
  };
  const observer = new MutationObserver(() => {
    if (!current) return;
    const now = performance.now();
    if (current.step.feedbackMs === null) current.step.feedbackMs = Math.round(now - current.start);
    current.lastChange = now;
  });
  observer.observe(target.document.body, { subtree: true, childList: true, attributes: true, characterData: true });
  const onPointer = () => begin();
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Enter" || event.key === " ") begin();
  };
  target.addEventListener("pointerdown", onPointer, true);
  target.addEventListener("keydown", onKey, true);
  return () => {
    observer.disconnect();
    target.clearTimeout(timer);
    target.removeEventListener("pointerdown", onPointer, true);
    target.removeEventListener("keydown", onKey, true);
  };
}
