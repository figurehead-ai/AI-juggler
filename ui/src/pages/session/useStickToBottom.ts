/**
 * 0.3.5 (ADR-0024) — a conversation's scroll: the newest at the bottom.
 * Opening, it shows the bottom. New words keep the view at the bottom
 * only when the reader is already there; a reader who scrolled up stays
 * exactly where they are, and a "New messages" pill takes them down.
 * Earlier turns put above keep the turn they were reading in place.
 *
 * Where the reader is, is known from their own scrolling only (never
 * reset by an update), and the turn they read is found by its id
 * (`data-turn-id`), not by CSS scroll anchoring.
 */
import { useCallback, useLayoutEffect, useRef, useState, type RefObject } from "react";

/** How close to the bottom counts as at it (px). */
export const AT_BOTTOM_PX = 48;

type Anchor = { id: string; top: number } | null;

function anchorOf(box: HTMLElement): Anchor {
  for (const element of box.querySelectorAll<HTMLElement>("[data-turn-id]")) {
    if (element.offsetTop + element.offsetHeight > box.scrollTop) {
      return { id: element.dataset.turnId ?? "", top: element.offsetTop - box.scrollTop };
    }
  }
  return null;
}

export function useStickToBottom(
  box: RefObject<HTMLElement | null>,
  { firstId, lastKey }: { firstId: string | null; lastKey: string | null },
): { onScroll: () => void; pill: boolean; toBottom: () => void; keep: () => void } {
  const atBottom = useRef(true);
  const anchor = useRef<Anchor>(null);
  const seen = useRef<{ firstId: string | null; lastKey: string | null }>({ firstId: null, lastKey: null });
  const [pill, setPill] = useState(false);

  const onScroll = useCallback(() => {
    const element = box.current;
    if (!element) return;
    atBottom.current = element.scrollHeight - element.scrollTop - element.clientHeight < AT_BOTTOM_PX;
    anchor.current = anchorOf(element);
    if (atBottom.current) setPill(false);
  }, [box]);

  useLayoutEffect(() => {
    const element = box.current;
    if (!element || lastKey === null) return;
    const before = seen.current;
    seen.current = { firstId, lastKey };
    if (before.lastKey === null) {
      // Opening: the newest in view.
      element.scrollTop = element.scrollHeight;
    } else if (before.firstId !== firstId && anchor.current) {
      // Earlier turns came in above: the one being read stays put.
      const kept = anchor.current;
      const found = [...element.querySelectorAll<HTMLElement>("[data-turn-id]")].find((turn) => turn.dataset.turnId === kept.id);
      if (found) element.scrollTop = found.offsetTop - kept.top;
    }
    if (before.lastKey !== null && before.lastKey !== lastKey) {
      if (atBottom.current) element.scrollTop = element.scrollHeight;
      else setPill(true);
    }
    anchor.current = anchorOf(element);
  }, [box, firstId, lastKey]);

  const toBottom = useCallback(() => {
    const element = box.current;
    if (!element) return;
    element.scrollTop = element.scrollHeight;
    atBottom.current = true;
    setPill(false);
  }, [box]);

  /** The box changed size (a line over it came or went): a reader at
   * the bottom stays there. */
  const keep = useCallback(() => {
    const element = box.current;
    if (element && atBottom.current) element.scrollTop = element.scrollHeight;
  }, [box]);

  return { onScroll, pill, toBottom, keep };
}
