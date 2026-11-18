/**
 * 0.4.6 (D-126 §4b, the rc1 X2 lab: fifty members wrapped down a narrow
 * column with a sideways scrollbar): who holds each role, as one list in
 * the inspector — each member on a row of its own (its title, then its
 * role), a page at a time with Show more, and a search once there are
 * more than a few. A row opens that member in the inspector, where its
 * role is changed.
 */
import { useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { roleLabel } from "../../lib/pools";
import type { Selection } from "./Canvas";

/** Rows shown at first, and each Show more. */
const PAGE = 10;
/** A search once there are more than this many. */
const SEARCH_FROM = 8;

export type HolderRow = { key: string; name: string; when?: string | null; role: string; select: Selection };

export function RoleHolders({ rows, onSelect }: { rows: readonly HolderRow[]; onSelect?: (selection: Selection) => void }): JSX.Element | null {
  const [words, setWords] = useState("");
  const [shown, setShown] = useState(PAGE);
  if (rows.length === 0) return null;
  const query = words.trim().toLowerCase();
  const found = query ? rows.filter((row) => `${row.name} ${row.when ?? ""} ${roleLabel(row.role)}`.toLowerCase().includes(query)) : rows;
  const page = found.slice(0, shown);
  const left = found.length - page.length;
  return (
    <div className="flex min-w-0 flex-col gap-1.5" data-testid="role-holders">
      <span className="section-label" id="role-holders-title">
        Who holds each role
      </span>
      {rows.length > SEARCH_FROM && (
        <input
          type="search"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          value={words}
          onChange={(event) => {
            setWords(event.target.value);
            setShown(PAGE);
          }}
          placeholder={`Find one of ${rows.length}`}
          aria-label="Find a member"
          className="h-8 rounded-lg border border-hairline bg-surface-2 px-2 text-meta text-ink outline-none placeholder:text-faint focus-visible:ring-2 focus-visible:ring-accent/60"
          data-testid="role-holders-find"
        />
      )}
      <ul aria-labelledby="role-holders-title" className="-mx-2 flex min-w-0 flex-col">
        {page.map((row) => (
          <li key={row.key} className="min-w-0">
            <button
              type="button"
              disabled={!onSelect}
              onClick={() => onSelect?.(row.select)}
              className="flex w-full min-w-0 items-baseline gap-2 rounded-lg px-2 py-1.5 text-left outline-none enabled:hover:bg-surface-3/60 focus-visible:ring-2 focus-visible:ring-accent/60"
              data-testid="role-holder"
            >
              <span className="min-w-0 flex-1 truncate text-meta text-ink" title={row.when ? `${row.name}, ${row.when}` : row.name}>
                {row.name}
                {row.when && <span className="text-label text-faint"> · {row.when}</span>}
              </span>
              <span className="shrink-0 text-label text-muted">{roleLabel(row.role)}</span>
            </button>
          </li>
        ))}
      </ul>
      {found.length === 0 && <p className="text-meta text-muted">No member matches “{words.trim()}”.</p>}
      {left > 0 && (
        <span className="flex items-center gap-2">
          <Button size="sm" variant="ghost" onClick={() => setShown((now) => now + PAGE)} data-testid="role-holders-more">
            Show {Math.min(PAGE, left)} more
          </Button>
          <span className="text-label text-muted">
            {page.length} of {found.length}
          </span>
        </span>
      )}
    </div>
  );
}
