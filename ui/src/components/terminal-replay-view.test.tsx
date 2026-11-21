/**
 * SK-23 (the 0.4.7 persona runs): a running session's page opened on a
 * blank "Live" terminal from Home, and on its question from the sidebar.
 * Both ways mount the session's terminal view: from Home with its page,
 * from another session's page in the page already there, the view moved
 * to the new id. Either way, what the session printed before shows, once,
 * and "Live" waits for it; what it printed while the view was setting up
 * (after the kept output was read, before its listener was in place) is
 * not lost.
 */
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

/** The backend's terminals: each one's whole output, and who listens. */
const backend = vi.hoisted(() => {
  const encode = (text: string) => btoa(String.fromCharCode(...new TextEncoder().encode(text)));
  const bytes = (text: string) => new TextEncoder().encode(text).length;
  const outputs = new Map<string, string>();
  const listeners = new Map<string, Set<(chunk: { id: string; data: string; end: number }) => void>>();
  /** While set, a listener is put in place only when `listen()` is called. */
  let held: (() => void)[] | null = null;
  return {
    outputs,
    /** The session prints `text`: kept, and sent to whoever listens. */
    print(id: string, text: string) {
      const all = (outputs.get(id) ?? "") + text;
      outputs.set(id, all);
      for (const heard of listeners.get(id) ?? []) heard({ id, data: encode(text), end: bytes(all) });
    },
    holdListeners() {
      held = [];
    },
    listen() {
      const waiting = held ?? [];
      held = null;
      waiting.forEach((place) => place());
    },
    reset() {
      outputs.clear();
      listeners.clear();
      held = null;
    },
    /** What's kept, read as the call is made (a backend reads it then). */
    snapshot(id: string, from?: number) {
      const all = outputs.get(id);
      if (all === undefined) return Promise.reject(new Error("unknown"));
      const raw = new TextEncoder().encode(all);
      const after = raw.slice(Math.min(raw.length, from ?? 0));
      return Promise.resolve({ data: btoa(String.fromCharCode(...after)), end: raw.length });
    },
    hear(id: string, heard: (chunk: { id: string; data: string; end: number }) => void, listening?: () => void) {
      let gone = false;
      const place = () => {
        if (gone) return;
        if (!listeners.has(id)) listeners.set(id, new Set());
        listeners.get(id)!.add(heard);
        listening?.();
      };
      if (held) held.push(place);
      else void Promise.resolve().then(place);
      return () => {
        gone = true;
        listeners.get(id)?.delete(heard);
      };
    },
  };
});

/** The xterm stand-ins made, in order: what each was given to show. */
const terms = vi.hoisted(() => [] as { shown: string; disposed: boolean }[]);

vi.mock("@xterm/xterm", () => {
  class Terminal {
    options: Record<string, unknown>;
    buffer = { active: { viewportY: 0, baseY: 0 } };
    textarea = document.createElement("textarea");
    rows = 24;
    cols = 80;
    record = { shown: "", disposed: false };
    constructor(options: Record<string, unknown>) {
      this.options = { ...options };
      terms.push(this.record);
    }
    loadAddon() {}
    registerLinkProvider() {
      return { dispose() {} };
    }
    open() {}
    attachCustomKeyEventHandler() {}
    onScroll() {
      return { dispose() {} };
    }
    onWriteParsed() {
      return { dispose() {} };
    }
    onData() {
      return { dispose() {} };
    }
    write(data: string | Uint8Array, done?: () => void) {
      this.record.shown += typeof data === "string" ? data : new TextDecoder().decode(data);
      if (done) void Promise.resolve().then(done);
    }
    resize() {}
    focus() {}
    scrollToBottom() {}
    hasSelection() {
      return false;
    }
    getSelection() {
      return "";
    }
    dispose() {
      this.record.disposed = true;
    }
  }
  return { Terminal };
});
vi.mock("@xterm/addon-fit", () => ({ FitAddon: class { proposeDimensions() { return { cols: 80, rows: 24 }; } } }));
vi.mock("@xterm/addon-search", () => ({ SearchAddon: class { dispose() {} } }));
vi.mock("@xterm/addon-web-links", () => ({ WebLinksAddon: class { dispose() {} } }));
vi.mock("../lib/terminal-render", () => ({ loadGlyphRenderer: () => {}, TERMINAL_FONTS: "monospace" }));
vi.mock("../lib/terminal-caret", () => ({ blinkWhileUsed: () => {}, followCaret: () => null, holdInput: () => {} }));
vi.mock("../lib/bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("../lib/bridge")>()), hasLiveBackend: () => true }));
vi.mock("../lib/ipc/terminal", () => ({
  followTerminalWindows: () => () => {},
  terminalOpenLink: async () => {},
  terminalPopIn: async () => {},
  terminalPopOut: async () => {},
}));
vi.mock("../lib/ipc/terminal-io", () => ({
  terminalSnapshot: (id: string, from?: number) => backend.snapshot(id, from),
  terminalAttach: async (id: string) => ({ id }),
  onTerminalOutput: (id: string, heard: (chunk: { id: string; data: string; end: number }) => void, listening?: () => void) => backend.hear(id, heard, listening),
  onTerminalExit: () => () => {},
  terminalResize: async () => {},
  terminalWrite: async () => {},
}));

import { TerminalView } from "./TerminalView";

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
});
afterEach(() => {
  cleanup();
  backend.reset();
  terms.length = 0;
  vi.unstubAllGlobals();
});

const QUESTION = " │ Bash command: npm test\r\n │ Do you want to proceed?\r\n │ ❯ 1. Yes\r\n";

it("from Home: the page mounts with the session's terminal, and shows what it printed before, under Live", async () => {
  backend.print("s-asks", " ✻ Welcome to Claude Code\r\n");
  backend.print("s-asks", QUESTION);
  render(<TerminalView id="s-asks" title="web" status="waiting_user" />);
  // Being read: its rows say so, never a blank pane under "Live".
  expect(screen.getByTestId("terminal-replaying")).toBeTruthy();
  expect(screen.queryByText("Live")).toBeNull();
  await waitFor(() => expect(screen.getByText("Live")).toBeTruthy());
  expect(screen.queryByTestId("terminal-replaying")).toBeNull();
  expect(terms.at(-1)!.shown).toBe(" ✻ Welcome to Claude Code\r\n" + QUESTION);
});

it("from the sidebar on another session's page: the view moves to the session and shows its own output, none of the other's", async () => {
  backend.print("s-error", "npm ERR! missing script\r\n");
  backend.print("s-asks", QUESTION);
  const view = render(<TerminalView id="s-error" title="api" status="running" />);
  await waitFor(() => expect(terms.at(-1)!.shown).toContain("npm ERR!"));
  view.rerender(<TerminalView id="s-asks" title="web" status="waiting_user" />);
  await waitFor(() => expect(screen.getByText("Live")).toBeTruthy());
  const shown = terms.at(-1)!;
  await waitFor(() => expect(shown.shown).toBe(QUESTION));
  expect(terms[0]!.disposed).toBe(true);
  // Printed later, it follows, once.
  act(() => backend.print("s-asks", "\r\n   (answered)\r\n"));
  expect(shown.shown).toBe(QUESTION + "\r\n   (answered)\r\n");
});

it("what it printed after its kept output was read, before the view listened, shows once (the session then waits)", async () => {
  backend.print("s-asks", " ✻ Welcome to Claude Code\r\n");
  backend.holdListeners();
  render(<TerminalView id="s-asks" title="web" status="running" />);
  // The kept output is read; then the session asks, with no one listening yet.
  await waitFor(() => expect(terms.at(-1)!.shown).toBe(" ✻ Welcome to Claude Code\r\n"));
  backend.print("s-asks", QUESTION);
  await act(async () => backend.listen());
  await waitFor(() => expect(terms.at(-1)!.shown).toBe(" ✻ Welcome to Claude Code\r\n" + QUESTION));
  // Nothing twice once more comes.
  act(() => backend.print("s-asks", "y"));
  expect(terms.at(-1)!.shown).toBe(" ✻ Welcome to Claude Code\r\n" + QUESTION + "y");
});

it("one it can't reach yet says Reconnecting over its own rows, never a skeleton for ever", async () => {
  render(<TerminalView id="s-not-here" title="web" status="running" />);
  expect(screen.getByTestId("terminal-replaying")).toBeTruthy();
  await waitFor(() => expect(screen.queryByTestId("terminal-replaying")).toBeNull());
  expect(screen.getByText("Reconnecting")).toBeTruthy();
});
