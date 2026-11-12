/**
 * AJ-E2104 — the rail by place's smaller parts: a section's head (Needs
 * you, Open now with its order menu, D-47), the fold chevron and the
 * "Also:" line. `PlaceBoard` draws the list with them.
 */
import { Fragment, type JSX, type ReactNode } from "react";
import { flushSync } from "react-dom";
import { moveLine } from "../lib/roving";
import { IconAlert } from "./icons";
import { IconHistory, IconWindow } from "./icons-places";
import { IconPlus } from "./icons-actions";
import type { HiddenApp } from "../lib/shown-apps";
import type { AlsoCounts, Line, OffRail, SectionId } from "../lib/rail-lines";
import { isSearching, parseQuery, showInRail, useRailView, type ShowKey } from "../lib/rail-view";
import { searchHistory } from "../lib/nav";

const SECTION: Record<SectionId, { label: string; tone: string; Icon: typeof IconAlert; testId: string }> = {
  needs: { label: "Needs you", tone: "text-warning", Icon: IconAlert, testId: "rail-needs-you" },
  open: { label: "Open now", tone: "text-live", Icon: IconWindow, testId: "rail-open-now" },
  // D-100: apps found running that you haven't added (Add), then D-64:
  // your apps that aren't running.
  offered: { label: "Running, not added", tone: "text-ink-2", Icon: IconPlus, testId: "rail-running-not-added" },
  idle: { label: "Inactive apps", tone: "text-ink-2", Icon: IconHistory, testId: "rail-inactive-apps" },
};

/** A fold chevron, as the Project lens's (FoldButton). */
export function Chevron({ open }: { open: boolean }): JSX.Element {
  return (
    <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" className={"ml-auto shrink-0 text-muted transition-transform motion-reduce:transition-none " + (open ? "rotate-90" : "")}>
      <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

/** A link line under a place ("+3 quiet", "Last 3 inactive" with its
 * chevron, "Show 4 more"). */
export function RailLink({ onClick, testId, expanded, children }: { onClick: () => void; testId: string; expanded?: boolean; children: ReactNode }): JSX.Element {
  return (
    <button type="button" aria-expanded={expanded} onClick={onClick} data-rail-nav={expanded === undefined ? undefined : true} data-testid={testId} className="ml-3 flex h-[24px] items-center gap-1 rounded-md px-2.5 text-label text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
      {children}
      {expanded !== undefined && <Chevron open={expanded} />}
    </button>
  );
}

/** A section's head: a click folds it in place, and unfolds it. */
export function SectionHead({ id, count, open, onToggle, extra }: { id: SectionId; count: number; open: boolean; onToggle: () => void; extra?: ReactNode }): JSX.Element {
  const { label, tone, Icon, testId } = SECTION[id];
  return (
    <div className="@container relative flex h-[30px] items-end px-0.5">
      <h3 className={"flex min-w-0 flex-1 text-meta font-semibold " + tone} data-testid={testId}>
        <button type="button" aria-expanded={open} data-rail-nav data-testid="rail-section-fold" onClick={onToggle} className="flex min-h-6 w-full items-center gap-1.5 rounded-md px-1.5 outline-none hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60">
          <Icon width={13} height={13} aria-hidden="true" />
          {label}
          <span className="tabular-nums">{count}</span>
          <Chevron open={open} />
        </button>
      </h3>
      {/* D-47: Open now's order menu, on its head, before the fold chevron
          (which stays at the end, as Needs you's does). */}
      {extra && <div className="absolute bottom-0 right-7 flex h-6 max-w-[55%] items-center">{extra}</div>}
    </div>
  );
}

const ALSO: { key: keyof AlsoCounts; show: ShowKey; words: string }[] = [
  { key: "ended", show: "ended", words: "ended today" },
  { key: "inactive", show: "inactive", words: "inactive" },
  { key: "background", show: "background", words: "background" },
];

/** "Also: n ended today · n inactive · n background · History ›": each
 * part turns its view on (and off). */
export function AlsoLine({ also, hidden, offRail = [], openApp }: { also: AlsoCounts; hidden: HiddenApp[]; offRail?: OffRail[]; openApp?: (key: string) => void }): JSX.Element {
  const view = useRailView();
  // 24 px targets (WCAG 2.5.8), the line wrapping when it must.
  const link = "inline-flex min-h-6 items-center rounded px-0.5 outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";
  return (
    <p className="flex min-h-[36px] flex-wrap items-center gap-x-1 px-2 pt-2 text-label text-muted" data-testid="rail-also">
      <span>Also:</span>
      {ALSO.filter((part) => also[part.key] > 0).map((part) => (
        <Fragment key={part.key}>
          <button type="button" aria-pressed={view.show[part.show]} data-testid={`rail-also-${part.key}`} onClick={() => showInRail(part.show, !view.show[part.show])} className={link + (view.show[part.show] ? " text-accent-hi" : "")}>
            {also[part.key]} {part.words}
          </button>
          <span aria-hidden="true">·</span>
        </Fragment>
      ))}
      {hidden.map((part) => (
        <Fragment key={part.app}>
          <button type="button" data-testid="rail-also-hidden" className={link} title="Settings › Desktop apps" onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "settings" } }))}>
            {part.count} in {part.app} hidden · Settings
          </button>
          <span aria-hidden="true">·</span>
        </Fragment>
      ))}
      {/* AJ-E3204: an App kept off the rail by its own Settings, and the way back. */}
      {offRail.map((part) => (
        <Fragment key={part.key}>
          <button type="button" data-testid="rail-also-off" className={link} title={`${part.name} isn't shown in the rail: open its Settings`} onClick={() => openApp?.(part.key)}>
            {part.short} off the rail{part.open > 0 ? ` (${part.open} open)` : ""}
          </button>
          <span aria-hidden="true">·</span>
        </Fragment>
      ))}
      <button type="button" data-testid="rail-also-history" className={link} onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history" } }))}>
        History ›
      </button>
    </p>
  );
}

/** A rail search that finds nothing (persona D3 on rc33: "login" left
 * only the Also line, with no word of why): says so, and offers History's
 * search, which reaches every ended session and what was said in them.
 * `words` is what was typed; History is searched for `text`, the plain
 * words (an `is:` or `app:` is the rail's own). */
export function RailNoMatch({ words, text, children }: { words: string; text: string; children?: ReactNode }): JSX.Element {
  return (
    <div className="flex flex-col items-start gap-1.5 px-3 py-4" data-testid="rail-no-match">
      <p className="text-label font-medium text-ink">{words ? <>No open or recent session matches “{words}”</> : "No sessions match the filters"}</p>
      {text !== "" && (
        <button
          type="button"
          onClick={() => searchHistory(text)}
          className="-mx-1 rounded-md px-1 text-left text-label text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="rail-no-match-history"
        >
          Search all history for “{text}” ›
        </button>
      )}
      {children}
    </div>
  );
}

/** The lines that draw no session: all a search that matched nothing leaves. */
const NOT_A_MATCH: ReadonlySet<Line["kind"]> = new Set(["section", "label", "also"]);

/** The rail by place's `RailNoMatch`: drawn when `query` searches and
 * left no session among `lines` (only heads and the Also line), while
 * there is `anything` to find. */
export function RailSearchFoundNothing({ query, anything, lines }: { query: string; anything: boolean; lines: readonly Line[] }): JSX.Element | null {
  const parsed = parseQuery(query);
  if (!anything || !isSearching(parsed) || lines.some((line) => !NOT_A_MATCH.has(line.kind))) return null;
  return <RailNoMatch words={query.trim()} text={parsed.text} />;
}

/** The nearest box that scrolls `element` (the list's, wherever the rail
 * puts its scrolling). */
/** ↑/↓ past the first or last line drawn of a windowed list: the list
 * scrolls a row, is drawn there at once, and focus moves on. The scroll's
 * own event waits for a frame; keys pressed before it scrolled the list
 * away from the focus, and it was lost (0.4.1). */
export function stepPastEdge(list: HTMLElement, box: HTMLElement, step: 1 | -1, row: number, redraw: () => void): void {
  box.scrollTop += step * row;
  flushSync(redraw);
  moveLine(list, step);
}

export function scrollerOf(element: HTMLElement): HTMLElement | null {
  for (let at = element.parentElement; at; at = at.parentElement) {
    if (at.scrollHeight > at.clientHeight && /auto|scroll/.test(getComputedStyle(at).overflowY)) return at;
  }
  return null;
}

