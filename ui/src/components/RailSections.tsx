/**
 * AJ-E2103 — the rail's order, before the new rail (AJ-E2104): what needs
 * you first, then what is open now, then one line for the rest, "Also:
 * n ended today · n inactive · n background · History ›", each part
 * turning its view on. Nothing hidden is silent: a view that is off
 * leaves its count on that line.
 */
import { Fragment, useState, type JSX, type ReactNode } from "react";
import { ConversationRow, asksYou, railSlice, type OutsideScope } from "./OutsideConversations";
import { useActionHost } from "./ItemMenu";
import { outside, useConversations } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { InstanceInfo } from "../lib/bridge";
import { isSnoozed, type RowGroup, type SnoozeState } from "../lib/railview";

const ENDED = new Set(["finished", "stopped", "failed", "handed_over"]);

/** The default view's order for AI Juggler's own sessions: what needs
 * you (a snoozed one waits in its project, as before), what is open (the
 * rest, under their projects), and what ended today (behind "Also:"; an
 * older one is in History). A pinned session stays where the user put it. */
export interface RailOrder {
  needs: InstanceInfo[];
  endedToday: InstanceInfo[];
  /** The rows of a project's group that show under Open now. */
  shown: (rows: InstanceInfo[]) => InstanceInfo[];
}

export function railOrder(sorted: InstanceInfo[], snooze: SnoozeState, now: number): RailOrder {
  const needs = sorted.filter((row) => row.needs_attention !== null && !isSnoozed(row, snooze, now));
  const folded = (row: InstanceInfo) => ENDED.has(row.status) && !row.pinned && !needs.includes(row);
  const since = startOfDay(now);
  return {
    needs,
    endedToday: sorted.filter((row) => folded(row) && (row.last_activity_ms ?? row.started_at_ms ?? 0) >= since),
    shown: (rows) => rows.filter((row) => !needs.includes(row) && !folded(row)),
  };
}

/** Open now's groups: a project shows while something of it is open
 * (its header still sums up all its sessions), and a pinned one stays
 * where the user put it (AJ-E1502). */
export function openGroups(order: RailOrder | null, groups: RowGroup[], all: ConversationInfo[], scope: OutsideScope, pinned: ReadonlySet<string> = new Set()): RowGroup[] {
  if (!order) return groups;
  return groups.filter((group) => pinned.has(group.id) || order.shown(group.rows).length > 0 || railSlice(all, group.id, scope).shown.length > 0);
}

export type AlsoView = "ended" | "inactive" | "background";
const VIEWS: AlsoView[] = ["ended", "inactive", "background"];
const WORDS: Record<AlsoView, string> = { ended: "ended today", inactive: "inactive", background: "background" };
const TITLES: Record<AlsoView, string> = { ended: "Ended today", inactive: "Inactive", background: "Background" };

/** The start of `now`'s day, local time. */
function startOfDay(now: number): number {
  const day = new Date(now);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

/** What each part of the "Also:" line holds, of the sessions AI Juggler
 * didn't start: ended today; in an app's sidebar with no process on it;
 * open but headless. */
export function alsoParts(all: ConversationInfo[], now: number): Record<AlsoView, ConversationInfo[]> {
  const mine = outside(all);
  const since = startOfDay(now);
  return {
    ended: mine.filter((c) => !c.live && !c.inactive && (c.last_activity_ms ?? 0) >= since),
    inactive: mine.filter((c) => !c.live && c.inactive),
    background: mine.filter((c) => c.live && c.background),
  };
}

/** A section's label; with `onToggle`, a click folds the section in
 * place and unfolds it (Needs you, Open now). */
function SectionLabel({ children, testId, open, onToggle }: { children: ReactNode; testId?: string; open?: boolean; onToggle?: () => void }): JSX.Element {
  const words = "text-meta font-semibold text-muted";
  if (!onToggle) {
    return (
      <p className={"px-3 pb-0.5 pt-1.5 " + words} data-testid={testId}>
        {children}
      </p>
    );
  }
  return (
    <h3 className="px-1 pt-1" data-testid={testId}>
      <button type="button" aria-expanded={open} data-testid="rail-section-fold" onClick={onToggle} className={"flex min-h-6 w-full items-center gap-1 rounded-md px-2 text-left outline-none hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60 " + words}>
        {children}
        <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" className={"ml-auto shrink-0 text-muted transition-transform motion-reduce:transition-none " + (open ? "rotate-90" : "")}>
          <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
    </h3>
  );
}

/** The fold key of the Project lens's Open now (kept with the groups'). */
export const OPEN_NOW = "rail-open-now";

/** "Open now", above the projects' open sessions: folds them in place. */
export function OpenNowLabel({ open, onToggle }: { open: boolean; onToggle: () => void }): JSX.Element {
  return (
    <SectionLabel testId="rail-open-now" open={open} onToggle={onToggle}>
      Open now
    </SectionLabel>
  );
}

interface RowProps {
  selectedId: string | null;
  density: "comfortable" | "compact";
  now: number;
}

function Rows({ rows, selectedId, density, now }: RowProps & { rows: ConversationInfo[] }): JSX.Element {
  const host = useActionHost();
  return (
    <>
      {rows.map((conversation) => (
        <ConversationRow
          key={`${conversation.account_id}/${conversation.id}`}
          conversation={conversation}
          selected={selectedId === conversation.id}
          density={density}
          now={now}
          onOpen={(c) => host.openConversation?.(c)}
        />
      ))}
    </>
  );
}

/** Needs you: AI Juggler's sessions that need you, then the ones other
 * apps say ask you or finished something to look at. Never grouped. */
export function NeedsYouSection({ rows, renderRow, ...props }: RowProps & { rows: InstanceInfo[]; renderRow: (row: InstanceInfo) => ReactNode }): JSX.Element | null {
  const { all } = useConversations();
  const [open, setOpen] = useState(true);
  const asking = outside(all).filter(asksYou);
  if (rows.length === 0 && asking.length === 0) return null;
  return (
    <section aria-label="Needs you" data-testid="rail-needs-you" className="mb-1 flex flex-col gap-0.5">
      <SectionLabel open={open} onToggle={() => setOpen((now) => !now)}>
        Needs you <span className="tabular-nums">{rows.length + asking.length}</span>
      </SectionLabel>
      {open && rows.map(renderRow)}
      {open && <Rows rows={asking} {...props} />}
    </section>
  );
}

/** The "Also:" line, and the views it turns on; `ended` and `renderRow`
 * are AI Juggler's own sessions that ended today. */
export function AlsoLine({ ended, renderRow, ...props }: RowProps & { ended: InstanceInfo[]; renderRow: (row: InstanceInfo) => ReactNode }): JSX.Element {
  const { all } = useConversations();
  const [shown, setShown] = useState<ReadonlySet<AlsoView>>(new Set());
  const parts = alsoParts(all, props.now);
  const count = (view: AlsoView) => parts[view].length + (view === "ended" ? ended.length : 0);
  const toggle = (view: AlsoView) =>
    setShown((current) => {
      const next = new Set(current);
      if (next.has(view)) next.delete(view);
      else next.add(view);
      return next;
    });
  const link = "rounded px-0.5 outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";
  return (
    <div className="mt-1 flex flex-col gap-0.5" data-testid="rail-also">
      <p className="flex flex-wrap items-center gap-x-1 px-3 py-1 text-label text-muted">
        <span>Also:</span>
        {VIEWS.filter((view) => count(view) > 0).map((view) => (
          <Fragment key={view}>
            <button type="button" aria-pressed={shown.has(view)} data-testid={`rail-also-${view}`} onClick={() => toggle(view)} className={link + (shown.has(view) ? " text-accent-hi" : "")}>
              {count(view)} {WORDS[view]}
            </button>
            <span aria-hidden="true">·</span>
          </Fragment>
        ))}
        <button type="button" data-testid="rail-also-history" className={link} onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history" } }))}>
          History ›
        </button>
      </p>
      {VIEWS.filter((view) => shown.has(view) && count(view) > 0).map((view) => (
        <section key={view} aria-label={TITLES[view]} data-testid={`rail-also-view-${view}`} className="flex flex-col gap-0.5">
          <SectionLabel open onToggle={() => toggle(view)}>{TITLES[view]}</SectionLabel>
          {view === "ended" && ended.map(renderRow)}
          <Rows rows={parts[view]} {...props} />
        </section>
      ))}
    </div>
  );
}
