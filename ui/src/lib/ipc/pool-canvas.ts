/**
 * 0.3 (redesign §5) — a pool's canvas: where you put each member, who
 * talks to whom, what waits on you and the pool's newest activity (one
 * IPC module per feature, ENGINEERING-GUIDE U3). Placing a node and Tidy
 * up change only the canvas, never a pool, a member or a session.
 *
 * Reads name the members the canvas shows (at most 200) and never grow
 * with the history (I6). The Pools screen's cards read their open
 * members' places and tasks in progress in one call (at most 60 pools).
 *
 * The preview draws its edges from its own messages (`previewPoolMessages`)
 * and its asks from its own Needs you, keeps its own positions, and takes
 * extra edges, asks, stream lines and tasks in progress from
 * `__AIJ_POOL_CANVAS__` (by pool id); the calls that change things are
 * recorded in `__AIJ_POOL_CANVAS_CALLS__`.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import { messagesNeeds, previewPoolMessages } from "./messages";
import { tasksNeeds } from "./tasks";
import type { PoolCanvasInfo } from "../../types/generated/PoolCanvasInfo";
import type { CanvasEdgeInfo } from "../../types/generated/CanvasEdgeInfo";
import type { NodeAskInfo } from "../../types/generated/NodeAskInfo";
import type { NodePositionInfo } from "../../types/generated/NodePositionInfo";
import type { PendingInfo } from "../../types/generated/PendingInfo";
import type { PoolPreviewInfo } from "../../types/generated/PoolPreviewInfo";
import type { StreamLineInfo } from "../../types/generated/StreamLineInfo";

export type { PoolCanvasInfo, CanvasEdgeInfo, NodeAskInfo, NodePositionInfo, PendingInfo, PoolPreviewInfo, StreamLineInfo };

/** Edges count the last week (the backend's `EDGE_WINDOW_MS`). */
export const EDGE_WINDOW_MS = 7 * 24 * 60 * 60 * 1000;
/** The most members one canvas read names (the backend's `MOST_NODES`). */
export const MOST_NODES = 200;
/** The most pools one cards read names (`MOST_PREVIEWS`). */
export const MOST_PREVIEWS = 60;

/** What the preview adds to a pool's canvas. */
export type CanvasSeed = Partial<Pick<PoolCanvasInfo, "edges" | "asks" | "stream" | "positions">> & { moving?: number };
interface Seams {
  __AIJ_POOL_CANVAS__?: Record<string, CanvasSeed>;
  __AIJ_POOL_CANVAS_CALLS__?: string[];
}
const seams = () => window as unknown as Seams;
const record = (call: string) => (seams().__AIJ_POOL_CANVAS_CALLS__ ??= []).push(call);

/** The preview's positions, by pool, then session. */
let placed: Map<string, Map<string, NodePositionInfo>> | null = null;
function previewPlaced(poolId: string): Map<string, NodePositionInfo> {
  placed ??= new Map();
  let pool = placed.get(poolId);
  if (!pool) {
    pool = new Map((seams().__AIJ_POOL_CANVAS__?.[poolId]?.positions ?? []).map((row) => [row.session_id, { ...row }]));
    placed.set(poolId, pool);
  }
  return pool;
}

/** The preview's canvas: edges from its messages among the shown members,
 * what waits on them, its asks, and the seed's extras. */
async function preview(poolId: string, shown: string[]): Promise<PoolCanvasInfo> {
  const ids = new Set(shown);
  const seed = seams().__AIJ_POOL_CANVAS__?.[poolId] ?? {};
  const since = Date.now() - EDGE_WINDOW_MS;
  const edges = new Map<string, CanvasEdgeInfo>();
  const edgeOf = (from: string, to: string) => {
    const key = `${from}>${to}`;
    let edge = edges.get(key);
    if (!edge) {
      edge = { from_session_id: from, to_session_id: to, messages: 0, tasks: 0, last_at_ms: 0, thread_id: null, pending: [] };
      edges.set(key, edge);
    }
    return edge;
  };
  const needs = await messagesNeeds();
  const askOf = (message: string) => needs.find((need) => need.message_id === message && need.ask_id);
  for (const message of previewPoolMessages(poolId)) {
    const from = message.from_session_id;
    if (!from || !ids.has(from) || message.from_kind !== "agent") continue;
    for (const to of message.recipients) {
      if (!to.session_id || !ids.has(to.session_id) || to.session_id === from) continue;
      const edge = edgeOf(from, to.session_id);
      if (message.at_ms >= since) edge.messages += 1;
      if (message.at_ms >= edge.last_at_ms) Object.assign(edge, { last_at_ms: message.at_ms, thread_id: message.thread_id });
      const ask = askOf(message.id);
      const kind = ask?.kind === "deliver" ? "hop_limit" : ask && to.state === "held" ? "asked" : to.state === "unread" && to.deliverable ? "waiting" : null;
      if (kind) edge.pending.push({ message_id: message.id, thread_id: message.thread_id, subject: message.subject, recipient: to.id, kind, ask_id: ask?.ask_id ?? null, at_ms: message.at_ms });
    }
  }
  for (const extra of seed.edges ?? []) {
    if (ids.has(extra.from_session_id) && ids.has(extra.to_session_id)) edges.set(`${extra.from_session_id}>${extra.to_session_id}`, { ...extra, pending: [...extra.pending] });
  }
  const asks: NodeAskInfo[] = needs
    .filter((need) => need.pool_id === poolId && need.kind === "ask" && need.ask_id)
    .map((need) => {
      const sender = previewPoolMessages(poolId).find((m) => m.id === need.message_id)?.from_session_id ?? null;
      return { session_id: sender && ids.has(sender) ? sender : null, source: "message", id: need.ask_id!, title: need.title, offer_always: need.offer_always, at_ms: need.at_ms };
    });
  // A seeded task ask stays while its need is open (answered, it goes).
  const open = new Set((await tasksNeeds()).map((need) => need.id));
  asks.push(
    ...(seed.asks ?? [])
      .filter((ask) => ask.source !== "task" || open.has(ask.id))
      .map((ask) => ({ ...ask, session_id: ask.session_id && ids.has(ask.session_id) ? ask.session_id : null })),
  );
  return {
    pool_id: poolId,
    positions: [...previewPlaced(poolId).values()].filter((row) => ids.has(row.session_id)).map((row) => ({ ...row })),
    edges: [...edges.values()].sort((a, b) => `${a.from_session_id}>${a.to_session_id}`.localeCompare(`${b.from_session_id}>${b.to_session_id}`)),
    asks: asks.sort((a, b) => a.at_ms - b.at_ms),
    stream: (seed.stream ?? []).map((line) => ({ ...line })),
    since_ms: since,
  };
}

/** A pool's canvas, for the members it shows. */
export async function poolCanvas(poolId: string, sessionIds: string[]): Promise<PoolCanvasInfo> {
  const shown = sessionIds.slice(0, MOST_NODES);
  if (!hasLiveBackend()) return preview(poolId, shown);
  return invoke<PoolCanvasInfo>("pools_canvas", { poolId, sessionIds: shown });
}

/** Puts members' nodes where you dropped them (all or nothing). */
export async function poolCanvasPlace(poolId: string, nodes: NodePositionInfo[]): Promise<void> {
  const rounded = nodes.map((node) => ({ session_id: node.session_id, x: Math.round(node.x), y: Math.round(node.y) }));
  if (!hasLiveBackend()) {
    record(`place:${poolId}:${rounded.map((n) => `${n.session_id}@${n.x},${n.y}`).join(";")}`);
    const pool = previewPlaced(poolId);
    for (const node of rounded) pool.set(node.session_id, node);
    return;
  }
  return invoke<void>("pools_canvas_place", { poolId, nodes: rounded });
}

/** Forgets where you put a pool's members ("Tidy up"). */
export async function poolCanvasTidy(poolId: string): Promise<void> {
  if (!hasLiveBackend()) {
    record(`tidy:${poolId}`);
    previewPlaced(poolId).clear();
    return;
  }
  return invoke<void>("pools_canvas_tidy", { poolId });
}

/** The Pools screen's cards: each pool's open members' places and its
 * tasks in progress. */
export async function poolCanvasPreviews(poolIds: string[]): Promise<PoolPreviewInfo[]> {
  const asked = poolIds.slice(0, MOST_PREVIEWS);
  if (!hasLiveBackend()) {
    return asked.map((poolId) => ({
      pool_id: poolId,
      positions: [...previewPlaced(poolId).values()].map((row) => ({ ...row })),
      moving: seams().__AIJ_POOL_CANVAS__?.[poolId]?.moving ?? 0,
    }));
  }
  return invoke<PoolPreviewInfo[]>("pools_canvas_previews", { poolIds: asked });
}

/** Tests: forget the preview's positions between cases. */
export function resetPreviewCanvas(): void {
  placed = null;
}
