/**
 * AJ-E2103 — the rail's order, before the new rail (AJ-E2104): what needs
 * you first, then what is open now, then one line for the rest, "Also:
 * n ended today · n inactive · n background · History ›", each part
 * turning its view on. Nothing hidden is silent: a view that is off
 * leaves its count on that line.
 */
import { Fragment, useState, type JSX, type ReactNode } from "react";
import { ConversationRow, asksYou } from "./OutsideConversations";
import { useActionHost } from "./ItemMenu";
import { outside, useConversations } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { InstanceInfo } from "../lib/bridge";
import { isSnoozed, type SnoozeState } from "../lib/railview";

/** AI Juggler's own sessions that need you now (a snoozed one waits in
 * its project, as before). */
export function needsYouRows(sorted: InstanceInfo[], snooze: SnoozeState, now: number): InstanceInfo[] {
  return sorted.filter((row) => row.needs_attention !== null && !isSnoozed(row, snooze, now));
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

function SectionLabel({ children, testId }: { children: ReactNode; testId?: string }): JSX.Element {
  return (
    <p className="px-3 pb-0.5 pt-1.5 text-[10px] font-medium uppercase tracking-[0.08em] text-muted" data-testid={testId}>
      {children}
    </p>
  );
}

/** "Open now", above the projects' open sessions. */
export function OpenNowLabel(): JSX.Element {
  return <SectionLabel testId="rail-open-now">Open now</SectionLabel>;
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
  const asking = outside(all).filter(asksYou);
  if (rows.length === 0 && asking.length === 0) return null;
  return (
    <section aria-label="Needs you" data-testid="rail-needs-you" className="mb-1 flex flex-col gap-0.5">
      <SectionLabel>Needs you</SectionLabel>
      {rows.map(renderRow)}
      <Rows rows={asking} {...props} />
    </section>
  );
}

/** The "Also:" line, and the views it turns on. */
export function AlsoLine(props: RowProps): JSX.Element {
  const { all } = useConversations();
  const [shown, setShown] = useState<ReadonlySet<AlsoView>>(new Set());
  const parts = alsoParts(all, props.now);
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
      <p className="flex flex-wrap items-center gap-x-1 px-3 py-1 text-[11px] text-muted">
        <span>Also:</span>
        {VIEWS.filter((view) => parts[view].length > 0).map((view) => (
          <Fragment key={view}>
            <button type="button" aria-pressed={shown.has(view)} data-testid={`rail-also-${view}`} onClick={() => toggle(view)} className={link + (shown.has(view) ? " text-accent-hi" : "")}>
              {parts[view].length} {WORDS[view]}
            </button>
            <span aria-hidden="true">·</span>
          </Fragment>
        ))}
        <button type="button" data-testid="rail-also-history" className={link} onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history" } }))}>
          History ›
        </button>
      </p>
      {VIEWS.filter((view) => shown.has(view) && parts[view].length > 0).map((view) => (
        <section key={view} aria-label={TITLES[view]} data-testid={`rail-also-view-${view}`} className="flex flex-col gap-0.5">
          <SectionLabel>{TITLES[view]}</SectionLabel>
          <Rows rows={parts[view]} {...props} />
        </section>
      ))}
    </div>
  );
}
