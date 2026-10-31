/**
 * How the UI reaches the backend: Tauri IPC inside the app, or — in a
 * Vite dev page opened with `?devBridge=<port>&token=<…>` — the read-only
 * test bridge of a running AI Juggler Dev build (docs/qa/DEV-BRIDGE.md).
 * With neither, `liveBackend` is false and lib/bridge.ts serves the
 * design-preview mocks.
 *
 * Production bundles carry no bridge: `import.meta.env.DEV` is `false`
 * there, so the block that reads the URL and imports ./devBridge is dead
 * code and goes (a test builds the bundle and checks).
 */
import { invoke as tauriInvoke, type InvokeArgs } from "@tauri-apps/api/core";
import {
  listen as tauriListen,
  type EventCallback,
  type UnlistenFn,
} from "@tauri-apps/api/event";
import {
  open as tauriOpenDialog,
  save as tauriSaveDialog,
} from "@tauri-apps/plugin-dialog";
import { timed } from "./timing";

/** What a non-Tauri transport provides. */
export type Transport = {
  invoke<T>(cmd: string, args?: InvokeArgs): Promise<T>;
  listen<T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn>;
  /** The error a write meets: in `errorText`'s shape, said in the page. */
  refuse(what: string): unknown;
};

const tauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let readOnly: Promise<Transport> | null = null;
if (import.meta.env.DEV && !tauri && typeof window !== "undefined") {
  const page = new URL(window.location.href);
  if (page.searchParams.has("devBridge")) {
    readOnly = import("./devBridge").then((bridge) => bridge.connect(page));
  }
}

/** A live backend answers: the app, or the read-only test bridge. */
export const liveBackend = tauri || readOnly !== null;

/** True when the backend is the read-only test bridge: writes are off. */
export const readOnlyBackend = readOnly !== null;

/** Every call is timed (0.3.2, `./timing`): names and times, never data. */
export function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  return timed(cmd, () =>
    readOnly
      ? readOnly.then((transport) => transport.invoke<T>(cmd, args))
      : tauriInvoke<T>(cmd, args),
  );
}

export function listen<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  return readOnly
    ? readOnly.then((transport) => transport.listen<T>(event, handler))
    : tauriListen<T>(event, handler);
}

/** The native file pickers start a write (export, restore, import): the
 * bridge refuses them before anything opens. */
export const openDialog: typeof tauriOpenDialog = (options) =>
  readOnly
    ? readOnly.then((transport) => Promise.reject(transport.refuse("open dialog")))
    : tauriOpenDialog(options);

export const saveDialog: typeof tauriSaveDialog = (options) =>
  readOnly
    ? readOnly.then((transport) => Promise.reject(transport.refuse("save dialog")))
    : tauriSaveDialog(options);
