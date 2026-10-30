/**
 * A pool's canvas (redesign §5): its members as nodes, who talks to whom
 * as edges, what waits on you on what it concerns — watched and changed
 * in one place, like a diagram board.
 *
 * - Pan by dragging the background or scrolling; zoom with ⌘/Ctrl-scroll
 *   or a pinch, the buttons, or + and −; Fit shows everything.
 * - Drag a node to move it (its place is kept); drag a session from the
 *   palette to add it; drag a node onto the palette to take it out (Undo
 *   puts it back).
 * - Keyboard: Tab reaches the nodes (one stop), the arrows move between
 *   them, Option+arrows move one, Enter opens it in the inspector, Delete
 *   takes it out, Shift+F10 opens its menu, Escape lets go.
 * - Semantic zoom (0.3 polish): from 100 % up each node is its full card;
 *   below, the same box draws its words at screen size (CompactNode) and
 *   what waits on an edge is a badge that opens its answer. The places,
 *   Tidy up, Fit and the keys are the same in both.
 */
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type JSX, type KeyboardEvent, type PointerEvent } from "react";

import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";
import type { PoolInfo } from "../../lib/ipc/pools";
import { CanvasEdges } from "./CanvasEdges";
import { CanvasToolbar } from "./CanvasToolbar";
import { CanvasNode } from "./CanvasNode";
import { NodeMenu } from "./NodeMenu";
import { Palette } from "./Palette";
import { cannotRemove, nameOf, removeFromPool } from "./actions";
import { fitView, neighbour, NODE_H, NODE_W, toCanvas, zoomAt, type Point, type View } from "./layout";
import type { CanvasData, MemberNode } from "./useCanvas";

export type Selection = { kind: "none" } | { kind: "pool" } | { kind: "node"; id: string } | { kind: "edge"; key: string };

type Drag =
  | { kind: "node"; id: string; start: Point; origin: Point; moved: boolean }
  | { kind: "session"; session: InstanceInfo; start: Point; moved: boolean }
  | { kind: "pan"; start: Point; view: View; moved: boolean };

/** Each pool's view while the app runs (back to a pool: where you left it). */
const views = new Map<string, View>();
/** Tests: every pool back to its first view. */
export const forgetCanvasViews = () => views.clear();
const NUDGE = 16;

function useReducedMotion(): boolean {
  const query = typeof window !== "undefined" && window.matchMedia ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
  const [reduced, setReduced] = useState(query?.matches ?? false);
  useEffect(() => {
    if (!query) return;
    const change = () => setReduced(query.matches);
    query.addEventListener?.("change", change);
    return () => query.removeEventListener?.("change", change);
  }, [query]);
  return reduced;
}

const inside = (box: DOMRect | undefined, at: Point) => !!box && at.x >= box.left && at.x <= box.right && at.y >= box.top && at.y <= box.bottom;

export function Canvas({
  pool,
  data,
  candidates,
  projects,
  selection,
  onSelect,
  onAdd,
}: {
  pool: PoolInfo;
  data: CanvasData;
  candidates: InstanceInfo[];
  projects: ProjectInfo[];
  selection: Selection;
  onSelect: (selection: Selection, focusInspector?: boolean) => void;
  /** Puts a session in the pool, at a canvas point when it was dropped. */
  onAdd: (session: InstanceInfo, at: Point | null) => void;
}): JSX.Element {
  const { members, points, canvas, place, pulses } = data;
  const viewport = useRef<HTMLDivElement | null>(null);
  const palette = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [view, setViewState] = useState<View | null>(views.get(pool.id) ?? null);
  const setView = useCallback(
    (next: View) => {
      views.set(pool.id, next);
      setViewState(next);
    },
    [pool.id],
  );
  const [drag, setDrag] = useState<Drag | null>(null);
  const [dragAt, setDragAt] = useState<Point | null>(null);
  const [overPalette, setOverPalette] = useState(false);
  const [menu, setMenu] = useState<{ id: string; at: Point } | null>(null);
  const [tabStop, setTabStop] = useState<string | null>(null);
  // The palette starts open where there's room for it beside the nodes,
  // or when the pool is empty (it's how one starts).
  const [paletteOpen, setPaletteOpen] = useState<boolean | null>(null);
  const justDragged = useRef(false);
  const reducedMotion = useReducedMotion();

  useLayoutEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const measure = () => setSize({ w: element.clientWidth, h: element.clientHeight });
    measure();
    const observer = typeof ResizeObserver !== "undefined" ? new ResizeObserver(measure) : null;
    observer?.observe(element);
    return () => observer?.disconnect();
  }, []);
  useEffect(() => {
    if (paletteOpen === null && size.w > 0 && canvas && !data.list.loading) setPaletteOpen(size.w >= 900 || members.length === 0);
  }, [paletteOpen, size.w, canvas, members.length, data.list.loading]);
  // The palette sits over the top left: fit what's beside it (or below
  // its folded line).
  const fit = useCallback(
    () => setView(fitView(points.values(), size.w, size.h, 48, paletteOpen ? 260 : 0, paletteOpen ? 0 : 44)),
    [points, size, setView, paletteOpen],
  );
  // The first time this pool's nodes are known: everything in view.
  useEffect(() => {
    if (view === null && size.w > 0 && members.length > 0 && canvas && paletteOpen !== null) fit();
  }, [view, size.w, members.length, canvas, fit, paletteOpen]);
  const shown = view ?? { x: size.w / 2 - NODE_W / 2, y: 48, scale: 1 };

  // Scroll pans; ⌘/Ctrl-scroll (and a trackpad's pinch) zooms at the pointer.
  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const onWheel = (event: WheelEvent) => {
      if ((event.target as HTMLElement).closest("[data-canvas-palette], [data-canvas-scroll]")) return;
      event.preventDefault();
      const box = element.getBoundingClientRect();
      const current = views.get(pool.id) ?? shown;
      if (event.ctrlKey || event.metaKey) setView(zoomAt(current, Math.exp(-event.deltaY * 0.01), { x: event.clientX - box.left, y: event.clientY - box.top }));
      else setView({ ...current, x: current.x - event.deltaX, y: current.y - event.deltaY });
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, [pool.id, shown, setView]);

  const byId = useMemo(() => new Map(members.map((member) => [member.id, member])), [members]);
  const names = useMemo(() => new Map(members.map((member) => [member.id, nameOf(member.session)])), [members]);
  const drawn = useMemo(() => {
    if (drag?.kind !== "node" || !dragAt) return points;
    const next = new Map(points);
    next.set(drag.id, dragAt);
    return next;
  }, [points, drag, dragAt]);

  const remove = useCallback((member: MemberNode) => void removeFromPool(pool, member, points.get(member.id), place), [pool, points, place]);

  // One drag at a time, followed on the window (it may leave the canvas).
  useEffect(() => {
    if (!drag) return;
    const move = (event: globalThis.PointerEvent) => {
      const at = { x: event.clientX, y: event.clientY };
      const far = Math.hypot(at.x - drag.start.x, at.y - drag.start.y) > 4;
      if (far && !drag.moved) setDrag({ ...drag, moved: true });
      if (drag.kind === "pan") {
        setView({ ...drag.view, x: drag.view.x + at.x - drag.start.x, y: drag.view.y + at.y - drag.start.y });
        return;
      }
      setOverPalette(inside(palette.current?.getBoundingClientRect(), at));
      if (drag.kind === "node") setDragAt({ x: drag.origin.x + (at.x - drag.start.x) / shown.scale, y: drag.origin.y + (at.y - drag.start.y) / shown.scale });
      else setDragAt(at);
    };
    const up = (event: globalThis.PointerEvent) => {
      const at = { x: event.clientX, y: event.clientY };
      const moved = drag.moved || Math.hypot(at.x - drag.start.x, at.y - drag.start.y) > 4;
      const onPalette = inside(palette.current?.getBoundingClientRect(), at);
      const box = viewport.current?.getBoundingClientRect();
      if (drag.kind === "pan" && !moved) onSelect({ kind: "none" });
      if (drag.kind === "node" && moved) {
        justDragged.current = true;
        const member = byId.get(drag.id);
        if (onPalette && member) {
          if (!cannotRemove(pool)) remove(member);
        } else {
          const to = { x: drag.origin.x + (at.x - drag.start.x) / shown.scale, y: drag.origin.y + (at.y - drag.start.y) / shown.scale };
          void place([{ session_id: drag.id, x: to.x, y: to.y }]);
        }
      }
      if (drag.kind === "session" && moved && box && inside(box, at) && !onPalette) {
        const to = toCanvas(shown, { x: at.x - box.left, y: at.y - box.top });
        onAdd(drag.session, { x: to.x - NODE_W / 2, y: to.y - NODE_H / 2 });
      }
      setDrag(null);
      setDragAt(null);
      setOverPalette(false);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
    };
  }, [drag, shown, byId, pool, place, remove, setView, onSelect, onAdd]);

  const startNode = useCallback(
    (id: string, event: PointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) return;
      const target = event.target as HTMLElement;
      if ((target.closest("button") && !target.closest("[data-node-name]")) || target.closest("[data-node-popover]")) return;
      const origin = points.get(id);
      if (!origin) return;
      event.stopPropagation();
      setDrag({ kind: "node", id, start: { x: event.clientX, y: event.clientY }, origin, moved: false });
    },
    [points],
  );
  const select = useCallback(
    (id: string, focusInspector: boolean) => {
      if (justDragged.current) {
        justDragged.current = false;
        return;
      }
      setTabStop(id);
      onSelect({ kind: "node", id }, focusInspector);
    },
    [onSelect],
  );
  const focusNode = (id: string) => {
    setTabStop(id);
    viewport.current?.querySelector<HTMLButtonElement>(`[data-node-name="${CSS.escape(id)}"]`)?.focus();
  };
  const onKey = useCallback(
    (id: string, event: KeyboardEvent<HTMLButtonElement>) => {
      const member = byId.get(id);
      if (!member) return;
      const arrow = event.key as "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight";
      if (arrow.startsWith("Arrow") && event.altKey) {
        event.preventDefault();
        const at = points.get(id);
        const [dx, dy] = arrow === "ArrowUp" ? [0, -NUDGE] : arrow === "ArrowDown" ? [0, NUDGE] : arrow === "ArrowLeft" ? [-NUDGE, 0] : [NUDGE, 0];
        if (at) void place([{ session_id: id, x: at.x + dx, y: at.y + dy }]);
      } else if (arrow.startsWith("Arrow")) {
        event.preventDefault();
        const next = neighbour(points, id, arrow);
        if (next) focusNode(next);
      } else if ((event.key === "Delete" || event.key === "Backspace") && !cannotRemove(pool)) {
        event.preventDefault();
        remove(member);
      } else if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
        event.preventDefault();
        const box = event.currentTarget.getBoundingClientRect();
        setMenu({ id, at: { x: box.left, y: box.bottom + 4 } });
      } else if (event.key === "Escape") {
        onSelect({ kind: "none" });
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [byId, points, place, pool, remove, onSelect],
  );
  const onCanvasKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest("input, textarea")) return;
    const middle = { x: size.w / 2, y: size.h / 2 };
    if (event.key === "+" || event.key === "=") setView(zoomAt(shown, 1.2, middle));
    else if (event.key === "-") setView(zoomAt(shown, 1 / 1.2, middle));
    else return;
    event.preventDefault();
  };

  const asksFor = useMemo(() => {
    const out = new Map<string, NonNullable<typeof canvas>["asks"]>();
    for (const ask of canvas?.asks ?? []) if (ask.session_id) out.set(ask.session_id, [...(out.get(ask.session_id) ?? []), ask]);
    return out;
  }, [canvas]);
  const heldFor = useMemo(() => {
    const out = new Map<string, number>();
    for (const edge of canvas?.edges ?? []) {
      const held = edge.pending.filter((p) => p.kind === "hop_limit").length;
      if (held > 0) out.set(edge.to_session_id, (out.get(edge.to_session_id) ?? 0) + held);
    }
    return out;
  }, [canvas]);
  const firstStop = tabStop && byId.has(tabStop) ? tabStop : selection.kind === "node" && byId.has(selection.id) ? selection.id : (members[0]?.id ?? null);
  const dragged = drag?.kind === "node" && drag.moved ? byId.get(drag.id) : undefined;
  const menuMember = menu ? byId.get(menu.id) : undefined;

  return (
    <div
      ref={viewport}
      role="region"
      aria-label={`${pool.name}: canvas`}
      aria-describedby="canvas-help"
      onKeyDown={onCanvasKey}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        const target = event.target as HTMLElement;
        if (target.closest("[data-canvas-palette], [data-canvas-toolbar], [data-node], button, [data-edge]")) return;
        setDrag({ kind: "pan", start: { x: event.clientX, y: event.clientY }, view: shown, moved: false });
      }}
      className={"relative h-full min-h-0 overflow-hidden bg-canvas " + (drag?.kind === "pan" && drag.moved ? "cursor-grabbing" : "")}
      style={{ backgroundImage: "radial-gradient(var(--color-hairline) 1px, transparent 1px)", backgroundSize: `${18 * shown.scale}px ${18 * shown.scale}px`, backgroundPosition: `${shown.x}px ${shown.y}px` }}
      data-testid="pool-canvas"
    >
      <p id="canvas-help" className="sr-only">
        Arrow keys move between members, Option and the arrows move one, Enter opens it, Delete takes it out of the pool, Shift F10 opens its menu.
      </p>
      <div className="absolute left-0 top-0 origin-top-left" style={{ transform: `translate(${shown.x}px, ${shown.y}px) scale(${shown.scale})` }} data-testid="canvas-world">
        <CanvasEdges
          edges={canvas?.edges ?? []}
          points={drawn}
          names={names}
          selected={selection.kind === "edge" ? selection.key : null}
          pulses={pulses}
          reducedMotion={reducedMotion}
          scale={shown.scale}
          onSelect={(key) => onSelect({ kind: "edge", key })}
        />
        {members.map((member) => {
          const at = drawn.get(member.id);
          return at ? (
            <CanvasNode
              key={member.id}
              member={member}
              at={at}
              asks={asksFor.get(member.id) ?? []}
              held={heldFor.get(member.id) ?? 0}
              selected={selection.kind === "node" && selection.id === member.id}
              tabStop={firstStop === member.id}
              dragging={drag?.kind === "node" && drag.id === member.id && drag.moved}
              scale={shown.scale}
              onSelect={select}
              onMenu={(id, point) => setMenu({ id, at: point })}
              onPointerDown={startNode}
              onKey={onKey}
            />
          ) : null;
        })}
      </div>

      {members.length === 0 && canvas && (
        <div className={"pointer-events-none absolute inset-0 flex items-center justify-center p-8" + (paletteOpen ? " pl-[17rem]" : "")} data-testid="canvas-empty">
          <div className="flex max-w-sm flex-col items-center gap-2 rounded-2xl border-2 border-dashed border-hairline bg-surface/80 px-6 py-8 text-center">
            <span className="text-section text-ink">No one in this pool yet</span>
            <span className="text-body text-muted">Drag a session from the left onto the canvas, or press Add beside it.</span>
          </div>
        </div>
      )}

      <div className="pointer-events-none absolute bottom-16 left-3 top-3 flex flex-col">
        <div ref={palette} className="flex min-h-0 flex-col">
        <Palette
          sessions={candidates}
          projects={projects}
          removing={dragged ? { over: overPalette, why: cannotRemove(pool) } : null}
          open={paletteOpen ?? false}
          onOpen={setPaletteOpen}
          onStartDrag={(session, event) => {
            if (event.button !== 0) return;
            setDrag({ kind: "session", session, start: { x: event.clientX, y: event.clientY }, moved: false });
          }}
          onAdd={(session) => onAdd(session, null)}
        />
        </div>
      </div>

      {drag?.kind === "session" && drag.moved && dragAt && (
        <div className="pointer-events-none fixed z-50 flex w-56 items-center gap-2 rounded-xl border border-accent bg-surface px-3 py-2 text-meta text-ink shadow-lg" style={{ left: dragAt.x - 112, top: dragAt.y - 18 }} data-testid="canvas-drag-ghost">
          {nameOf(drag.session)}
          <span className="ml-auto text-label text-muted">{overPalette ? "" : "Drop to add"}</span>
        </div>
      )}

      {data.more > 0 && (
        <span className="absolute right-3 top-3 rounded-full border border-hairline bg-surface px-2.5 py-1 text-label text-muted" data-testid="canvas-more">
          {data.more} more in the List view
        </span>
      )}

      <CanvasToolbar
        scale={shown.scale}
        onZoom={(factor) => setView(zoomAt(shown, factor, { x: size.w / 2, y: size.h / 2 }))}
        onFit={fit}
        onTidy={() => void data.tidy().then(() => (views.delete(pool.id), setViewState(null)))}
      />

      {menu && menuMember && <NodeMenu pool={pool} member={menuMember} members={members} at={menu.at} onClose={() => setMenu(null)} onRemove={remove} />}
    </div>
  );
}
