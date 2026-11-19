/**
 * The pool canvas's tidy layout (the rc3 critique, pure and tested): the
 * automatic layout's places with its members put so their lines cross
 * least. A pool never laid out opens with it once (useCanvas), and Tidy
 * up lays it out so; the user's own places are kept after.
 */
import { layout, layoutOrder, NODE_H, NODE_W, type LayoutNode, type Point } from "./layout";

/** A line between two members, either way (an edge's two ends). */
export type Link = { from_session_id: string; to_session_id: string };

/** Up to this many members under the lead every order may be tried (7!
 * is 5,040), else swaps that make it tidier; past MOST_MEMBERS a pool's
 * lines can't all be kept apart, and its automatic layout stands (a big
 * pool reads best in the List view). WORK bounds the tries by what each
 * costs (lines × lines and lines × nodes), so it never holds the page. */
const EVERY_ORDER = 7;
const MOST_MEMBERS = 24;
const WORK = 1_500_000;

/** How untidy the members' places are (their middles, by index): lines
 * crossing lines first, then lines behind a third node (with air round
 * it), then their length. */
function untidy(xs: Float64Array, ys: Float64Array, pairs: readonly (readonly [number, number])[]): number {
  let score = 0;
  const turn = (ax: number, ay: number, bx: number, by: number, cx: number, cy: number) => Math.sign((bx - ax) * (cy - ay) - (by - ay) * (cx - ax));
  for (let i = 0; i < pairs.length; i++) {
    const [a, b] = pairs[i]!;
    const [ax, ay, bx, by] = [xs[a]!, ys[a]!, xs[b]!, ys[b]!];
    score += Math.hypot(bx - ax, by - ay);
    for (let j = i + 1; j < pairs.length; j++) {
      const [c, d] = pairs[j]!;
      if (c === a || c === b || d === a || d === b) continue;
      const [cx, cy, dx, dy] = [xs[c]!, ys[c]!, xs[d]!, ys[d]!];
      if (turn(ax, ay, bx, by, cx, cy) * turn(ax, ay, bx, by, dx, dy) < 0 && turn(cx, cy, dx, dy, ax, ay) * turn(cx, cy, dx, dy, bx, by) < 0) score += 1e6;
    }
    // Behind a box: the line clipped to it (Liang–Barsky) is not empty.
    for (let n = 0; n < xs.length; n++) {
      if (n === a || n === b) continue;
      let from = 0;
      let to = 1;
      const sides = [
        [ax - bx, ax - (xs[n]! - NODE_W / 2 - 10)],
        [bx - ax, xs[n]! + NODE_W / 2 + 10 - ax],
        [ay - by, ay - (ys[n]! - NODE_H / 2 - 10)],
        [by - ay, ys[n]! + NODE_H / 2 + 10 - ay],
      ] as const;
      for (const [p, q] of sides) {
        if (p === 0) to = q < 0 ? -1 : to;
        else if (p < 0) from = Math.max(from, q / p);
        else to = Math.min(to, q / p);
      }
      if (from < to) score += 1e4;
    }
  }
  return score;
}

/**
 * The tidy layout (the rc3 critique): the automatic layout's places (the
 * leads on top, the rest in centred rows under them), the members put in
 * them so their lines cross least — no line across another, none behind
 * a third node, then the shortest. The same members and lines always
 * land the same way. Null when no line joins two of them, or past
 * MOST_MEMBERS (the automatic layout is it).
 */
export function tidyLayout(nodes: LayoutNode[], links: readonly Link[]): Map<string, Point> | null {
  const ordered = layoutOrder(nodes);
  const rest = ordered.filter((node) => node.role !== "lead");
  const index = new Map(ordered.map((node, at) => [node.id, at]));
  const keys = new Set<string>();
  const pairs: (readonly [number, number])[] = [];
  for (const link of links) {
    const a = index.get(link.from_session_id);
    const b = index.get(link.to_session_id);
    if (a === undefined || b === undefined || a === b || keys.has(`${Math.min(a, b)}|${Math.max(a, b)}`)) continue;
    keys.add(`${Math.min(a, b)}|${Math.max(a, b)}`);
    pairs.push([a, b]);
  }
  if (pairs.length === 0 || rest.length > MOST_MEMBERS) return null;
  const out = layout(nodes, new Map());
  // Each member's middle, by its index; the rest's places, in slot order.
  const xs = new Float64Array(ordered.map((node) => out.get(node.id)!.x + NODE_W / 2));
  const ys = new Float64Array(ordered.map((node) => out.get(node.id)!.y + NODE_H / 2));
  const first = ordered.length - rest.length;
  const slots = rest.map((node) => out.get(node.id)!);
  const put = (order: readonly number[]) => {
    order.forEach((member, slot) => {
      xs[first + member] = slots[slot]!.x + NODE_W / 2;
      ys[first + member] = slots[slot]!.y + NODE_H / 2;
    });
    return untidy(xs, ys, pairs);
  };
  const tries = Math.floor(WORK / (pairs.length * (pairs.length + ordered.length) + 1));
  let best = rest.map((_, at) => at);
  let score = put(best);
  const orders = rest.reduce((all, _, at) => all * (at + 1), 1);
  if (rest.length <= EVERY_ORDER && orders <= tries) {
    // Every order, from the automatic one; the first of the tidiest wins.
    const walk = (done: number[], left: number[]) => {
      if (left.length === 0) {
        const next = put(done);
        if (next < score) [best, score] = [[...done], next];
        return;
      }
      left.forEach((member, at) => walk([...done, member], [...left.slice(0, at), ...left.slice(at + 1)]));
    };
    walk([], best);
  } else {
    // Swaps while one makes it tidier, as many as the work allows.
    let spent = 0;
    for (let better = true; better && spent < tries; ) {
      better = false;
      for (let i = 0; i < best.length && spent < tries; i++)
        for (let j = i + 1; j < best.length && spent < tries; j++, spent++) {
          const next = [...best];
          [next[i], next[j]] = [next[j]!, next[i]!];
          const value = put(next);
          if (value < score) [best, score, better] = [next, value, true];
        }
    }
  }
  best.forEach((member, slot) => out.set(rest[member]!.id, slots[slot]!));
  return out;
}
