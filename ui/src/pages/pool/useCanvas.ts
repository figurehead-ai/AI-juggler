/**
 * A pool canvas's data, kept live without polling storms: its members a
 * page at a time (up to {@link CANVAS_NODES}; the rest are in View as
 * list), each shown with its live row from the rail when it has one; the
 * canvas read for those members, read again when messages, tasks or
 * sessions change (the backend's events, a burst read once); and where
 * the user put each node, changed at once and saved behind it.
 *
 * An edge whose newest activity moved since the last read pulses once.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { errorText, onSessionsUpdated, type InstanceInfo } from "../../lib/bridge";
import { onMessagesChanged } from "../../lib/ipc/messages";
import { onTasksChanged } from "../../lib/ipc/tasks";
import { poolCanvas, poolCanvasPlace, poolCanvasTidy, type NodePositionInfo, type PoolCanvasInfo } from "../../lib/ipc/pool-canvas";
import type { ContextCard } from "../../lib/ipc/pools";
import { usePoolMembers } from "../../lib/pools";
import { showToast } from "../../components/Toast";
import { layout, type Point } from "./layout";

/** The most members drawn on one canvas (smooth at that many). */
export const CANVAS_NODES = 60;
/** A burst of changes is read once, this long after it starts. */
const SETTLE_MS = 300;
/** Its members' cards (what each says it's doing) are read again this
 * often while the window is in view: they change on the read model's
 * tick, not by an event. */
const CARDS_EVERY_MS = 15_000;
/** How long an edge's pulse shows. */
export const PULSE_MS = 1_600;

export type MemberNode = { id: string; session: InstanceInfo; card: ContextCard; role: string };

export const edgeKey = (from: string, to: string) => `${from}>${to}`;

export function useCanvas(poolId: string, sessions: InstanceInfo[]) {
  const list = usePoolMembers(poolId);
  const { items, hasMore, loadingMore, loading, loadMore, refresh: refreshMembers } = list;
  // Up to the canvas's cap, a page at a time.
  useEffect(() => {
    if (hasMore && !loadingMore && !loading && items.length < CANVAS_NODES) loadMore();
  }, [hasMore, loadingMore, loading, items.length, loadMore]);
  const live = useMemo(() => new Map(sessions.map((session) => [session.id, session])), [sessions]);
  const members: MemberNode[] = useMemo(
    () =>
      items.slice(0, CANVAS_NODES).map((row) => ({
        id: row.session.id,
        session: live.get(row.session.id) ?? row.session,
        card: row.card,
        role: row.card.role,
      })),
    [items, live],
  );
  const ids = useMemo(() => members.map((member) => member.id), [members]);
  const idsKey = ids.join("|");

  const [canvas, setCanvas] = useState<PoolCanvasInfo | null>(null);
  const [pulses, setPulses] = useState<Map<string, number>>(new Map());
  const lastSeen = useRef<Map<string, number> | null>(null);
  const read = useCallback(async () => {
    if (ids.length === 0) {
      setCanvas((was) => (was && was.pool_id === poolId && was.edges.length === 0 ? was : { pool_id: poolId, positions: [], edges: [], asks: [], stream: [], since_ms: Date.now() }));
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
      const nodes = first ? [...points].map(([id, at]) => moved.find((row) => row.session_id === id) ?? { session_id: id, x: at.x, y: at.y }) : moved;
      setMine((was) => {
        const next = new Map(was);
        for (const node of nodes) next.set(node.session_id, { x: node.x, y: node.y });
        return next;
      });
      try {
        await poolCanvasPlace(poolId, nodes);
      } catch (cause) {
        showToast({ message: "Couldn't keep that place", detail: errorText(cause), tone: "error" });
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
  }, [poolId, saved]);

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
