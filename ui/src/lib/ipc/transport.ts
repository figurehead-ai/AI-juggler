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
import { Channel, invoke as tauriInvoke, type InvokeArgs } from "@tauri-apps/api/core";
import {
  listen as tauriListen,
  type EventCallback,
  type UnlistenFn,
} from "@tauri-apps/api/event";
import type { Commands } from "../../types/generated/commands";
import type { PickKind } from "../../types/generated/PickKind";
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

/** Every call is timed (0.3.2, `./timing`): names and times, never data.
 * Untyped: only `call` below uses it. */
function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  return timed(cmd, () =>
    readOnly
      ? readOnly.then((transport) => transport.invoke<T>(cmd, args))
      : tauriInvoke<T>(cmd, args),
  );
}

/** A command's arguments: optional when it takes none it must have. */
type CallArgs<K extends keyof Commands> = Record<string, never> extends Commands[K]["args"]
  ? [args?: Commands[K]["args"]]
  : [args: Commands[K]["args"]];

/** AJ-E2019 (ADR-0029): calls a backend command, its name, arguments and
 * result checked against the Rust signature (`types/generated/commands.ts`,
 * generated from the command registry). The one way lib/ipc reaches the
 * backend. */
export function call<K extends keyof Commands>(name: K, ...args: CallArgs<K>): Promise<Commands[K]["result"]> {
  return invoke<Commands[K]["result"]>(name, args[0]);
}

/** A stream for a command's changes (a Tauri `Channel`). The read-only
 * bridge carries none: what streams is message text, which it never
 * serves (AJ-E2108), so the call goes through `withoutStream` and the
 * bridge refuses it as any it doesn't serve, in words, never a crash
 * (0.3.5). */
export function channel<T>(onMessage: (message: T) => void): Channel<T> | null {
  if (readOnly) return null;
  const made = new Channel<T>();
  made.onmessage = onMessage;
  return made;
}

/** A streaming command where no stream can be made (the read-only
 * bridge, see `channel`): sent bare, so the bridge refuses it in its own
 * words. */
export function withoutStream<K extends keyof Commands>(name: K): Promise<Commands[K]["result"]> {
  return invoke<Commands[K]["result"]>(name);
}

export function listen<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  return readOnly
    ? readOnly.then((transport) => transport.listen<T>(event, handler))
    : tauriListen<T>(event, handler);
}

/** AJ-E2003 R6: the native pickers open from Rust, which remembers the
 * pick so the command that uses it can tell the user chose that path (the
 * page never hands the backend a path of its own). `kind` decides the
 * picker and what the pick may be used for; null when cancelled. The
 * read-only bridge refuses it (a pick starts a write). */
export function pickPath(kind: PickKind, fileName?: string): Promise<string | null> {
  return call("pick_path", { kind, fileName: fileName ?? null });
}
