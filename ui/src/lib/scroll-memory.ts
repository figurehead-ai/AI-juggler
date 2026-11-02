/**
 * 0.3.4 — where each screen was scrolled to, kept while the app runs (the
 * page-switching report: every return to Home, Projects or Usage started
 * at the top). A screen's scroll box takes `useScrollMemory(key)` as its
 * ref: it records the box's scroll as it moves, and puts it back when
 * the box is shown again — at once when the content is already there
 * (cached), else as it arrives, for up to a second, and never against
 * the user (a wheel, a touch, a key or a click in it stops it) or against
 * the screen's own scrolling (a section it scrolls to).
 *
 * `subject` is what the box shows (a search, a section): a new one starts
 * at the top. At most `KEEP` boxes are remembered, the least recently
 * scrolled forgotten first.
 */
import { useCallback } from "react";
import { whenForgotten } from "./shared-read";

const KEEP = 100;
/** How long a return waits for its content to be tall enough. */
const RESTORE_MS = 1000;

const memory = new Map<string, { subject: string; top: number }>();

function remember(key: string, subject: string, top: number): void {
  memory.delete(key);
  memory.set(key, { subject, top });
  while (memory.size > KEEP) memory.delete(memory.keys().next().value as string);
}

/** Where `key`'s box was left for `subject` (0: the top). */
export function rememberedScroll(key: string, subject = ""): number {
  const kept = memory.get(key);
  return kept && kept.subject === subject ? kept.top : 0;
}

whenForgotten(() => memory.clear());

const INTENT = ["wheel", "touchstart", "keydown", "pointerdown"] as const;

/** A ref for a scroll box whose position is kept under `key` (`null`:
 * not kept). `active` false: the box is hidden (a tab not shown); its
 * position is put back when it shows. */
export function useScrollMemory<E extends HTMLElement = HTMLDivElement>(
  key: string | null,
  { subject = "", active = true }: { subject?: string; active?: boolean } = {},
): (element: E | null) => (() => void) | undefined {
  return useCallback(
    (element: E | null) => {
      if (!element || key === null) return undefined;
      const kept = memory.get(key);
      if (kept && kept.subject !== subject) remember(key, subject, 0);
      const want = rememberedScroll(key, subject);
      // A box kept on screen for another subject (one session's page to
      // the next) goes to that one's place, or the top.
      let restoring = active && Math.abs(element.scrollTop - want) > 1;
      let frame = 0;
      let set = -1;
      const deadline = performance.now() + RESTORE_MS;
      const stop = () => {
        restoring = false;
        cancelAnimationFrame(frame);
      };
      const step = () => {
        if (!restoring) return;
        // Something else moved it since (the screen's own scrolling).
        if (set >= 0 && Math.abs(element.scrollTop - set) > 1) return stop();
        element.scrollTop = want;
        set = element.scrollTop;
        if (Math.abs(set - want) <= 1 || performance.now() > deadline) return stop();
        frame = requestAnimationFrame(step);
      };
      step();
      const onScroll = () => {
        if (!restoring) remember(key, subject, element.scrollTop);
      };
      element.addEventListener("scroll", onScroll, { passive: true });
      INTENT.forEach((name) => element.addEventListener(name, stop, { passive: true }));
      return () => {
        stop();
        element.removeEventListener("scroll", onScroll);
        INTENT.forEach((name) => element.removeEventListener(name, stop));
      };
    },
    [key, subject, active],
  );
}
