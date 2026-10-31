/**
 * AJ-E2108 — a box that scrolls says when it has more below what shows:
 * a "More below" chip under it (a click scrolls on), never a fade, which
 * made the last card look disabled. 0.3.2: shared by the session page's
 * side column and the shortcut sheet.
 */
import { useEffect, useRef, useState, type JSX, type RefObject } from "react";

/** The box to watch, whether it has more below, and its onScroll. */
export function useMoreBelow(): [RefObject<HTMLDivElement | null>, boolean, () => void] {
  const box = useRef<HTMLDivElement | null>(null);
  const [more, setMore] = useState(false);
  const measure = () => {
    const el = box.current;
    if (el) setMore(el.scrollHeight - el.clientHeight - el.scrollTop > 4);
  };
  useEffect(() => {
    measure();
    const el = box.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(measure);
    watch.observe(el);
    for (const child of el.children) watch.observe(child);
    return () => watch.disconnect();
  });
  return [box, more, measure];
}

/** The chip: under the box, never over it, so what shows stays whole. */
export function MoreBelow({ box, className = "", testId }: { box: RefObject<HTMLDivElement | null>; className?: string; testId?: string }): JSX.Element {
  return (
    <button
      type="button"
      onClick={() => box.current?.scrollBy({ top: box.current.clientHeight * 0.8, behavior: "smooth" })}
      className={`shrink-0 items-center justify-center gap-1.5 self-center rounded-full px-3 py-0.5 text-meta font-medium text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60 ${className || "inline-flex"}`}
      data-testid={testId}
    >
      More below
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M6 9l6 6 6-6" />
      </svg>
    </button>
  );
}
