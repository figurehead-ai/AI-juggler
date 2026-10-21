/**
 * The read-only test bridge transport (docs/qa/DEV-BRIDGE.md): the real
 * UI in a browser, against a running AI Juggler Dev build. Dev builds
 * only — transport.ts imports this module behind `import.meta.env.DEV`,
 * so production bundles never contain it.
 *
 * Reads go to `POST /v1/invoke/<command>` with the run's bearer token;
 * the refresh events arrive over `GET /v1/events` (SSE). That stream is
 * read with fetch rather than EventSource, because EventSource can't send
 * the Authorization header and the token must not sit in a URL. Any
 * command the bridge doesn't serve — every write — is refused here,
 * before anything is sent, and the page says so.
 */
import type { InvokeArgs } from "@tauri-apps/api/core";
import type { EventCallback, UnlistenFn } from "@tauri-apps/api/event";
import type { Transport } from "./transport";

export const READ_ONLY_MESSAGE = "Read-only test bridge — not available";

/** Where a reload in the same tab finds the run's port and token. */
const SESSION_KEY = "aij.readOnlyBridge";

type Session = { port: number; token: string };

/** The port from `?devBridge=`; the token from `token=` in the query or
 * the fragment, then out of the address bar (kept for this tab only). */
export function sessionFrom(page: URL, storage: Storage | null): Session | null {
  const port = Number(page.searchParams.get("devBridge"));
  const fragment = new URLSearchParams(page.hash.replace(/^#/, ""));
  const token = page.searchParams.get("token") ?? fragment.get("token");
  if (token !== null) {
    page.searchParams.delete("token");
    fragment.delete("token");
    page.hash = fragment.toString();
  }
  let session: Session | null = null;
  if (Number.isInteger(port) && port > 0 && port < 65536 && token && /^[0-9a-f]{64}$/.test(token)) {
    session = { port, token };
    try {
      storage?.setItem(SESSION_KEY, JSON.stringify(session));
    } catch {
      // Storage off: a reload needs the URL again.
    }
  } else {
    try {
      const kept = JSON.parse(storage?.getItem(SESSION_KEY) ?? "null") as Session | null;
      if (kept && kept.port === port && /^[0-9a-f]{64}$/.test(kept.token)) session = kept;
    } catch {
      session = null;
    }
  }
  return session;
}

/** The transport for this page, from its URL. */
export async function connect(page: URL): Promise<Transport> {
  let storage: Storage | null = null;
  try {
    storage = window.sessionStorage;
  } catch {
    storage = null;
  }
  const session = sessionFrom(page, storage);
  window.history.replaceState(window.history.state, "", page.toString());
  const notice = banner();
  if (!session) {
    notice.say("Read-only test bridge: no token — open the URL from docs/qa/DEV-BRIDGE.md.");
    return transport(null, new Set(), notice);
  }
  const base = `http://127.0.0.1:${session.port}`;
  try {
    const response = await fetch(`${base}/v1/commands`, {
      headers: { Authorization: `Bearer ${session.token}` },
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const reads = new Set((await response.json()) as string[]);
    notice.say("Read-only test bridge — real data; nothing here can change it.");
    return transport({ base, token: session.token }, reads, notice);
  } catch {
    notice.say("Read-only test bridge: can't reach the Dev build. Is it running with AIJ_DEV_BRIDGE=1?");
    return transport(null, new Set(), notice);
  }
}

type Endpoint = { base: string; token: string };

function transport(endpoint: Endpoint | null, reads: Set<string>, notice: Notice): Transport {
  const refuse = (what: string) => {
    notice.say(`${READ_ONLY_MESSAGE} (${what}).`);
    console.info(`[read-only test bridge] refused: ${what}`);
    return { code: "read_only_bridge", retryable: false, user_message: READ_ONLY_MESSAGE };
  };
  const stream = endpoint ? events(endpoint) : null;
  return {
    refuse,
    async invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
      if (!endpoint || !reads.has(cmd)) throw refuse(cmd);
      const response = await fetch(`${endpoint.base}/v1/invoke/${encodeURIComponent(cmd)}`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${endpoint.token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(args ?? {}),
      });
      const body: unknown = await response.json().catch(() => null);
      if (response.status === 403) throw refuse(cmd);
      if (!response.ok) {
        throw body ?? { code: "bridge", retryable: true, user_message: `Test bridge: HTTP ${response.status}` };
      }
      return body as T;
    },
    async listen<T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> {
      if (!stream) return () => {};
      return stream.add(event, handler as EventCallback<unknown>);
    },
  };
}

type Listener = { event: string; handler: EventCallback<unknown> };

/** One SSE stream for the page, shared by every listener; it reconnects
 * with a growing pause if the Dev build goes away. */
function events(endpoint: Endpoint) {
  const listeners = new Set<Listener>();
  let id = 0;
  let pause = 1000;

  const deliver = (frame: string) => {
    let name = "";
    const data: string[] = [];
    for (const line of frame.split("\n")) {
      if (line.startsWith("event: ")) name = line.slice(7);
      else if (line.startsWith("data: ")) data.push(line.slice(6));
    }
    if (!name || data.length === 0) return;
    let payload: unknown;
    try {
      payload = JSON.parse(data.join("\n"));
    } catch {
      return;
    }
    id += 1;
    for (const listener of listeners) {
      if (listener.event === name) listener.handler({ event: name, id, payload } as never);
    }
  };

  const open = async () => {
    try {
      const response = await fetch(`${endpoint.base}/v1/events`, {
        headers: { Authorization: `Bearer ${endpoint.token}` },
      });
      if (!response.ok || !response.body) throw new Error(`HTTP ${response.status}`);
      pause = 1000;
      const reader = response.body.pipeThrough(new TextDecoderStream()).getReader();
      let buffered = "";
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        buffered += value;
        let end = buffered.indexOf("\n\n");
        while (end >= 0) {
          deliver(buffered.slice(0, end));
          buffered = buffered.slice(end + 2);
          end = buffered.indexOf("\n\n");
        }
      }
    } catch {
      // Gone: try again below.
    }
    setTimeout(() => void open(), pause);
    pause = Math.min(pause * 2, 30_000);
  };
  void open();

  return {
    add(event: string, handler: EventCallback<unknown>): UnlistenFn {
      const listener = { event, handler };
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

type Notice = { say(text: string): void };

/** A plain status line at the foot of the page, so nobody mistakes the
 * bridge for the app: it names the mode and each refused write. */
function banner(): Notice {
  const line = document.createElement("div");
  line.setAttribute("role", "status");
  line.setAttribute("aria-live", "polite");
  line.dataset.readOnlyBridge = "";
  Object.assign(line.style, {
    position: "fixed",
    left: "0",
    right: "0",
    bottom: "0",
    zIndex: "2147483647",
    padding: "4px 12px",
    font: "12px/1.4 system-ui, sans-serif",
    background: "#7c2d12",
    color: "#ffffff",
    pointerEvents: "none",
    textAlign: "center",
  });
  document.body.append(line);
  return {
    say(text: string) {
      line.textContent = text;
    },
  };
}
