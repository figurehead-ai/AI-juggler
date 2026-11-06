/**
 * Helpers the design preview's mock calls share.
 */


/** AJ-E1503 seam: `__AIJ_LOAD_HANG__` keeps the first lists loading, so
 * the captures can show the placeholders. */
export function loadHang(): Promise<never> | null {
  return (window as unknown as { __AIJ_LOAD_HANG__?: boolean }).__AIJ_LOAD_HANG__ ? new Promise<never>(() => {}) : null;
}
