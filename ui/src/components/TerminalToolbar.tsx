/**
 * AJ-E1506 — the strip above a terminal (wireframe 05): whether it is
 * live, and its tools — find, copy, text size, maximize, and its own
 * window. Short labels, each with its key in the tooltip; a word on what
 * just happened (copied, or why a link didn't open) at its start.
 */
import type { JSX, ReactNode } from "react";

export type TerminalConnection = "live" | "reconnecting" | "ended";

const PILL: Record<TerminalConnection, { words: string; dot: string; tone: string }> = {
  live: { words: "Live", dot: "dot-live bg-live", tone: "text-live" },
  reconnecting: { words: "Reconnecting", dot: "bg-warning", tone: "text-warning" },
  ended: { words: "Ended", dot: "bg-faint", tone: "text-muted" },
};

const mac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
/** A key as this Mac (or PC) writes it: ⌘F, or Ctrl+Shift+F. */
export const keyName = (key: string) => (mac ? `⌘${key}` : `Ctrl+Shift+${key}`);

function Tool({ label, title, pressed, onClick, children, testId }: { label: string; title: string; pressed?: boolean; onClick: () => void; children?: ReactNode; testId: string }): JSX.Element {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      aria-pressed={pressed}
      className="rounded-md px-2 py-1 text-label text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60 aria-pressed:bg-surface-2 aria-pressed:text-ink"
      data-testid={testId}
    >
      {children ?? label}
    </button>
  );
}

export function TerminalToolbar({
  connection,
  note,
  searchOpen,
  maximized,
  ownWindow,
  onFind,
  onCopy,
  onZoom,
  onMaximize,
  onWindow,
}: {
  connection: TerminalConnection;
  /** What just happened ("Copied", a link that didn't open), said politely. */
  note: string | null;
  searchOpen: boolean;
  maximized: boolean;
  /** In the terminal's own window: "Back" instead of "Own window", no Maximize. */
  ownWindow: boolean;
  onFind: () => void;
  onCopy: () => void;
  onZoom: (step: -1 | 1) => void;
  onMaximize: () => void;
  onWindow: () => void;
}): JSX.Element {
  const pill = PILL[connection];
  return (
    <div className="flex flex-wrap items-center gap-x-1 gap-y-1 border-b border-hairline bg-surface px-2 py-1" data-testid="terminal-toolbar">
      <span className={`flex items-center gap-1.5 px-1.5 text-label font-medium ${pill.tone}`} data-testid="terminal-connection" data-connection={connection}>
        <span aria-hidden="true" className={`size-1.5 rounded-full ${pill.dot}`} />
        {pill.words}
      </span>
      <span className="min-w-0 flex-1 truncate px-1 text-label text-muted" role="status" data-testid="terminal-note">
        {note}
      </span>
      <Tool label="Find" title={`Find in the terminal (${keyName("F")})`} pressed={searchOpen} onClick={onFind} testId="terminal-find" />
      <Tool label="Copy" title="Copy the selection, or what's on screen" onClick={onCopy} testId="terminal-copy" />
      <span className="flex items-center" role="group" aria-label="Text size">
        <Tool label="Smaller text" title={`Smaller text (${keyName("−")})`} onClick={() => onZoom(-1)} testId="terminal-smaller">
          A−<span className="sr-only"> smaller text</span>
        </Tool>
        <Tool label="Larger text" title={`Larger text (${keyName("+")})`} onClick={() => onZoom(1)} testId="terminal-larger">
          A+<span className="sr-only"> larger text</span>
        </Tool>
      </span>
      {!ownWindow && (
        <Tool label={maximized ? "Restore" : "Maximize"} title={`${maximized ? "Back to the panel" : "Fill the window"} (${keyName("↩")}${maximized ? ", or Esc outside the terminal" : ""})`} pressed={maximized} onClick={onMaximize} testId="terminal-maximize" />
      )}
      <Tool
        label={ownWindow ? "Back to the main window" : "Own window"}
        title={ownWindow ? "Show this terminal in the main window again; the session keeps running" : "Open this terminal in its own window, for another display; the session keeps running"}
        onClick={onWindow}
        testId="terminal-window"
      />
    </div>
  );
}
