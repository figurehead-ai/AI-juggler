/**
 * When a terminal may be fitted to its host (the rc31 walk, and rc27
 * behind the switch modal: after switching between sessions a terminal
 * sometimes re-rendered about 12 columns wide until something resized
 * it). A fit measured while the host was hidden or not yet laid out gave
 * those few columns, and the PTY heard them. A fit now runs only on a
 * host that is shown and has its real size, and never resizes to fewer
 * columns than a terminal can use: the host's ResizeObserver and its
 * coming into view fit it again once it has its size.
 */

/** Fewer columns than this is a measure taken mid-layout, never a size
 * to give the PTY (a real terminal pane is far wider). */
export const MIN_FIT_COLS = 20;

/** Whether `host` is shown with a real size now. */
export function hostShown(host: HTMLElement): boolean {
  if (!host.isConnected || host.offsetParent === null) return false;
  if (host.clientWidth <= 0 || host.clientHeight <= 0) return false;
  const check = (host as HTMLElement & { checkVisibility?: (options?: { visibilityProperty?: boolean }) => boolean }).checkVisibility;
  return check ? check.call(host, { visibilityProperty: true }) : true;
}

/** The size to fit `host` to now, from the fit add-on's proposal; `null`
 * when it isn't shown with its real size, or the proposal is a mid-layout
 * one. */
export function fitSize(host: HTMLElement, propose: () => { cols: number; rows: number } | undefined): { cols: number; rows: number } | null {
  if (!hostShown(host)) return null;
  const dims = propose();
  if (!dims || !Number.isFinite(dims.cols) || !Number.isFinite(dims.rows)) return null;
  if (dims.cols < MIN_FIT_COLS || dims.rows <= 0) return null;
  return dims;
}
