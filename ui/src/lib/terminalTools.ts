/**
 * AJ-E1506 — the terminal tools' pieces that work on xterm itself: the
 * file-path links (`findPathLinks` on each line, mapped to cells), what's
 * on screen for Copy, and the tools' keys.
 */
import type { IBufferLine, ILinkProvider, Terminal } from "@xterm/xterm";
import { findPathLinks } from "./terminalLinks";
import { matches, type ShortcutId } from "./shortcuts";

/** Each character of a line's text → its cell's column (1-based); wide
 * characters take two cells, so text index and column differ after one. */
function columns(line: IBufferLine, cols: number): number[] {
  const at: number[] = [];
  for (let x = 0; x < cols; x += 1) {
    const cell = line.getCell(x);
    if (!cell || cell.getWidth() === 0) continue;
    const chars = cell.getChars() || " ";
    for (let i = 0; i < chars.length; i += 1) at.push(x + 1);
  }
  return at;
}

/** File paths as links: hover says how to open, ⌘-click (Ctrl-click) opens. */
export function pathLinkProvider(term: Terminal, open: (text: string) => void, hint: (text: string | null) => void): ILinkProvider {
  return {
    provideLinks(y, callback) {
      const line = term.buffer.active.getLine(y - 1);
      if (!line) return callback(undefined);
      const text = line.translateToString(true);
      const found = findPathLinks(text);
      if (found.length === 0) return callback(undefined);
      const at = columns(line, term.cols);
      callback(
        found.map((link) => ({
          range: { start: { x: at[link.start] ?? link.start + 1, y }, end: { x: at[link.end - 1] ?? link.end, y } },
          text: link.text,
          decorations: { underline: true, pointerCursor: true },
          activate: (event, linkText) => {
            if (event.metaKey || event.ctrlKey) open(linkText);
          },
          hover: () => hint(link.text),
          leave: () => hint(null),
        })),
      );
    },
  };
}

/** The selection, or else what's on screen now (trailing blank lines dropped). */
export function copyText(term: Terminal): string {
  const selected = term.getSelection();
  if (selected) return selected;
  const buffer = term.buffer.active;
  const lines: string[] = [];
  for (let y = buffer.viewportY; y < buffer.viewportY + term.rows; y += 1) lines.push(buffer.getLine(y)?.translateToString(true) ?? "");
  while (lines.length > 0 && lines.at(-1)!.trim() === "") lines.pop();
  return lines.join("\n");
}

export type TerminalKey = "find" | "larger" | "smaller" | "reset" | "maximize";

const KEYS: [ShortcutId, TerminalKey][] = [
  ["terminal-find", "find"],
  ["terminal-larger", "larger"],
  ["terminal-smaller", "smaller"],
  ["terminal-reset", "reset"],
  ["terminal-maximize", "maximize"],
];

/** The tools' keys while typing in the terminal, from the shortcut
 * registry (the "?" sheet lists the same ones). */
export function terminalKey(event: KeyboardEvent): TerminalKey | null {
  return KEYS.find(([id]) => matches(id, event))?.[1] ?? null;
}
