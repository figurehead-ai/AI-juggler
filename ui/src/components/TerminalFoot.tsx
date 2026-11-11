/**
 * The strip below a terminal's rows: what used to float over them. The
 * rc20–rc22 walks found controls covering terminal text — the ⌃Tab hint
 * over the last row, and macOS's input-source mark (a small ⊖ it draws at
 * the focused field's caret; xterm keeps its caret on the cursor cell, so
 * the mark sat over "Help" in Claude Code's /help and beside OpenCode's
 * status line). Here live the link hint, Jump to the bottom and the ⌃Tab
 * hint, and the 44 px slot on the left is where styles.css parks the
 * caret (and so the mark), except while an IME composes.
 */
import type { JSX } from "react";
import { keyName } from "./TerminalToolbar";

export function TerminalFoot({ hintId, linkHint, scrolledUp, onJump }: { hintId: string; linkHint: string | null; scrolledUp: boolean; onJump: () => void }): JSX.Element {
  return (
    <div className="flex h-6 shrink-0 items-center gap-2 pl-11 pr-2" data-testid="terminal-foot">
      {linkHint && (
        <p className="pointer-events-none min-w-0 truncate rounded bg-surface-3 px-1.5 py-0.5 text-label text-muted" data-testid="terminal-link-hint">
          {/Mac/.test(navigator.platform) ? "⌘-click" : "Ctrl-click"} opens {linkHint}
        </p>
      )}
      <span className="flex-1" />
      {scrolledUp && (
        <button
          type="button"
          className="shrink-0 rounded-md border border-hairline bg-surface px-2 py-0.5 text-label text-ink hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60"
          onClick={onJump}
          data-testid="terminal-jump-bottom"
        >
          ↓ Jump to the bottom
        </button>
      )}
      {/* Shown only while the terminal has focus; always read with it. */}
      <p
        id={hintId}
        className="pointer-events-none shrink-0 whitespace-nowrap rounded bg-surface-3 px-1.5 py-0.5 text-label text-muted opacity-0 group-focus-within/term:opacity-100"
        data-testid="terminal-leave-hint"
      >
        ⌃Tab leaves the terminal · {keyName("F")} finds
      </p>
    </div>
  );
}
