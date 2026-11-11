/**
 * This window's own storage, kept per data folder (rc32). The webview's
 * storage is one per app build, whatever HOME it runs on: a lab's Home
 * said "Kimi Code is installed" from another HOME's scan, and its Account
 * pages opened on the tab another lab left. The backend names this data
 * folder's scope before any script runs (`window.__AIJ_STORE_SCOPE__`,
 * `lab_scope::store_scope`): empty on the user's own home, whose keys
 * stay as they always were. Every read and write is guarded: storage may
 * be unavailable, and then nothing is remembered.
 */

/** `key` as this data folder keeps it. */
export function scopedKey(key: string): string {
  const scope: unknown = Reflect.get(window, "__AIJ_STORE_SCOPE__");
  return typeof scope === "string" && scope !== "" ? `${key}@${scope}` : key;
}

/** What this data folder keeps under `key` (`null`: nothing, or no storage). */
export function readKept(key: string): string | null {
  try {
    return localStorage.getItem(scopedKey(key));
  } catch {
    return null;
  }
}

/** Keeps `value` under `key` for this data folder (nothing without storage). */
export function keep(key: string, value: string): void {
  try {
    localStorage.setItem(scopedKey(key), value);
  } catch {
    // No storage: this visit only.
  }
}
