import { useEffect, useId, useRef } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import {
  hasLiveBackend,
  onTerminalOutput,
  terminalAttach,
  terminalResize,
  terminalSnapshot,
  terminalWrite,
} from "../lib/bridge";

const SCROLLBACK_LINES = 5_000;

const TABBABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * AJ-E804 (WCAG 2.1.2, no keyboard trap): Tab and Shift+Tab belong to the
 * shell — completion, and the agent's own mode cycling — so Ctrl+Tab /
 * Ctrl+Shift+Tab leave the terminal instead. Terminals send no sequence
 * for Ctrl+Tab, so no CLI loses a key. Moves focus to the next (or
 * previous) focusable element outside `host`.
 */
function leaveTerminal(host: HTMLElement, backwards: boolean) {
  const all = Array.from(document.querySelectorAll<HTMLElement>(TABBABLE)).filter(
    (el) => el.offsetParent !== null || el === document.activeElement,
  );
  const outside = all.filter((el) => !host.contains(el));
  const firstInside = all.findIndex((el) => host.contains(el));
  const before = all.slice(0, Math.max(0, firstInside)).filter((el) => !host.contains(el));
  const after = all.slice(firstInside).filter((el) => !host.contains(el));
  const target = backwards ? (before.at(-1) ?? outside.at(-1)) : (after[0] ?? outside[0]);
  target?.focus();
}

function decodeBase64(data: string): Uint8Array {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

/**
 * One xterm instance bound to a backend PTY (M1-4), embedded in the panel
 * of whatever session/login selected it: replays the retained tail on
 * (re)mount, streams live output, sends keystrokes, resizes with its
 * container. Closing and reopening a panel loses nothing.
 */
export function TerminalView({ id, title }: { id: string; title: string }) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const hintId = useId();

  useEffect(() => {
    const host = hostRef.current;
    if (!host || termRef.current) return;

    const term = new Terminal({
      scrollback: SCROLLBACK_LINES,
      cursorBlink: true,
      fontSize: 12.5,
      fontFamily: "'SF Mono', 'JetBrains Mono', 'Fira Code', Menlo, monospace",
      allowProposedApi: true,
      theme: {
        background: "#05070b",
        foreground: "#cdd5e0",
        cursor: "#7c7cf4",
        selectionBackground: "#7c7cf440",
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    term.attachCustomKeyEventHandler((event) => {
      if (event.key === "Tab" && event.ctrlKey) {
        if (event.type === "keydown") {
          event.preventDefault();
          leaveTerminal(host, event.shiftKey);
        }
        return false;
      }
      return true;
    });
    term.textarea?.setAttribute("aria-describedby", hintId);
    termRef.current = term;
    fitRef.current = fit;

    if (hasLiveBackend()) {
      terminalSnapshot(id).then((b64) => {
        if (b64) term.write(decodeBase64(b64));
      });
      terminalAttach(id);
      term.onData((data) => {
        void terminalWrite(id, data);
      });
    } else {
      term.write(
        [
          "\u001b[1;38;5;140m" + title + "\u001b[0m",
          "",
          "  ? Ask anything — this terminal is isolated to its account",
          "  and project.",
          "",
          "\u001b[38;5;245m(preview — run the app for a live terminal)\u001b[0m",
          "",
        ].join("\r\n"),
      );
    }

    const containerObserver = new ResizeObserver(() => {
      if (!host.offsetParent) return;
      try {
        const dims = fit.proposeDimensions();
        if (dims && dims.cols > 0 && dims.rows > 0) {
          term.resize(dims.cols, dims.rows);
          if (hasLiveBackend()) void terminalResize(id, dims.cols, dims.rows);
        }
      } catch {
        /* mid-teardown resizes are harmless */
      }
    });
    containerObserver.observe(host);
    // Initial fit once the host is laid out.
    requestAnimationFrame(() => {
      try {
        fitRef.current?.fit();
        termRef.current?.focus();
      } catch {
        /* panel mid-animation */
      }
    });

    return () => {
      containerObserver.disconnect();
      term.dispose();
      termRef.current = null;
    };
  }, [id, title, hintId]);

  useEffect(() => {
    if (!hasLiveBackend()) return;
    const off = onTerminalOutput(id, (b64) => termRef.current?.write(decodeBase64(b64)));
    return off;
  }, [id]);

  return (
    <div className="group/term relative h-full w-full">
      <div ref={hostRef} className="h-full w-full px-3 pt-2" data-terminal-host={id} />
      {/* Shown only while the terminal has focus; always read with it. */}
      <p
        id={hintId}
        className="pointer-events-none absolute bottom-1 right-2 rounded bg-surface-3 px-1.5 py-0.5 text-[10px] text-muted opacity-0 group-focus-within/term:opacity-100"
        data-testid="terminal-leave-hint"
      >
        ⌃Tab leaves the terminal
      </p>
    </div>
  );
}
