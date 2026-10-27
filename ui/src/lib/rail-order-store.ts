/**
 * D-47 — the one store of Open now's order (`rail-order.ts` has the rules):
 * the chosen order, the user's arrangement of rows and places, folded
 * places and the pins of sessions other apps run.
 *
 * Kept in AI Juggler's settings (`rail_order`, one small JSON value), so a
 * restart and a backup keep it, like pins and labels (AJ-E1502 Req.4). A
 * copy in this window's storage paints the first frame at once (the
 * settings read is a call away); the settings value wins once read. A
 * value that can't be read is never written over by the rail's own
 * bookkeeping, only by something the user does.
 */
import { useSyncExternalStore } from "react";
import { appSettingGet, appSettingSet } from "./bridge";
import { EMPTY_ORDER, foldKey, moveIn, parseOrder, reconcile, resetOrder, type OrderLens, type OrderMode, type Present, type RailOrder } from "./rail-order";

export const ORDER_KEY = "rail_order";
const CACHE_KEY = "aij.rail.order.v1";
const WRITE_DELAY_MS = 250;

let current: RailOrder | null = null;
let loaded = false;
let started = false;
let readFailed = false;
let timer: number | null = null;
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of listeners) listener();
}

function cached(): RailOrder {
  try {
    return parseOrder(localStorage.getItem(CACHE_KEY));
  } catch {
    return EMPTY_ORDER;
  }
}

function load(): void {
  if (started) return;
  started = true;
  void appSettingGet(ORDER_KEY).then(
    (raw) => {
      // Nothing kept yet: this window's copy (or the defaults) stands.
      if (raw !== null) current = parseOrder(raw);
      loaded = true;
      emit();
    },
    () => {
      readFailed = true;
      loaded = true;
      emit();
    },
  );
}

/** The record now (the cached copy until the settings value is read). */
export function railOrder(): RailOrder {
  load();
  current ??= cached();
  return current;
}

function write(): void {
  if (timer !== null) window.clearTimeout(timer);
  timer = window.setTimeout(() => {
    timer = null;
    const value = JSON.stringify(railOrder());
    try {
      localStorage.setItem(CACHE_KEY, value);
    } catch {
      // This window's copy only speeds up the first frame.
    }
    void appSettingSet(ORDER_KEY, value).catch(() => undefined);
  }, WRITE_DELAY_MS);
}

/** Changes the record (something the user did): kept, and drawn. */
export function updateOrder(change: (order: RailOrder) => RailOrder): void {
  current = change(railOrder());
  readFailed = false;
  write();
  emit();
}

/** The rail's bookkeeping (new rows and places at the end, gone ones
 * pruned), once the kept record is read. */
export function reconcileOrder(present: Present): void {
  if (!loaded) return;
  const { order, changed } = reconcile(railOrder(), present);
  if (!changed) return;
  current = order;
  if (!readFailed) write();
  emit();
}

export function useRailOrder(): RailOrder {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    railOrder,
    railOrder,
  );
}

export function setOrderMode(mode: OrderMode): void {
  if (railOrder().mode !== mode) updateOrder((order) => ({ ...order, mode }));
}

/** Pins or unpins a session another app runs (by its rail key). */
export function togglePin(key: string): void {
  updateOrder((order) => ({ ...order, pins: order.pins.includes(key) ? order.pins.filter((pin) => pin !== key) : [...order.pins, key] }));
}

/** Moves a row within its group: `shown` is the group's rows in order. */
export function moveRow(shown: readonly string[], from: number, to: number): void {
  updateOrder((order) => ({ ...order, rows: moveIn(order.rows, shown, from, to) }));
}

/** Moves a group (a place) among the lens's groups, in order. */
export function moveGroup(lens: OrderLens, shown: readonly string[], from: number, to: number): void {
  updateOrder((order) => ({ ...order, groups: { ...order.groups, [lens]: moveIn(order.groups[lens], shown, from, to) } }));
}

/** Folds or unfolds a group, remembered. */
export function toggleFold(lens: OrderLens, group: string): void {
  const key = foldKey(lens, group);
  updateOrder((order) => ({ ...order, folded: order.folded.includes(key) ? order.folded.filter((entry) => entry !== key) : [...order.folded, key] }));
}

/** Reset my order; returns what to put back for Undo. */
export function resetMyOrder(): RailOrder {
  const before = railOrder();
  updateOrder(resetOrder);
  return before;
}

/** Puts a whole record back (Undo). */
export function restoreOrder(order: RailOrder): void {
  updateOrder(() => order);
}

/** Forget the cached record (tests, a storage reset). */
export function resetRailOrderCache(): void {
  if (timer !== null) window.clearTimeout(timer);
  timer = null;
  current = null;
  loaded = false;
  started = false;
  readFailed = false;
}
