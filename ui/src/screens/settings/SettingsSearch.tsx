/**
 * AJ-E2905: Settings' search — a field atop the sidebar (`/` from
 * anywhere in Settings, Esc clears). It lists the sections that match as
 * "Pane › Section", ↑↓ to move and ↩ to open one at its place; the panes
 * with nothing that matches dim in the sidebar. It searches the index
 * (`settings-index.ts`), so no pane is drawn to search it.
 */
import { useEffect, useId, useRef, useState, type JSX } from "react";
import { type IndexEntry } from "./settings-index";

export type SearchHit = IndexEntry & { paneLabel: string; rowAnchor: string };

export function SettingsSearch({
  query,
  hits,
  onQuery,
  onPick,
}: {
  query: string;
  hits: SearchHit[];
  onQuery: (query: string) => void;
  onPick: (hit: SearchHit) => void;
}): JSX.Element {
  const inputRef = useRef<HTMLInputElement>(null);
  const [active, setActive] = useState(0);
  const listId = useId();
  useEffect(() => setActive(0), [query]);
  // `/` focuses it, unless you're typing somewhere.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "/" || event.metaKey || event.ctrlKey || event.altKey)
        return;
      const target = event.target as HTMLElement | null;
      if (
        target &&
        (target.isContentEditable ||
          ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
      )
        return;
      event.preventDefault();
      inputRef.current?.focus();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  const searching = query.trim() !== "";
  const pick = (hit: SearchHit | undefined) => {
    if (!hit) return;
    onPick(hit);
    onQuery("");
  };
  return (
    <div
      className="mb-3 flex flex-col gap-1.5 px-1"
      data-testid="settings-search"
    >
      <div className="relative">
        <span
          aria-hidden="true"
          className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted"
        >
          ⌕
        </span>
        <input
          ref={inputRef}
          type="search"
          role="combobox"
          aria-expanded={searching && hits.length > 0}
          aria-controls={searching && hits.length > 0 ? listId : undefined}
          aria-activedescendant={
            searching && hits[active] ? `${listId}-${active}` : undefined
          }
          aria-keyshortcuts="/"
          value={query}
          onChange={(event) => onQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              // Esc clears it, then leaves it.
              event.preventDefault();
              event.stopPropagation();
              if (query) onQuery("");
              else inputRef.current?.blur();
            } else if (event.key === "ArrowDown" && hits.length > 0) {
              event.preventDefault();
              setActive((index) => (index + 1) % hits.length);
            } else if (event.key === "ArrowUp" && hits.length > 0) {
              event.preventDefault();
              setActive((index) => (index - 1 + hits.length) % hits.length);
            } else if (event.key === "Enter") {
              event.preventDefault();
              pick(hits[active]);
            }
          }}
          placeholder="Search settings /"
          aria-label="Search settings"
          className="h-7 w-full rounded-lg border border-hairline bg-surface/70 pl-6 pr-2 text-meta text-ink outline-none placeholder:text-muted focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="settings-search-field"
        />
      </div>
      {searching && hits.length === 0 && (
        <p role="status" className="rounded-lg border border-hairline bg-surface px-2 py-1.5 text-meta text-muted" data-testid="settings-search-none">
          No setting matches “{query.trim()}”.
        </p>
      )}
      {searching && hits.length > 0 && (
        <ul id={listId} role="listbox" aria-label="Settings that match" className="flex flex-col gap-0.5 rounded-lg border border-hairline bg-surface p-1" data-testid="settings-search-results">
          {hits.map((hit, index) => (
            <li
              key={`${hit.pane}:${hit.anchor}`}
              id={`${listId}-${index}`}
              role="option"
              aria-selected={index === active}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => pick(hit)}
              onMouseMove={() => setActive(index)}
              className={"flex cursor-default flex-col rounded-md px-2 py-1 " + (index === active ? "bg-accent-soft" : "")}
              data-testid="settings-search-hit"
              data-anchor={hit.anchor}
            >
              <span className="text-label text-muted">{hit.paneLabel}</span>
              <span className="text-meta text-ink">{hit.title}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
