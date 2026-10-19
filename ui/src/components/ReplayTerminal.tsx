/**
 * AJ-E1007 — a read-only terminal for a replay: the same xterm view a
 * live Instance uses, with input disabled. Nothing typed here reaches
 * anything.
 */
import { useEffect, useRef, type JSX } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import type { ReplayEntryInfo } from "../lib/bridge";

const ROLE_COLOUR: Record<string, string> = {
  user: "\x1b[36m",
  assistant: "\x1b[35m",
  system: "\x1b[33m",
};

export function ReplayTerminal({ entries }: { entries: ReplayEntryInfo[] }): JSX.Element {
  const hostRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const term = new Terminal({
      disableStdin: true,
      convertEol: true,
      scrollback: 50_000,
      // The same look as a live Instance's terminal (TerminalView).
      fontSize: 12.5,
      fontFamily: "'SF Mono', 'JetBrains Mono', 'Fira Code', Menlo, monospace",
      cursorStyle: "underline",
      cursorBlink: false,
      theme: {
        background: "#05070b",
        foreground: "#cdd5e0",
        cursor: "#05070b",
        selectionBackground: "#7c7cf440",
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    fit.fit();
    for (const entry of entries) {
      const colour = ROLE_COLOUR[entry.role] ?? "\x1b[90m";
      term.writeln(`${colour}${entry.role}\x1b[0m`);
      term.writeln(entry.text);
      term.writeln("");
    }
    const resize = new ResizeObserver(() => fit.fit());
    resize.observe(host);
    return () => {
      resize.disconnect();
      term.dispose();
    };
  }, [entries]);
  return <div ref={hostRef} className="h-full min-h-[240px] w-full" aria-label="Transcript, read-only" role="log" />;
}
