/**
 * Code that loads when it's first needed, not at start: screens past
 * Home, sheets and dialogs, and the terminal (xterm is a third of the
 * script). The first window paints from only what it shows; once it has,
 * the rest is fetched in idle time ([`prefetchLazy`]), so opening a
 * screen doesn't wait.
 */
import { lazy, Suspense, useEffect, useState, type ComponentProps, type ComponentType, type JSX, type ReactNode } from "react";

const loaders: { load: () => Promise<unknown>; first: boolean }[] = [];

/** How many parts are loading now. While any is, `<html>` carries
 * `data-loading-parts`: a part on screen may still be its empty
 * placeholder, so the visual gate waits for it to clear before a capture
 * (a terminal pane was caught empty on some runs and drawn on others). */
let loading = 0;

function add(by: number): void {
  loading += by;
  if (typeof document !== "undefined") document.documentElement.toggleAttribute("data-loading-parts", loading > 0);
}

function counted<M>(loader: () => Promise<M>): () => Promise<M> {
  return () => {
    add(1);
    return loader().finally(() => add(-1));
  };
}

/** Suspense's empty placeholder, counted while it's on screen: React
 * holds a lazy part's reveal up to 300 ms after its code is here, so the
 * code's own count had ended while the part was still blank (the gate's
 * selection-line: the pool line's conversation empty in some renders).
 * Its count ends in the same flush as the revealed part's first effects,
 * so a part that then reads its data (`loadingPart`) leaves no gap. */
function Pending({ children }: { children?: ReactNode }): JSX.Element | null {
  useEffect(() => {
    add(1);
    return () => add(-1);
  }, []);
  return children === undefined ? null : <>{children}</>;
}

/** A part's first data, counted like its code: while it loads, the part
 * on screen is its placeholder (the pool line's conversation was caught
 * as its skeleton in some renders and drawn in others), so the gate waits
 * for it too. Only a part's first read: a re-read keeps what it shows. */
export function loadingPart<T>(work: Promise<T>): Promise<T> {
  return counted(() => work)();
}

/** `module[name]`, loaded on first use. It waits on its own (nothing
 * around it blanks while it loads), and is fetched after first paint
 * anyway: `first` puts it at the head of that queue (what's opened most
 * often right after start). Once its code is here (that prefetch, or an
 * earlier use) it renders at once: a first open never goes through
 * Suspense, whose reveal React holds for 300 ms (a blank main area on
 * the first open of Projects, Usage, History or Tasks). `placeholder` is
 * drawn while its code loads (SK-8: a skeleton, never an empty box). */
export function lazyNamed<M, K extends keyof M>(loader: () => Promise<M>, name: K, first = false, placeholder?: () => ReactNode): M[K] {
  type C = ComponentType<Record<string, unknown>>;
  let ready: C | null = null;
  const counting = counted(loader);
  const load = () =>
    counting().then((module) => {
      ready = module[name] as unknown as C;
      return module;
    });
  loaders.push({ load, first });
  const Lazy = lazy(() => load().then((module) => ({ default: module[name] as unknown as C })));
  function Loaded(props: ComponentProps<C>): JSX.Element {
    // Decided once per use: a part shown while it loaded keeps its
    // Suspense, so its arrival never remounts it (and its state).
    const [direct] = useState(() => ready);
    if (direct) {
      const Ready = direct;
      return <Ready {...props} />;
    }
    return (
      <Suspense fallback={<Pending>{placeholder?.()}</Pending>}>
        <Lazy {...props} />
      </Suspense>
    );
  }
  Loaded.displayName = `Lazy(${String(name)})`;
  return Loaded as unknown as M[K];
}

/** Reads a main screen's first data in the same idle queue, after the
 * parts opened first (0.1.5 review: a screen opened blank for seconds). */
export function prefetchData(load: () => Promise<unknown>): void {
  loaders.push({ load, first: false });
}

let started = false;

/** Fetches every lazily loaded part, one at a time in idle time, once the
 * first window has painted. Later calls do nothing. */
export function prefetchLazy(): void {
  if (started) return;
  started = true;
  const idle = (next: () => void) => {
    // A part can finish loading after its window is gone (a test's
    // environment torn down): then there is nothing left to fetch for.
    if (typeof window === "undefined") return;
    const w = window as unknown as { requestIdleCallback?: (fn: () => void, opts?: { timeout: number }) => void };
    if (w.requestIdleCallback) w.requestIdleCallback(next, { timeout: 2000 });
    else setTimeout(next, 50);
  };
  const queue = [...loaders.filter((entry) => entry.first), ...loaders.filter((entry) => !entry.first)];
  const step = () => {
    const entry = queue.shift();
    if (!entry) return;
    void entry
      .load()
      .catch(() => undefined)
      .finally(() => idle(step));
  };
  // Just after the first paint, not during it.
  requestAnimationFrame(() => setTimeout(() => idle(step), 0));
}
