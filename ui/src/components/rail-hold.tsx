/**
 * 0.4.1 — what's under the pointer or focus holds still. The rail keeps
 * its rows' ORDER while the pointer or focus is in the list (rail-lines'
 * `frozen`), but a line drawn above them (a Needs you notice arriving, a
 * row ending, a line growing) still pushed them down: a click aimed at a
 * place's Open door landed on the notice, or nowhere (3efc37b61).
 *
 * So while the pointer or keyboard focus is in the list, a change in
 * height above the line under it (the focused one first, when focus came
 * from the keyboard) is scrolled away in the same frame, before paint:
 * a layout effect after each render and a ResizeObserver on the list
 * (WKWebView has no CSS `overflow-anchor`). The new line is drawn above,
 * out of view if need be, and a small "1 new above" pill at the view's
 * top says so (until it's scrolled into view, or the pill is clicked).
 * A MutationObserver holds a line put in outside a render before the
 * next input event.
 *
 * With no room to scroll, a spacer makes it: below the list when lines
 * arrive at the top (the list scrolls into it), above when lines leave
 * while the list is at its top. The bottom one shrinks as the list
 * scrolls back up; the top one goes when the pointer and focus leave.
 * Nothing animates but the pill's scroll, and that only without reduced
 * motion.
 */
import { useEffect, useLayoutEffect, useRef, useState, type JSX, type RefObject } from "react";
import { IconChevronDown } from "./icons-controls";

/** The attribute a line's wrapper carries: its stable key (lib/rail-lines' lineKey). */
export const LINE_KEY = "data-line-key";
const LINE = `[${LINE_KEY}]`;

/** The kinds of line the pill counts (not section heads, headers or links). */
const COUNTED = /^(notice|need|row|plain|past|idle):/;

/** Spacers made so the list can hold still. */
interface Pads {
  top: number;
  bottom: number;
}

/** The boxes that scroll `element`, nearest first: those styled to scroll
 * (one may grow with its content, never scrolling: the rail's list sits
 * in the rail's scroll, so each is tried in turn). */
function scrollersOf(element: HTMLElement): HTMLElement[] {
  const out: HTMLElement[] = [];
  for (let at = element.parentElement; at && at !== document.body; at = at.parentElement) {
    if (/auto|scroll/.test(getComputedStyle(at).overflowY)) out.push(at);
  }
  return out;
}

/** Scrolls `by` (down when positive) across `boxes`, nearest first, as far
 * as each goes; what none could take is returned. */
function scrollAcross(boxes: readonly HTMLElement[], by: number): number {
  let rest = by;
  for (const box of boxes) {
    if (Math.abs(rest) < 0.5) break;
    const was = box.scrollTop;
    box.scrollTop = was + rest;
    rest -= box.scrollTop - was;
  }
  return rest;
}

function reducedMotion(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function keyboardFocus(element: Element): boolean {
  try {
    return element.matches(":focus-visible");
  } catch {
    return false;
  }
}

interface Anchor {
  key: string;
  /** Its top in the list's content (the viewport's plus the scrolls): a
   * scroll of the user's isn't a change to undo, only a change in layout. */
  top: number;
  /** The lines drawn above it then, by key. */
  above: Set<string>;
}

interface HoldStill {
  /** The list's own handlers (pointer and focus, in and out). */
  props: {
    onPointerEnter: (event: React.PointerEvent<HTMLElement>) => void;
    onPointerMove: (event: React.PointerEvent<HTMLElement>) => void;
    onPointerLeave: () => void;
    onFocus: () => void;
    onBlur: () => void;
  };
  /** Drawn first in the list: the pill (when something new arrived above,
   * out of view) and the spacer above. */
  before: JSX.Element;
  /** Drawn last: the spacer below. */
  after: JSX.Element;
}

/** Keeps the line under the pointer or focus still in `root`; `enter` and
 * `leave` are told when the pointer or focus comes in and goes (`leave` a
 * moment later, as focus moves on inside). */
export function useHoldStill(root: RefObject<HTMLElement | null>, on: { enter?: () => void; leave?: () => void } = {}): HoldStill {
  const topPad = useRef<HTMLDivElement | null>(null);
  const bottomPad = useRef<HTMLDivElement | null>(null);
  const pads = useRef<Pads>({ top: 0, bottom: 0 });
  const pointer = useRef<{ x: number; y: number; key: string | null } | null>(null);
  const anchor = useRef<Anchor | null>(null);
  // The new lines above (the pill's), kept in a ref too for the handlers.
  const [fresh, setFreshState] = useState<readonly string[]>([]);
  const freshRef = useRef<readonly string[]>([]);
  const setFresh = (next: readonly string[]) => {
    freshRef.current = next;
    setFreshState(next);
  };

  const setPads = (next: Pads) => {
    pads.current = next;
    if (topPad.current) topPad.current.style.height = `${next.top}px`;
    if (bottomPad.current) bottomPad.current.style.height = `${next.bottom}px`;
  };
  const boxes = (): HTMLElement[] => (root.current ? scrollersOf(root.current) : []);
  const scrolled = (list: readonly HTMLElement[]) => list.reduce((sum, box) => sum + box.scrollTop, 0);

  const lineOf = (key: string): HTMLElement | null => {
    for (const line of root.current?.querySelectorAll<HTMLElement>(LINE) ?? []) if (line.getAttribute(LINE_KEY) === key) return line;
    return null;
  };
  const keysAbove = (line: HTMLElement): Set<string> => {
    const out = new Set<string>();
    for (const other of root.current?.querySelectorAll<HTMLElement>(LINE) ?? []) {
      if (other === line) break;
      out.add(other.getAttribute(LINE_KEY)!);
    }
    return out;
  };

  /** Keyboard focus in the list (a row clicked keeps focus, but the
   * pointer may be far away by then: that isn't someone reading here). */
  const keyed = (): Element | null => {
    const active = document.activeElement;
    return active && root.current?.contains(active) && keyboardFocus(active) ? active : null;
  };
  const inside = (): boolean => pointer.current !== null || keyed() !== null;

  /** The line to hold: keyboard focus's, else the pointer's. */
  const pick = (): HTMLElement | null => {
    const list = root.current;
    if (!list) return null;
    const focused = keyed()?.closest<HTMLElement>(LINE);
    if (focused) return focused;
    const at = pointer.current;
    if (!at) return null;
    const hit = typeof document.elementFromPoint === "function" ? document.elementFromPoint(at.x, at.y)?.closest<HTMLElement>(LINE) : null;
    return hit && list.contains(hit) ? hit : at.key !== null ? lineOf(at.key) : null;
  };

  const record = (line: HTMLElement | null = pick()) => {
    anchor.current = line ? { key: line.getAttribute(LINE_KEY)!, top: line.getBoundingClientRect().top + scrolled(boxes()), above: keysAbove(line) } : null;
  };

  /** Where the list's view starts, on screen (the innermost box's top). */
  const edgeOf = (list: readonly HTMLElement[]): number | null => (list.length === 0 ? null : Math.max(...list.map((box) => box.getBoundingClientRect().top)));
  /** Lines new above, not wholly in view now (the pill's). */
  const hiddenOf = (keys: Iterable<string>): string[] => {
    const edge = edgeOf(boxes());
    if (edge === null) return [];
    return [...keys].filter((key) => {
      const line = lineOf(key);
      return line !== null && line.getBoundingClientRect().top < edge - 1;
    });
  };
  /** The pill at the view's top, wherever the list is scrolled to. */
  const cueRef = useRef<HTMLDivElement | null>(null);
  const placeCue = () => {
    const cue = cueRef.current;
    const at = cue?.parentElement;
    const edge = edgeOf(boxes());
    if (!cue || !at || edge === null) return;
    cue.style.top = `${Math.max(0, edge - at.getBoundingClientRect().top) + 4}px`;
  };

  const hold = () => {
    const was = anchor.current;
    if (!inside() || !root.current) return;
    if (!was) return record();
    const line = lineOf(was.key);
    if (!line) return record();
    const list = boxes();
    const delta = line.getBoundingClientRect().top + scrolled(list) - was.top;
    if (Math.abs(delta) < 0.5) return;
    if (delta > 0) {
      // A spacer above made earlier gives way first (it moves the line up),
      // then the scroll; with no room left, a spacer below makes it.
      const give = Math.min(pads.current.top, delta);
      if (give > 0) setPads({ top: pads.current.top - give, bottom: pads.current.bottom });
      let rest = scrollAcross(list, delta - give);
      for (let tries = 0; tries < 2 && rest > 0.5; tries += 1) {
        setPads({ top: pads.current.top, bottom: pads.current.bottom + rest });
        rest = scrollAcross(list, rest);
      }
      const added = [...keysAbove(line)].filter((key) => !was.above.has(key) && COUNTED.test(key));
      const hidden = hiddenOf(added);
      if (hidden.length > 0) setFresh([...new Set([...freshRef.current, ...hidden])]);
    } else {
      // At the top: a spacer above takes what the scroll can't.
      const rest = scrollAcross(list, delta);
      if (rest < -0.5) setPads({ top: pads.current.top - rest, bottom: pads.current.bottom });
    }
    record(line);
  };

  // After every render (a notice arrived, a row ended), before paint.
  useLayoutEffect(hold);
  useLayoutEffect(() => {
    if (fresh.length > 0) placeCue();
  });
  // A line that grows or shrinks by itself (a title that wraps).
  const holdRef = useRef(hold);
  useLayoutEffect(() => {
    holdRef.current = hold;
  });
  useEffect(() => {
    const list = root.current;
    if (!list || typeof ResizeObserver === "undefined") return undefined;
    const observer = new ResizeObserver(() => holdRef.current());
    observer.observe(list);
    // A line put in or taken out outside a render of ours: held before
    // the next input is handled (a press and its release can come before
    // a frame does, as 3efc37b61 measured).
    const changes = typeof MutationObserver === "undefined" ? null : new MutationObserver(() => holdRef.current());
    changes?.observe(list, { childList: true, subtree: true, characterData: true });
    return () => {
      observer.disconnect();
      changes?.disconnect();
    };
  }, [root]);

  // The user's scroll: the spacers shrink to what still holds the view,
  // the line under the pointer is the new one held, and the pill goes
  // once its lines are in view.
  const onScroll = () => {
    const list = boxes();
    const { top, bottom } = pads.current;
    if (bottom > 0) {
      // Back at the top none is needed; else what no box has spare below.
      const atTop = list.every((box) => box.scrollTop <= 0.5);
      const slack = Math.max(0, ...list.map((box) => box.scrollHeight - box.clientHeight - box.scrollTop));
      if (atTop || slack > 0) setPads({ top, bottom: atTop ? 0 : Math.max(0, bottom - slack) });
    }
    const box = list.find((one) => one.scrollTop >= top);
    if (top > 0 && box) {
      setPads({ top: 0, bottom: pads.current.bottom });
      box.scrollTop -= top;
    }
    if (inside()) record();
    if (freshRef.current.length > 0) {
      if (hiddenOf(freshRef.current).length === 0) setFresh([]);
      else placeCue();
    }
  };
  const onScrollRef = useRef(onScroll);
  useLayoutEffect(() => {
    onScrollRef.current = onScroll;
  });
  useEffect(() => {
    // Scroll events don't bubble: caught on the way down, for any box of ours.
    const heard = (event: Event) => {
      const target = event.target;
      if (target instanceof Node && root.current && target !== root.current && target.contains(root.current)) onScrollRef.current();
    };
    window.addEventListener("scroll", heard, { capture: true, passive: true });
    return () => window.removeEventListener("scroll", heard, { capture: true });
  }, [root]);

  /** Pointer and focus gone: a spacer above goes (nothing is aimed at now). */
  const release = () => {
    if (inside()) return;
    anchor.current = null;
    if (pads.current.top > 0) setPads({ top: 0, bottom: pads.current.bottom });
  };

  const onPointerMove = (event: React.PointerEvent<HTMLElement>) => {
    const line = event.target instanceof Element ? event.target.closest(LINE) : null;
    const key = line?.getAttribute(LINE_KEY) ?? null;
    const had = pointer.current?.key;
    pointer.current = { x: event.clientX, y: event.clientY, key };
    if (key !== had || !anchor.current) record();
  };
  const leave = () =>
    setTimeout(() => {
      release();
      on.leave?.();
    });
  const props: HoldStill["props"] = {
    onPointerEnter: (event) => {
      on.enter?.();
      onPointerMove(event);
    },
    onPointerMove,
    onPointerLeave: () => {
      pointer.current = null;
      leave();
    },
    onFocus: () => {
      on.enter?.();
      record();
    },
    onBlur: leave,
  };

  const showAbove = (keyboard: boolean) => {
    const lines = freshRef.current.map(lineOf).filter((line): line is HTMLElement => line !== null);
    setFresh([]);
    const first = lines[0];
    if (!first) return;
    first.scrollIntoView({ block: "nearest", behavior: reducedMotion() ? "auto" : "smooth" });
    // From the keyboard, focus goes to it (the pill it was on is gone).
    if (keyboard) first.querySelector<HTMLElement>("button, [tabindex]")?.focus({ preventScroll: true });
  };

  const cue =
    fresh.length > 0 ? (
      // Zero high, its pill laid over the view's top: it moves nothing.
      <div className="relative h-0">
        <div ref={cueRef} className="pointer-events-none absolute inset-x-0 top-0 z-10 flex justify-center">
          <button
            type="button"
            data-testid="rail-new-above"
            onClick={(event) => showAbove(event.detail === 0)}
            className="pointer-events-auto inline-flex min-h-6 items-center gap-1 rounded-full border border-accent/40 bg-surface px-2.5 text-label font-medium text-ink shadow-card outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60 motion-safe:animate-[aij-fade-in_140ms_ease-out]"
          >
            <IconChevronDown width={12} height={12} strokeWidth={2} aria-hidden="true" className="rotate-180" />
            {fresh.length} new above
          </button>
        </div>
      </div>
    ) : null;

  return {
    props,
    before: (
      <>
        {cue}
        <div ref={topPad} aria-hidden="true" data-testid="rail-hold-top" />
      </>
    ),
    after: <div ref={bottomPad} aria-hidden="true" data-testid="rail-hold-bottom" />,
  };
}
