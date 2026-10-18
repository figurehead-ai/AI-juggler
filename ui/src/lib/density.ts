/**
 * Density modes + the stale-data rule (AJ-E403). UI-state, not display
 * mapping: persistence, migration from the legacy collapse boolean, the
 * keyboard cycle, and the 2×-interval staleness threshold.
 */

export type Density = "comfortable" | "compact" | "minimal";

export const DENSITY_KEY = "aij.density";
/** Legacy boolean from the pre-E403 rail collapse — migrated, never kept. */
const LEGACY_COLLAPSED_KEY = "aij.rail.collapsed";

export const DENSITY_ORDER: Density[] = ["comfortable", "compact", "minimal"];

export function isValidDensity(value: string): value is Density {
  return (DENSITY_ORDER as string[]).includes(value);
}

/**
 * Reads the persisted density. The legacy `aij.rail.collapsed` boolean
 * migrates into `aij.density=minimal` on first read and the old key is
 * removed — never two competing keys.
 */
export function readDensity(storage: Storage = localStorage): Density {
  const legacy = storage.getItem(LEGACY_COLLAPSED_KEY);
  if (legacy !== null) {
    storage.removeItem(LEGACY_COLLAPSED_KEY);
    if (legacy === "1") {
      storage.setItem(DENSITY_KEY, "minimal");
      return "minimal";
    }
  }
  const raw = storage.getItem(DENSITY_KEY);
  return raw !== null && isValidDensity(raw) ? raw : "comfortable";
}

export function storeDensity(density: Density, storage: Storage = localStorage): void {
  storage.setItem(DENSITY_KEY, density);
}

export function nextDensity(current: Density): Density {
  const index = DENSITY_ORDER.indexOf(current);
  return DENSITY_ORDER[(index + 1) % DENSITY_ORDER.length]!;
}

/**
 * The stale-data rule (Req.5): a value older than 2× its refresh
 * interval is stale — it must render with a `stale - Nm` caption, never
 * as fresh. Unit: integer elapsed minutes (floor, minimum 1).
 */
export function staleMinutes(
  sampledAtMs: number,
  intervalMs: number,
  nowMs: number = Date.now(),
): number | null {
  const age = nowMs - sampledAtMs;
  if (age <= 2 * intervalMs) return null;
  return Math.max(1, Math.floor(age / 60_000));
}

/** Stale caption text (`stale - 2m`) or `null` while fresh. */
export function staleCaption(
  sampledAtMs: number,
  intervalMs: number,
  nowMs: number = Date.now(),
): string | null {
  const minutes = staleMinutes(sampledAtMs, intervalMs, nowMs);
  return minutes === null ? null : `stale - ${minutes}m`;
}

/** Refresh intervals (ms) that drive staleness (resource tick 5 s, quota 30 s). */
export const REFRESH_INTERVALS = {
  resources: 5_000,
  quota: 30_000,
} as const;

/**
 * The density-cycle shortcut predicate (Req.4): Ctrl+D works everywhere;
 * Cmd+D only inside the Tauri app — the web preview must keep the
 * browser's bookmark shortcut. Modifiers beyond the shortcut (shift,
 * alt) disqualify.
 */
export function isDensityShortcut(
  event: { key: string; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean },
  inApp: boolean,
): boolean {
  return (
    event.key.toLowerCase() === "d" &&
    !event.shiftKey &&
    !event.altKey &&
    (event.ctrlKey || (event.metaKey && inApp))
  );
}
