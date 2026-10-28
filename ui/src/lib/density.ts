/**
 * Density modes + the stale-data rule (AJ-E403). UI-state, not display
 * mapping: persistence, migration from the legacy collapse boolean, the
 * keyboard cycle, and the 2×-interval staleness threshold.
 */

export type Density = "comfortable" | "compact" | "minimal";

export const DENSITY_KEY = "aij.density";
/** D-25: dispatched when Settings changes the density (App follows it). */
export const DENSITY_EVENT = "aij:density-changed";
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

/** The density the rail opens to again after ⌘B folded it: the last
 * Comfortable or Compact chosen (AJ-E403: ⌘B never resets Compact). */
export const OPEN_DENSITY_KEY = "aij.density.open";

export function storeDensity(density: Density, storage: Storage = localStorage): void {
  storage.setItem(DENSITY_KEY, density);
  if (density !== "minimal") storage.setItem(OPEN_DENSITY_KEY, density);
}

/** The open density ⌘B comes back to (Comfortable until one is chosen). */
export function readOpenDensity(storage: Storage = localStorage): Exclude<Density, "minimal"> {
  const raw = storage.getItem(OPEN_DENSITY_KEY);
  return raw === "compact" ? "compact" : "comfortable";
}

/** ⌘B: fold to the icon column, or open to the density it had. */
export function toggledDensity(current: Density, storage: Storage = localStorage): Density {
  if (current === "minimal") return readOpenDensity(storage);
  // Remember what it folds from (a density chosen before this was kept).
  storage.setItem(OPEN_DENSITY_KEY, current);
  return "minimal";
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
 * Where a key is someone's typing, not a shortcut: a text field, an
 * editable element, or the terminal (xterm), where Ctrl+D is end-of-input.
 */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target.closest("input, textarea, select, [contenteditable=''], [contenteditable='true'], .xterm")) return true;
  return target instanceof HTMLElement && target.isContentEditable === true;
}

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
