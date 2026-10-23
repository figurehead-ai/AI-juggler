/**
 * AJ-E1506 — the terminal tools that need the Mac, over IPC: open a link
 * the terminal printed, and a terminal's own window. In the design
 * preview, opening a link does nothing and the windows are pretend ones
 * (the main window's "in its own window" state still shows).
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";

/** The backend's event: the ids of the terminals in their own windows. */
const WINDOWS_EVENT = "terminal://windows";
/** A terminal window's label: this, then the terminal's id. */
const LABEL_PREFIX = "terminal-";

const preview = { windows: new Set<string>(), listeners: new Set<(ids: string[]) => void>() };
const tellPreview = () => preview.listeners.forEach((handler) => handler([...preview.windows]));

/** Opens a link the terminal printed (the backend decides what it may open). */
export async function terminalOpenLink(id: string, target: string): Promise<void> {
  if (!hasLiveBackend()) {
    // The design preview's seam: the tests read what a click would open.
    (window as unknown as { __AIJ_OPENED_LINK__?: string }).__AIJ_OPENED_LINK__ = target;
    return;
  }
  return invoke<void>("terminal_open_link", { id, target });
}

/** Opens the terminal in its own window, or brings that window forward. */
export async function terminalPopOut(id: string, title: string): Promise<void> {
  if (!hasLiveBackend()) {
    preview.windows.add(id);
    tellPreview();
    return;
  }
  return invoke<void>("terminal_pop_out", { id, title });
}

/** Closes the terminal's own window: it shows here again. */
export async function terminalPopIn(id: string): Promise<void> {
  if (!hasLiveBackend()) {
    preview.windows.delete(id);
    tellPreview();
    return;
  }
  return invoke<void>("terminal_pop_in", { id });
}

/** Follows which terminals are in their own windows; the first answer is now. */
export function followTerminalWindows(handler: (ids: string[]) => void): () => void {
  if (!hasLiveBackend()) {
    preview.listeners.add(handler);
    handler([...preview.windows]);
    return () => void preview.listeners.delete(handler);
  }
  let stopped = false;
  let unlisten: (() => void) | null = null;
  void listen<string[]>(WINDOWS_EVENT, (event) => handler(event.payload)).then((stop) => (stopped ? stop() : (unlisten = stop)));
  void invoke<string[]>("terminal_windows").then((ids) => !stopped && handler(ids), () => null);
  return () => {
    stopped = true;
    unlisten?.();
  };
}

/** In a terminal's own window: that terminal's id (from the window's label). */
export function ownTerminalWindow(): string | null {
  const internals = (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } }).__TAURI_INTERNALS__;
  const label = internals?.metadata?.currentWindow?.label ?? "";
  return label.startsWith(LABEL_PREFIX) ? label.slice(LABEL_PREFIX.length) : null;
}
