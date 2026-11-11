/**
 * E31 (AJ-E3103 Req.3; deep/11 §8 rule 3) — an empty list teaches: what
 * would appear here, and the one step that brings it. A list a search or
 * a filter emptied says so, with the way back.
 */
import type { JSX, ReactNode } from "react";
import { Button } from "../primitives";

export interface Teach {
  /** What would appear here ("Sessions you start in Claude Code · Work"). */
  title: string;
  /** How it comes to be, in a line. */
  body?: string;
  /** The one step that brings it. */
  action?: { label: string; icon?: ReactNode; onClick: () => void };
}

export function RelatedEmpty({ teach, testId = "related-empty" }: { teach: Teach; testId?: string }): JSX.Element {
  return (
    <div className="flex flex-col items-center gap-3 rounded-card border border-hairline bg-surface px-6 py-8 text-center" data-testid={testId}>
      <div className="flex max-w-prose flex-col items-center gap-1">
        <p className="text-body font-medium text-ink">{teach.title}</p>
        {teach.body && <p className="text-meta text-ink-2">{teach.body}</p>}
      </div>
      {/* Secondary: the page's own action is its one filled button (AJ-E3305, One primary). */}
      {teach.action && (
        <Button size="sm" variant="secondary" onClick={teach.action.onClick}>
          {teach.action.icon}
          {teach.action.label}
        </Button>
      )}
    </div>
  );
}

/** Nothing matches the search or the filters: say what, and undo it. */
export function NothingMatches({ words, filtered, onClear }: { words: string; filtered: boolean; onClear: () => void }): JSX.Element {
  const what = words ? `Nothing here matches “${words}”${filtered ? " with these filters" : ""}.` : "Nothing here matches these filters.";
  return (
    <div className="flex flex-col items-center gap-2 rounded-card border border-hairline bg-surface px-6 py-6 text-center" data-testid="related-no-match">
      <p className="text-body text-ink-2">{what}</p>
      <button type="button" onClick={onClear} className="rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="related-clear">
        {words && filtered ? "Clear the search and filters" : words ? "Clear the search" : "Clear filters"}
      </button>
    </div>
  );
}
