/**
 * AJ-E1506 — find in the terminal: the xterm search addon behind a small
 * bar. Enter finds the next match, Shift+Enter the previous, Esc closes
 * and gives the keys back to the terminal. "3 of 12" as it goes.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import type { ISearchOptions, SearchAddon } from "@xterm/addon-search";

/** Matches marked in the scrollbar and the text, in the app's accent. */
export const SEARCH_LOOK: ISearchOptions = {
  decorations: {
    matchBackground: "#7c7cf455",
    matchBorder: "#7c7cf4aa",
    matchOverviewRuler: "#7c7cf4",
    activeMatchBackground: "#f4b73e",
    activeMatchBorder: "#f4b73e",
    activeMatchColorOverviewRuler: "#f4b73e",
  },
};

export function TerminalSearch({ search, onClose }: { search: SearchAddon; onClose: () => void }): JSX.Element {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<{ index: number; count: number } | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
    const sub = search.onDidChangeResults(({ resultIndex, resultCount }) => setResults({ index: resultIndex, count: resultCount }));
    return () => {
      sub.dispose();
      search.clearDecorations();
    };
  }, [search]);

  const find = (backwards: boolean, text = query) => {
    if (!text) {
      search.clearDecorations();
      setResults(null);
      return;
    }
    if (backwards) search.findPrevious(text, SEARCH_LOOK);
    else search.findNext(text, { ...SEARCH_LOOK, incremental: text !== query });
  };

  const words = !query || !results ? "" : results.count === 0 ? "No matches" : results.index < 0 ? `${results.count} found` : `${results.index + 1} of ${results.count}`;
  return (
    <div className="flex items-center gap-2 border-b border-hairline bg-surface px-2 py-1" data-testid="terminal-search">
      <input
        ref={inputRef}
        type="search"
        value={query}
        aria-label="Find in the terminal"
        placeholder="Find"
        className="h-6 min-w-0 flex-1 rounded-md border border-hairline bg-surface-2 px-2 text-xs text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
        onChange={(event) => {
          setQuery(event.target.value);
          find(false, event.target.value);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            find(event.shiftKey);
          } else if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            onClose();
          }
        }}
        data-testid="terminal-search-input"
      />
      <span className="w-20 shrink-0 text-right font-mono text-label text-muted" aria-live="polite" data-testid="terminal-search-count">
        {words}
      </span>
      <button type="button" className="rounded-md px-1.5 py-0.5 text-label text-muted hover:bg-surface-2 hover:text-ink" title="Previous (Shift+Enter)" onClick={() => find(true)}>
        ↑<span className="sr-only"> previous match</span>
      </button>
      <button type="button" className="rounded-md px-1.5 py-0.5 text-label text-muted hover:bg-surface-2 hover:text-ink" title="Next (Enter)" onClick={() => find(false)}>
        ↓<span className="sr-only"> next match</span>
      </button>
      <button type="button" className="rounded-md px-1.5 py-0.5 text-label text-muted hover:bg-surface-2 hover:text-ink" title="Close (Esc)" onClick={onClose} data-testid="terminal-search-close">
        Done
      </button>
    </div>
  );
}
