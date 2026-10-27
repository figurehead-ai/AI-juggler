/**
 * D-45 — arranging Open now in Your order: drag a row (within its place)
 * or a place by its handle, or move the focused one with ⌥⌘↑ and ⌥⌘↓ (the
 * drag's keyboard twin; Ctrl+Alt elsewhere). Each move is said in a polite
 * live region ("Moved to position 2 of 5"). Pointer events only: no
 * library. Pinned rows move among the pinned ones, the others among the
 * others (pinned rows lead).
 */
import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type KeyboardEvent as ReactKeyboardEvent, type RefObject } from "react";
import type { RailItem } from "../lib/rail-items";
import type { RailGroup } from "../lib/rail-places";
import { movedWords, rowRange, type OrderLens } from "../lib/rail-order";
import { moveGroup, moveRow } from "../lib/rail-order-store";
import { matches } from "../lib/shortcuts";

/** A drag under way: what moves, from where, and where it would land. */
export interface Drag {
  kind: "row" | "group";
  key: string;
  /** The row's group (rows only). */
  group: string | null;
  from: number;
  to: number;
}

export interface Arrange {
  drag: Drag | null;
  /** What the live region says now. */
  said: string;
  gripRow: (event: ReactPointerEvent<HTMLElement>, group: string, key: string) => void;
  gripGroup: (event: ReactPointerEvent<HTMLElement>, group: string) => void;
  /** ⌥⌘↑ / ⌥⌘↓ on a focused row or place; true when it was one. */
  onKey: (event: ReactKeyboardEvent<HTMLElement>) => boolean;
}

/** Moves start past this many pixels, so a click on the handle isn't one. */
const SLOP = 4;

interface Inputs {
  root: RefObject<HTMLElement | null>;
  /** Your order is on: anything else only says how to turn it on. */
  enabled: boolean;
  lens: OrderLens;
  groups: readonly RailGroup[];
  arranged: ReadonlyMap<string, RailItem[]>;
  /** How many rows a group draws now (a move past them shows them all). */
  drawn: (group: string) => number;
  showAll: (group: string) => void;
}

const sel = (value: string) => (typeof CSS !== "undefined" && CSS.escape ? CSS.escape(value) : value.replace(/["\\]/g, "\\$&"));

/** Where `y` lands among `boxes` (the dragged one left out): how many of
 * the others' middles are above it. */
export function landing(y: number, boxes: readonly { key: string; top: number; bottom: number }[], self: string): number {
  return boxes.filter((box) => box.key !== self && (box.top + box.bottom) / 2 < y).length;
}

export function useArrange({ root, enabled, lens, groups, arranged, drawn, showAll }: Inputs): Arrange {
  const [drag, setDrag] = useState<Drag | null>(null);
  const [said, setSaid] = useState("");
  const latest = useRef({ lens, groups, arranged, drawn, showAll });
  latest.current = { lens, groups, arranged, drawn, showAll };
  const stop = useRef<(() => void) | null>(null);
  useEffect(() => () => stop.current?.(), []);

  // The same words twice in a row are still read out.
  const say = (words: string) => setSaid((before) => (before === words ? `${words}\u00a0` : words));

  const rowsOf = (group: string) => (latest.current.arranged.get(group) ?? []).map((item) => item.key);
  const groupKeys = () => latest.current.groups.map((group) => group.key);

  const commit = (move: Drag) => {
    const of = move.kind === "row" && move.group !== null ? rowsOf(move.group).length : groupKeys().length;
    if (move.to === move.from) {
      say(`Stays at position ${move.from + 1} of ${of}`);
      return;
    }
    if (move.kind === "row" && move.group !== null) {
      const keys = rowsOf(move.group);
      moveRow(keys, move.from, move.to);
      if (move.to >= latest.current.drawn(move.group)) latest.current.showAll(move.group);
      say(movedWords(move.to, keys.length));
    } else if (move.kind === "group") {
      const keys = groupKeys();
      moveGroup(latest.current.lens, keys, move.from, move.to);
      say(movedWords(move.to, keys.length));
    }
  };

  /** Follows the pointer from `event` until it's let go (or Escape). */
  const follow = (event: ReactPointerEvent<HTMLElement>, start: Drag, boxes: () => { key: string; top: number; bottom: number }[], range: [number, number]) => {
    if (!enabled || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    stop.current?.();
    const y0 = event.clientY;
    const measured = boxes();
    let moving = false;
    let current = start;
    const move = (next: PointerEvent) => {
      if (!moving && Math.abs(next.clientY - y0) < SLOP) return;
      moving = true;
      const to = Math.max(range[0], Math.min(range[1], landing(next.clientY, measured, start.key)));
      if (to !== current.to || current === start) {
        current = { ...start, to };
        setDrag(current);
      }
    };
    const end = (drop: boolean) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", key, true);
      stop.current = null;
      setDrag(null);
      if (drop && moving) commit(current);
    };
    const up = () => end(true);
    const cancel = () => end(false);
    const key = (next: KeyboardEvent) => {
      if (next.key !== "Escape") return;
      next.preventDefault();
      next.stopPropagation();
      end(false);
      say("Move cancelled");
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
    stop.current = () => end(false);
  };

  const measure = (selector: string, attribute: string) => () =>
    Array.from(root.current?.querySelectorAll<HTMLElement>(selector) ?? []).map((element) => {
      const box = element.getBoundingClientRect();
      return { key: element.getAttribute(attribute) ?? "", top: box.top, bottom: box.bottom };
    });

  const gripRow: Arrange["gripRow"] = (event, group, key) => {
    const all = latest.current.arranged.get(group) ?? [];
    const from = all.findIndex((item) => item.key === key);
    if (from < 0) return;
    const range = rowRange(all, from);
    follow(event, { kind: "row", key, group, from, to: from }, measure(`[data-order-group="${sel(group)}"][data-order-row]`, "data-order-row"), range);
  };

  const gripGroup: Arrange["gripGroup"] = (event, group) => {
    const keys = groupKeys();
    const from = keys.indexOf(group);
    if (from < 0) return;
    follow(event, { kind: "group", key: group, group: null, from, to: from }, measure("[data-order-head]", "data-order-head"), [0, keys.length - 1]);
  };

  const onKey: Arrange["onKey"] = (event) => {
    if (!matches("rail-move", event)) return false;
    const target = event.target as HTMLElement;
    const row = target.closest<HTMLElement>("[data-order-row]");
    const head = row ? null : target.closest<HTMLElement>("[data-order-head]");
    if (!row && !head) return false;
    event.preventDefault();
    event.stopPropagation();
    if (!enabled) {
      say("Rows stay in this order. Pick Your order on Open now's header to arrange them.");
      return true;
    }
    const step = event.key === "ArrowUp" ? -1 : 1;
    if (row) {
      const group = row.getAttribute("data-order-group") ?? "";
      const key = row.getAttribute("data-order-row") ?? "";
      const all = latest.current.arranged.get(group) ?? [];
      const from = all.findIndex((item) => item.key === key);
      if (from < 0) return true;
      const [first, last] = rowRange(all, from);
      const to = Math.max(first, Math.min(last, from + step));
      commit({ kind: "row", key, group, from, to });
      refocus(`[data-order-row="${sel(key)}"] [data-rail-item]`);
    } else if (head) {
      const key = head.getAttribute("data-order-head") ?? "";
      const keys = groupKeys();
      const from = keys.indexOf(key);
      if (from < 0) return true;
      const to = Math.max(0, Math.min(keys.length - 1, from + step));
      commit({ kind: "group", key, group: null, from, to });
      refocus(`[data-order-head="${sel(key)}"] [data-testid="place-page"]`);
    }
    return true;
  };

  /** Focus stays on what moved, wherever it's drawn now. */
  const refocus = (selector: string) =>
    requestAnimationFrame(() => {
      const element = root.current?.querySelector<HTMLElement>(selector);
      element?.focus({ preventScroll: false });
    });

  return { drag, said, gripRow, gripGroup, onKey };
}
