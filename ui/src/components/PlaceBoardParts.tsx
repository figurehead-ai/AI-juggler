/**
 * AJ-E2104 — the rail by place's smaller parts: a section's head (Needs
 * you, Open now with its order menu, D-47), the fold chevron and the
 * "Also:" line. `PlaceBoard` draws the list with them.
 */
import { Fragment, type JSX, type ReactNode } from "react";
import { IconAlert } from "./icons";
import { IconHistory, IconWindow } from "./icons-places";
import type { HiddenApp } from "../lib/shown-apps";
import type { AlsoCounts, SectionId } from "../lib/rail-lines";
import { showInRail, useRailView, type ShowKey } from "../lib/rail-view";

const SECTION: Record<SectionId, { label: string; tone: string; Icon: typeof IconAlert; testId: string }> = {
  needs: { label: "Needs you", tone: "text-warning", Icon: IconAlert, testId: "rail-needs-you" },
  open: { label: "Open now", tone: "text-live", Icon: IconWindow, testId: "rail-open-now" },
  // D-64: the apps with nothing open, after the open ones.
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
    <div className="relative flex h-[30px] items-end px-0.5">
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
export function AlsoLine({ also, hidden }: { also: AlsoCounts; hidden: HiddenApp[] }): JSX.Element {
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
      <button type="button" data-testid="rail-also-history" className={link} onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history" } }))}>
        History ›
      </button>
    </p>
  );
}

/** The nearest box that scrolls `element` (the list's, wherever the rail
 * puts its scrolling). */
export function scrollerOf(element: HTMLElement): HTMLElement | null {
  for (let at = element.parentElement; at; at = at.parentElement) {
    if (at.scrollHeight > at.clientHeight && /auto|scroll/.test(getComputedStyle(at).overflowY)) return at;
  }
  return null;
}

