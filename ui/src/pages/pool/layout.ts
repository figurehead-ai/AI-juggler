/**
 * The pool canvas's geometry (pure, tested): the automatic first layout
 * (the lead on top, the others in centred rows under it; the same members
 * always land in the same places), fitting the canvas to the window, and
 * the curve each edge takes.
 *
 * Canvas units are pixels at 100 %. A node's position is its top-left
 * corner.
 */

/** A node's box at 100 %. */
export const NODE_W = 232;
export const NODE_H = 112;
/** The room between boxes in the automatic layout. */
const GAP_X = 40;
/** Room for a node that grows with what waits on it (an ask's answers). */
const GAP_Y = 104;

export type Point = { x: number; y: number };
export type LayoutNode = { id: string; role: string };

const ROLE_RANK: Record<string, number> = { lead: 0, member: 1, observer: 2 };

/** The order the automatic layout fills its places in: leads, members,
 * observers; then by id (never by activity, so nothing moves on its own). */
export function layoutOrder(nodes: LayoutNode[]): LayoutNode[] {
  return [...nodes].sort((a, b) => (ROLE_RANK[a.role] ?? 1) - (ROLE_RANK[b.role] ?? 1) || a.id.localeCompare(b.id));
}

/** How many to a row under the lead: up to three in one row, then a
 * block a little wider than it is tall (5 → 3 + 2, 20 → 5 × 4). */
function columnsFor(count: number): number {
  if (count <= 3) return Math.max(1, count);
  return Math.ceil(Math.sqrt(count * 1.2));
}

/** The places of a centred block of `count` boxes starting at `top`,
 * `columns` to a row. */
function rowsOf(count: number, top: number, columns: number): Point[] {
  const out: Point[] = [];
  for (let index = 0; index < count; index += 1) {
    const row = Math.floor(index / columns);
    const inRow = Math.min(columns, count - row * columns);
    const column = index % columns;
    const width = inRow * NODE_W + (inRow - 1) * GAP_X;
    out.push({ x: -width / 2 + column * (NODE_W + GAP_X), y: top + row * (NODE_H + GAP_Y) });
  }
  return out;
}

/** Two boxes overlap (with a little room around them). */
function overlaps(a: Point, b: Point, room = 16): boolean {
  return Math.abs(a.x - b.x) < NODE_W + room && Math.abs(a.y - b.y) < NODE_H + room;
}

/**
 * Every node's place: where the user put it, else the automatic layout's.
 * The leads share the top row; the rest fill centred rows under them. A
 * place the user already took is skipped, so an automatic node never
 * lands on one the user put down.
 */
export function layout(nodes: LayoutNode[], placed: ReadonlyMap<string, Point>): Map<string, Point> {
  const out = new Map<string, Point>();
  for (const node of nodes) {
    const at = placed.get(node.id);
    if (at) out.set(node.id, at);
  }
  const ordered = layoutOrder(nodes);
  const leads = ordered.filter((node) => node.role === "lead");
  const rest = ordered.filter((node) => node.role !== "lead");
  const taken = [...out.values()];
  const fill = (group: LayoutNode[], top: number) => {
    const waiting = group.filter((node) => !out.has(node.id));
    if (waiting.length === 0) return;
    // Enough places that the user's own never run them out.
    const slots = rowsOf(group.length + taken.length, top, columnsFor(group.length));
    let next = 0;
    for (const node of waiting) {
      while (next < slots.length && taken.some((at) => overlaps(at, slots[next]!))) next += 1;
      const slot = slots[next] ?? { x: next * (NODE_W + GAP_X), y: top };
      next += 1;
      out.set(node.id, slot);
      taken.push(slot);
    }
  };
  fill(leads, 0);
  fill(rest, leads.length > 0 ? NODE_H + GAP_Y * 1.5 : 0);
  return out;
}

/** The box around every node. */
export function bounds(points: Iterable<Point>): { x: number; y: number; w: number; h: number } | null {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const at of points) {
    minX = Math.min(minX, at.x);
    minY = Math.min(minY, at.y);
    maxX = Math.max(maxX, at.x + NODE_W);
    maxY = Math.max(maxY, at.y + NODE_H);
  }
  return minX === Infinity ? null : { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}

export type View = { x: number; y: number; scale: number };
export const MIN_SCALE = 0.3;
/** Fit never goes below this: a node still has room for its name and
 * its state on two lines (they draw at screen size below 100 %), and
 * past it you pan. */
export const FIT_MIN_SCALE = 0.45;
export const MAX_SCALE = 1.6;
export const clampScale = (scale: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));

/** The view that shows every node in a `width` × `height` window, never
 * larger than 100 %. */
export function fitView(points: Iterable<Point>, width: number, height: number, pad = 48, left = 0, top = 0): View {
  const box = bounds(points);
  const room = Math.max(width - left, width / 2);
  const middle = width - room / 2;
  if (!box || width <= 0 || height <= 0) return { x: middle - NODE_W / 2, y: pad, scale: 1 };
  // A node grows below its box with what waits on it: room for that too.
  const tall = box.h + NODE_H;
  const high = height - top;
  const scale = clampScale(Math.max(FIT_MIN_SCALE, Math.min(1, (room - pad * 2) / box.w, (high - pad * 2) / tall)));
  // Too big to fit at a readable size: the top (the lead) in view.
  const y = tall * scale > high - pad * 2 ? top + pad - box.y * scale : top + high / 2 - (box.y + tall / 2) * scale;
  return { x: middle - (box.x + box.w / 2) * scale, y, scale };
}

/** Zooms by `factor` around a point of the window, keeping it still. */
export function zoomAt(view: View, factor: number, at: Point): View {
  const scale = clampScale(view.scale * factor);
  const ratio = scale / view.scale;
  return { scale, x: at.x - (at.x - view.x) * ratio, y: at.y - (at.y - view.y) * ratio };
}

/** Where a window point is on the canvas. */
export function toCanvas(view: View, at: Point): Point {
  return { x: (at.x - view.x) / view.scale, y: (at.y - view.y) / view.scale };
}

/** Where a line from a box's middle toward `toward` leaves the box. */
function boxEdge(corner: Point, toward: Point, room = 6): Point {
  const cx = corner.x + NODE_W / 2;
  const cy = corner.y + NODE_H / 2;
  const dx = toward.x - cx;
  const dy = toward.y - cy;
  if (dx === 0 && dy === 0) return { x: cx, y: cy };
  const sx = (NODE_W / 2 + room) / Math.abs(dx || 1e-9);
  const sy = (NODE_H / 2 + room) / Math.abs(dy || 1e-9);
  const s = Math.min(sx, sy);
  return { x: cx + dx * s, y: cy + dy * s };
}

/**
 * An edge's curve from one box to another: a gentle arc, bent to one side
 * when the two talk both ways (so the two edges don't sit on each other),
 * leaving and reaching each box at its border. `mid` is where its label
 * sits.
 */
export function edgeCurve(from: Point, to: Point, bothWays: boolean): { d: string; mid: Point; end: Point; start: Point } {
  const a = { x: from.x + NODE_W / 2, y: from.y + NODE_H / 2 };
  const b = { x: to.x + NODE_W / 2, y: to.y + NODE_H / 2 };
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = Math.hypot(dx, dy) || 1;
  const bend = bothWays ? 36 : Math.min(28, length * 0.08);
  // To the right of the travel: each direction of a pair bends its own way.
  const control = { x: (a.x + b.x) / 2 - (dy / length) * bend, y: (a.y + b.y) / 2 + (dx / length) * bend };
  const start = boxEdge(from, control);
  const end = boxEdge(to, control, 10);
  const mid = { x: 0.25 * start.x + 0.5 * control.x + 0.25 * end.x, y: 0.25 * start.y + 0.5 * control.y + 0.25 * end.y };
  const r = (n: number) => Math.round(n * 10) / 10;
  return { d: `M ${r(start.x)} ${r(start.y)} Q ${r(control.x)} ${r(control.y)} ${r(end.x)} ${r(end.y)}`, mid, end, start };
}

/** The node the arrow keys move to from `from`: the nearest one whose
 * middle lies that way (within a 45° cone), else none. */
export function neighbour(points: ReadonlyMap<string, Point>, from: string, key: "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"): string | null {
  const here = points.get(from);
  if (!here) return null;
  const [ux, uy] = key === "ArrowUp" ? [0, -1] : key === "ArrowDown" ? [0, 1] : key === "ArrowLeft" ? [-1, 0] : [1, 0];
  let best: { id: string; score: number } | null = null;
  for (const [id, at] of points) {
    if (id === from) continue;
    const dx = at.x - here.x;
    const dy = at.y - here.y;
    const along = dx * ux + dy * uy;
    const across = Math.abs(dx * uy - dy * ux);
    if (along <= 0 || across > along) continue;
    const score = along + across * 2;
    if (!best || score < best.score) best = { id, score };
  }
  return best?.id ?? null;
}

/**
 * Semantic zoom (the 0.3 review: at fit a card's words were 9–10 px).
 * From 100 % up a node is its full card, drawn at the canvas's scale, so
 * nothing in it is under 12 px. Below, it keeps its box (its place, its
 * edges, Tidy up and Fit are the same) but draws its words at screen
 * size, as many lines as its box has room for: three (name, account and
 * tool, state and task), two (name, state) or one (name, the state its
 * dot).
 */
export const FULL_SCALE = 1;
export type NodeForm = { full: true } | { full: false; lines: 1 | 2 | 3; width: number; height: number };

export function nodeForm(scale: number): NodeForm {
  // A hair under 100 % (a zoom's rounding) is still 100 %.
  if (scale >= FULL_SCALE - 0.005) return { full: true };
  const width = NODE_W * scale;
  const height = NODE_H * scale;
  return { full: false, lines: height >= 84 ? 3 : height >= 50 ? 2 : 1, width, height };
}
