/**
 * AJ-E3011 round 4 (#5): adding a member and changing a role show at once
 * — a pending look on the next frame, well inside 100 ms — and settle when
 * the backend's answer reaches the screen; refused, they roll back and the
 * caller says why (its toast). The walk read Codex's 2–3 s arrival as a
 * failed click.
 *
 * One small store for the pool page: what's being added (by member ref)
 * and what role a member is changing to. Each entry also lapses after a
 * few seconds, so nothing stays pending if the screen never reads it.
 */
import { useMemo, useSyncExternalStore } from "react";

export interface PendingAdd {
  poolId: string;
  /** `session:…`, `app:…`, `account:…`, `tool:…`, `mac:…` or `all`. */
  ref: string;
  label: string;
  /** Its member kind (`session`, `app`…). */
  kind: string;
}

export interface PendingRole {
  poolId: string;
  /** A session's id, or a member ref (`app:…`). */
  ref: string;
  role: string;
}

/** The longest a change stays pending without the screen reading it. */
const LAPSE_MS = 6_000;

let adds: PendingAdd[] = [];
let roles: PendingRole[] = [];
const listeners = new Set<() => void>();
const emit = () => {
  for (const listener of listeners) listener();
};
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

function dropAdd(entry: PendingAdd): void {
  if (!adds.includes(entry)) return;
  adds = adds.filter((one) => one !== entry);
  emit();
}

function dropRole(entry: PendingRole): void {
  if (!roles.includes(entry)) return;
  roles = roles.filter((one) => one !== entry);
  emit();
}

/** Runs `work` with `entry` pending until the screen shows it (see
 * {@link settleAdds}); refused, it's gone at once and the error goes on to
 * the caller, which says why. */
export async function withPendingAdd<T>(entry: PendingAdd, work: () => Promise<T>): Promise<T> {
  const pending = { ...entry };
  adds = [...adds.filter((one) => !(one.poolId === entry.poolId && one.ref === entry.ref)), pending];
  emit();
  try {
    const out = await work();
    setTimeout(() => dropAdd(pending), LAPSE_MS);
    return out;
  } catch (cause) {
    dropAdd(pending);
    throw cause;
  }
}

/** The same for a role: pending until the screen shows the new role. */
export async function withPendingRole<T>(entry: PendingRole, work: () => Promise<T>): Promise<T> {
  const pending = { ...entry };
  roles = [...roles.filter((one) => !(one.poolId === entry.poolId && one.ref === entry.ref)), pending];
  emit();
  try {
    const out = await work();
    setTimeout(() => dropRole(pending), LAPSE_MS);
    return out;
  } catch (cause) {
    dropRole(pending);
    throw cause;
  }
}

/** The screen shows these members now: their adds are settled. */
export function settleAdds(poolId: string, present: ReadonlySet<string>): void {
  // Only a real change replaces the list: the screen reads it by identity.
  const kept = adds.filter((one) => one.poolId !== poolId || !present.has(one.ref));
  if (kept.length === adds.length) return;
  adds = kept;
  emit();
}

/** The screen shows these roles now: their changes are settled. */
export function settleRoles(poolId: string, shown: ReadonlyMap<string, string>): void {
  const kept = roles.filter((one) => one.poolId !== poolId || shown.get(one.ref) !== one.role);
  if (kept.length === roles.length) return;
  roles = kept;
  emit();
}

/** What's being added to `poolId` now. */
export function usePendingAdds(poolId: string): readonly PendingAdd[] {
  const all = useSyncExternalStore(subscribe, () => adds);
  return useMemo(() => all.filter((one) => one.poolId === poolId), [all, poolId]);
}

/** The roles members of `poolId` are changing to, by ref. */
export function usePendingRoles(poolId: string): ReadonlyMap<string, string> {
  const all = useSyncExternalStore(subscribe, () => roles);
  return useMemo(() => new Map(all.filter((one) => one.poolId === poolId).map((one) => [one.ref, one.role])), [all, poolId]);
}

/** Tests: start over. */
export function resetPending(): void {
  adds = [];
  roles = [];
  emit();
}
