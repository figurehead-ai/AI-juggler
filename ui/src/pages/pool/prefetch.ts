/**
 * X2 (D-62.20): a pool of 50 opens at once. Its canvas needs its members'
 * first page and then the canvas read for the ones it draws: two reads,
 * one after the other. On rc26 they were read only once the page opened,
 * so a 50-member pool took 120–140 ms from the click to its canvas.
 *
 * Pointing at a pool's card (hover or focus), or its card showing on the
 * Pools screen, reads both ahead (`prefetchPool`, in useCanvas); the page
 * then paints from them at once and reads them again behind (stale while
 * it revalidates). Here: the canvas reads, kept for the pools opened most
 * recently.
 */
import type { PoolCanvasInfo } from "../../lib/ipc/pool-canvas";

/** Canvas reads kept, by pool. */
const KEPT = 20;
const canvases = new Map<string, PoolCanvasInfo>();

/** The canvas read last for `poolId`, if one was. */
export function rememberedCanvas(poolId: string): PoolCanvasInfo | null {
  return canvases.get(poolId) ?? null;
}

/** Keeps `canvas` as its pool's newest read. */
export function rememberCanvas(canvas: PoolCanvasInfo): void {
  canvases.delete(canvas.pool_id);
  canvases.set(canvas.pool_id, canvas);
  for (const oldest of canvases.keys()) {
    if (canvases.size <= KEPT) break;
    canvases.delete(oldest);
  }
}

/** Forgets every canvas read (tests). */
export function forgetCanvases(): void {
  canvases.clear();
}
