/**
 * AJ-E1506 — a terminal in its own window (wireframe 05's pop-out): the
 * same PTY and scrollback as the main window's, filling a real window the
 * user can put on another display. "Back to the main window" closes it;
 * so does closing the window. The session keeps running either way.
 */
import type { JSX } from "react";
import { TerminalView } from "../components/TerminalView";

export function TerminalWindow({ id }: { id: string }): JSX.Element {
  return (
    <main className="flex h-screen w-screen flex-col bg-[#05070b]" aria-label="Terminal" data-testid="terminal-window-root">
      <TerminalView id={id} title="" ownWindow />
    </main>
  );
}
