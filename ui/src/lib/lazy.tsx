/**
 * Code that loads when it's first needed, not at start: screens past
 * Home, sheets and dialogs, and the terminal (xterm is a third of the
 * script). The first window paints from only what it shows; once it has,
 * the rest is fetched in idle time ([`prefetchLazy`]), so opening a
 * screen doesn't wait.
 */
import { lazy, Suspense, type ComponentProps, type ComponentType, type JSX } from "react";

const loaders: { load: () => Promise<unknown>; first: boolean }[] = [];

/** `module[name]`, loaded on first use. It waits on its own (nothing
 * around it blanks while it loads), and is fetched after first paint
 * anyway: `first` puts it at the head of that queue (what's opened most
 * often right after start). */
export function lazyNamed<M, K extends keyof M>(loader: () => Promise<M>, name: K, first = false): M[K] {
  loaders.push({ load: loader, first });
  type C = ComponentType<Record<string, unknown>>;
  const Lazy = lazy(() => loader().then((module) => ({ default: module[name] as unknown as C })));
  function Loaded(props: ComponentProps<C>): JSX.Element {
    return (
      <Suspense fallback={null}>
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
