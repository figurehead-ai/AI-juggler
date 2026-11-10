/**
 * A pool canvas's data, kept live without polling storms: its members a
 * page at a time (up to {@link CANVAS_NODES}; the rest are in View as
 * list), each shown with its live row from the rail when it has one; the
 * canvas read for those members, read again when messages, tasks or
 * sessions change (the backend's events, a burst read once); and where
 * the user put each node, changed at once and saved behind it. With no
 * member drawn, only the pool's stream is read.
 *
 * An edge whose newest activity moved since the last read pulses once.
 */
import { isActiveStatus } from "../../components/indicator";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { errorText } from "../../lib/bridge";
import { onSessionsUpdated } from "../../lib/ipc/instances";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { onMessagesChanged } from "../../lib/ipc/messages";
import { onTasksChanged } from "../../lib/ipc/tasks";
import { poolCanvas, poolCanvasPlace, poolCanvasTidy, type NodePositionInfo, type PoolCanvasInfo } from "../../lib/ipc/pool-canvas";
import { poolMembers, type ContextCard, type PoolMemberRow } from "../../lib/ipc/pools";
import { POOL_MEMBERS, usePoolMembers } from "../../lib/pools";
import { prefetchPaged, rememberedPage } from "../../lib/paged";
import { showToast } from "../../components/Toast";
import { layout, type Point } from "./layout";
import { nameOf } from "./actions";
import { rememberCanvas, rememberedCanvas } from "./prefetch";

/** The most members drawn on one canvas (smooth at that many). */
export const CANVAS_NODES = 60;
/** Past this many members, broad members' sessions draw as their group. */
export const GROUP_PAST = 50;
/** A burst of changes is read once, this long after it starts. */
const SETTLE_MS = 300;
/** Its members' cards (what each says it's doing) are read again this
 * often while the window is in view: they change on the read model's
 * tick, not by an event. */
const CARDS_EVERY_MS = 15_000;
/** How long an edge's pulse shows. */
export const PULSE_MS = 1_600;

/** `name`: the session's name, told apart from a member of the same name
 * by its start ("web · started 03:07"; the P7 and P11 walks). */
export type MemberNode = { id: string; session: InstanceInfo; card: ContextCard; role: string; name: string };

/** Each name once: where two sessions share one, each says when it started. */
export function distinctNames<T extends { session: InstanceInfo }>(rows: readonly T[]): (T & { name: string })[] {
  const plain = rows.map((row) => nameOf(row.session));
  const twice = new Set(plain.filter((name, at) => plain.indexOf(name) !== at));
  const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return rows.map((row, at) => ({ ...row, name: twice.has(plain[at]!) ? `${plain[at]} · started ${clock(row.session.started_at_ms)}` : plain[at]! }));
}

export const edgeKey = (from: string, to: string) => `${from}>${to}`;

/** Whether two reads of a stream say the same lines. */
const sameStream = (a: PoolCanvasInfo["stream"], b: PoolCanvasInfo["stream"]) => a.length === b.length && a.every((line, at) => line.at_ms === b[at]?.at_ms && line.words === b[at]?.words);

/** The members the canvas draws, as they are now (`live`): the open ones
 * and those named on their own, ended or not. */
export function drawnMembers(items: readonly PoolMemberRow[], live: ReadonlyMap<string, InstanceInfo>, total = items.length): MemberNode[] {
  // AJ-E3008 R2: past GROUP_PAST members, a broad member's sessions are its
  // group node (dashed, "N sessions now · new ones join"), not a node each;
  // only the sessions named on their own, and watched ones, are drawn.
  const grouped = total > GROUP_PAST;
  const drawn = items
    .map((row) => ({ row, session: live.get(row.session.id) ?? row.session }))
    // A session a sealed pool holds out of this one is in the List view only.
    .filter(({ row }) => !row.held_by)
    .filter(({ row }) => !grouped || row.via === "session" || row.via === "watched")
    .filter(({ row, session }) => isActiveStatus(session.status) || row.via === "session" || row.via === "watched")
    .map(({ row, session }) => ({ id: row.session.id, session, card: row.card, role: row.card.role }));
  return distinctNames(drawn);
}

/** Pools whose members and canvas are being read ahead. */
const reading = new Set<string>();

/** Reads `poolId`'s members and its canvas ahead, unless they're kept
 * already or being read. A failure is left to the page's own read. */
export async function prefetchPool(poolId: string): Promise<void> {
  if (reading.has(poolId) || rememberedCanvas(poolId)) return;
  reading.add(poolId);
  try {
    const key = `${POOL_MEMBERS}${poolId}`;
    await prefetchPaged(key, (page) => poolMembers(poolId, page));
    const page = rememberedPage<PoolMemberRow>(key);
    if (!page) return;
    const ids = drawnMembers(page.items, new Map(), page.total ?? page.items.length)
      .slice(0, CANVAS_NODES)
      .map((member) => member.id);
    rememberCanvas(await poolCanvas(poolId, ids));
  } catch {
    // A prefetch is a nicety: the page reads it itself.
  } finally {
    reading.delete(poolId);
  }
}

export function useCanvas(poolId: string, sessions: InstanceInfo[]) {
  const list = usePoolMembers(poolId);
  const { items, hasMore, loadingMore, loading, loadMore, refresh: refreshMembers } = list;
  // Up to the canvas's cap, a page at a time.
  useEffect(() => {
    if (hasMore && !loadingMore && !loading && items.length < CANVAS_NODES) loadMore();
  }, [hasMore, loadingMore, loading, items.length, loadMore]);
  const live = useMemo(() => new Map(sessions.map((session) => [session.id, session])), [sessions]);
  // The canvas draws the open work and the sessions named on their own;
  // an ended session a broad member brought in (an App's pile up as they
  // end) stays in the pool and the List view, counted on the canvas's
  // "more in the List view" (the P3 walk).
  const total = list.total ?? items.length;
  const members: MemberNode[] = useMemo(() => drawnMembers(items, live, total).slice(0, CANVAS_NODES), [items, live, total]);
  const ids = useMemo(() => members.map((member) => member.id), [members]);
  const idsKey = ids.join("|");

  // X2: a read made ahead (`prefetchPool`) paints at once; it's read again
  // behind it.
  const [canvas, setCanvasState] = useState<PoolCanvasInfo | null>(() => rememberedCanvas(poolId));
  const setCanvas = useCallback((next: PoolCanvasInfo | null | ((was: PoolCanvasInfo | null) => PoolCanvasInfo | null)) => {
    setCanvasState((was) => {
      const value = typeof next === "function" ? next(was) : next;
      if (value) rememberCanvas(value);
      return value;
    });
  }, []);
  const [pulses, setPulses] = useState<Map<string, number>>(new Map());
  const lastSeen = useRef<Map<string, number> | null>(null);
  const read = useCallback(async () => {
    if (ids.length === 0) {
      // No member drawn: nothing to place or join, but the pool's stream
      // is still its own (the one who just left says so, P5 step 4).
      setCanvas((was) => (was && was.pool_id === poolId && was.edges.length === 0 ? was : { pool_id: poolId, positions: [], edges: [], asks: [], stream: [], since_ms: Date.now() }));
      try {
        const next = await poolCanvas(poolId, []);
        setCanvas((was) => (was && was.pool_id === poolId && sameStream(was.stream, next.stream) ? was : { ...next, positions: [], edges: [], asks: [] }));
      } catch {
        // Keep what was there.
      }
      return;
    }
    try {
      const next = await poolCanvas(poolId, ids);
      const before = lastSeen.current;
      const now = Date.now();
      const moved = new Map<string, number>();
      for (const edge of next.edges) {
        const key = edgeKey(edge.from_session_id, edge.to_session_id);
        if (before && edge.last_at_ms > (before.get(key) ?? 0)) moved.set(key, now);
      }
      lastSeen.current = new Map(next.edges.map((edge) => [edgeKey(edge.from_session_id, edge.to_session_id), edge.last_at_ms]));
      if (moved.size > 0) setPulses((was) => new Map([...was, ...moved]));
      setCanvas(next);
    } catch {
      // Keep what was there: a canvas a moment old is still true enough.
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [poolId, idsKey]);
  useEffect(() => {
    lastSeen.current = null;
  }, [poolId]);
  useEffect(() => {
    void read();
    let timer: number | undefined;
    const soon = () => {
      if (timer !== undefined) return;
      timer = window.setTimeout(() => {
        timer = undefined;
        void read();
      }, SETTLE_MS);
    };
    const offs = [onMessagesChanged(soon), onTasksChanged(soon), onSessionsUpdated(soon)];
    return () => {
      if (timer !== undefined) window.clearTimeout(timer);
      offs.forEach((off) => off());
    };
  }, [read]);
  // The cards' own words, now and then, only while the window shows.
  useEffect(() => {
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") refreshMembers();
    }, CARDS_EVERY_MS);
    return () => window.clearInterval(timer);
  }, [refreshMembers]);
  // A pulse fades.
  useEffect(() => {
    if (pulses.size === 0) return;
    const timer = window.setTimeout(() => {
      const now = Date.now();
      setPulses((was) => new Map([...was].filter(([, at]) => now - at < PULSE_MS)));
    }, PULSE_MS);
    return () => window.clearTimeout(timer);
  }, [pulses]);

  // Where the user put each node: theirs at once, saved behind.
  const [mine, setMine] = useState<Map<string, Point>>(new Map());
  useEffect(() => setMine(new Map()), [poolId]);
  const saved = useMemo(() => {
    const out = new Map<string, Point>((canvas?.pool_id === poolId ? canvas.positions : []).map((row) => [row.session_id, { x: row.x, y: row.y }]));
    for (const [id, at] of mine) out.set(id, at);
    return out;
  }, [canvas, mine, poolId]);
  const points = useMemo(() => layout(members.map((member) => ({ id: member.id, role: member.role })), saved), [members, saved]);

  /** Saves where nodes are. The first time the user arranges a pool,
   * every shown node is saved where it is, so the rest stay put. */
  const place = useCallback(
    async (moved: NodePositionInfo[]) => {
      const first = saved.size === 0;
      const rest = first ? [...points].filter(([id]) => !moved.some((row) => row.session_id === id)).map(([id, at]) => ({ session_id: id, x: at.x, y: at.y })) : [];
      const nodes = [...moved, ...rest];
      setMine((was) => {
        const next = new Map(was);
        for (const node of nodes) next.set(node.session_id, { x: node.x, y: node.y });
        return next;
      });
      try {
        await poolCanvasPlace(poolId, nodes);
      } catch (cause) {
        showToast({ message: "Couldn't keep it there", detail: errorText(cause), tone: "error" });
        setMine(new Map());
        void read();
      }
    },
    [poolId, points, saved, read],
  );

  /** Tidy up: forgets every place and lays the pool out again; Undo puts
   * them back. */
  const tidy = useCallback(async () => {
    const before = [...saved].map(([id, at]) => ({ session_id: id, x: at.x, y: at.y }));
    try {
      await poolCanvasTidy(poolId);
      setMine(new Map());
      setCanvas((was) => (was ? { ...was, positions: [] } : was));
      if (before.length > 0) {
        showToast({
          message: "Laid out again",
          undo: {
            onUndo: async () => {
              await poolCanvasPlace(poolId, before);
              setMine(new Map(before.map((row) => [row.session_id, { x: row.x, y: row.y }])));
            },
          },
        });
      }
    } catch (cause) {
      showToast({ message: "Couldn't tidy up", detail: errorText(cause), tone: "error" });
    }
  }, [poolId, saved, setCanvas]);

  return {
    list,
    members,
    more: Math.max(0, (list.total ?? items.length) - members.length),
    canvas: canvas?.pool_id === poolId ? canvas : null,
    points,
    arranged: saved.size > 0,
    pulses,
    place,
    tidy,
    reread: read,
  };
}

export type CanvasData = ReturnType<typeof useCanvas>;
