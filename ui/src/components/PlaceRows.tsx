/**
 * AJ-E2104 — the new rail's rows and place headers.
 *
 * - A **Needs you** row: the app's icon with the profile's badge, the
 *   title in bold, "project · app · profile", and why it needs you in
 *   amber.
 * - An **Open now** row, under its place: indented, with a left edge in
 *   the profile's colour, a state dot, the title, "project · viewed N min
 *   ago", and the state word in its colour. Runway shows only when low.
 * - A **place header**: the app's icon and name, the profile chip, a
 *   count, and Open, which brings the running app forward (never
 *   launches it).
 *
 * Labels lead with the state (screen readers hear "Asks you, …" first);
 * a click or Enter goes to it when it's open in another app, and opens
 * its page otherwise; right-click, ⋯ or ⇧F10 opens its menu. Message
 * text never shows here (D-37).
 */
import type { JSX, ReactNode } from "react";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { PlaceIcon, AppIcon, ProfileChip } from "./PlaceMarks";
import { IconUpRight } from "./icons-places";
import { lowLeftWords } from "./indicator";
import { shortHeavy, useHeavyNote } from "../lib/machine";
import { STATE_TONE, stateWord, type RailItem, type RailPlace } from "../lib/rail-items";
import { agoWords } from "../lib/time-words";
import { markSeen } from "../lib/seen";
import { profileColor } from "../lib/profile-marks";

/** Why it needs you, in a few words. */
export function needWord(item: RailItem): string {
  switch (item.need) {
    case "limit":
      return "Limit reached";
    case "failed":
      return "Stopped with an error";
    case "stuck":
      return "Probably stuck";
    case "review":
      return stateWord("done_review");
    default:
      return stateWord(item.state, item.probably);
  }
}

/** Where it is, in words: "project · Claude · Work". */
export function whereWords(item: RailItem): string {
  return [item.project, item.place.app, item.place.profile].filter(Boolean).join(" · ");
}

/** Its primary: Go to it when open in another app, its page otherwise. */
export function usePrimary(item: RailItem, onSelect: (id: string) => void): () => void {
  const host = useActionHost();
  return () => {
    const c = item.conversation;
    if (item.session) onSelect(item.session.id);
    else if (c) {
      markSeen(c.account_id, c.id);
      if (c.live && host.goToConversation) void host.goToConversation(c);
      else host.openConversation?.(c);
    }
  };
}

interface RowProps {
  item: RailItem;
  selected: boolean;
  onSelect: (id: string) => void;
  now: number;
  /** "Sub-agents" is on in the View: the row says how many it runs. */
  subagents?: boolean;
}

function RowShell({ item, selected, onSelect, label, children, className, style, indent = false }: RowProps & { label: string; children: ReactNode; className: string; style?: React.CSSProperties; indent?: boolean }): JSX.Element {
  const primary = usePrimary(item, onSelect);
  // One hook either way: the session's menu, or the conversation's.
  const menu = useItemMenu(item.session ? "session" : "conversation", (item.session ?? item.conversation) as never, item.title);
  return (
    <div className={"group relative " + (indent ? "ml-3" : "")} {...menu.triggerProps}>
      <button
        type="button"
        data-rail-item={item.key}
        data-instance-id={item.session?.id}
        data-conversation-id={item.conversation?.id}
        aria-current={selected ? "true" : undefined}
        data-rail-selected={selected ? "true" : undefined}
        onClick={primary}
        style={style}
        className={
          "relative flex w-full min-w-0 items-center gap-2 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface " : "hover:bg-surface/60 ") +
          className
        }
      >
        <span className="sr-only">{label}</span>
        {children}
      </button>
      {menu.button({ label: `${item.session ? "Session" : "Conversation"} actions for ${item.title}`, className: "absolute bottom-0 left-0" })}
      {menu.element}
    </div>
  );
}

/** A Needs you row. */
export function NeedRow(props: RowProps): JSX.Element {
  const { item } = props;
  const word = needWord(item);
  const where = whereWords(item);
  return (
    <RowShell {...props} label={`${word}, ${item.title}, ${where}`} className="h-[44px] rounded-lg py-1.5 pl-2 pr-2">
      <span aria-hidden="true" className="flex min-w-0 flex-1 items-center gap-2.5">
        <PlaceIcon place={item.place} size={22} />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-[12.5px] font-semibold text-ink">{item.title}</span>
          <span className="truncate text-[10.5px] text-muted">{where}</span>
        </span>
        <span className="shrink-0 text-[10.5px] font-semibold text-warning" data-testid="need-word">
          {word}
        </span>
      </span>
    </RowShell>
  );
}

/** When it was last looked at or did something, in words. */
function whenWords(item: RailItem, now: number): string {
  if (item.viewedMs !== null) return `viewed ${agoWords(item.viewedMs, now)}`;
  return item.lastMs > 0 ? agoWords(item.lastMs, now) : "";
}

/** An Open now row under its place. */
export function PlaceRow(props: RowProps & { edge: string | null }): JSX.Element {
  const { item, now, edge, subagents } = props;
  const word = stateWord(item.state, item.probably);
  const tone = STATE_TONE[item.state];
  const low = item.session ? lowLeftWords(item.session) : null;
  const heavyNote = useHeavyNote(item.session?.id ?? "");
  const heavy = heavyNote ? shortHeavy(heavyNote) : null;
  const line = [item.project, whenWords(item, now)].filter(Boolean).join(" · ");
  const also = item.alsoIn.length > 0 ? `also open in ${item.alsoIn.join(" and ")}` : null;
  const helpers = subagents && item.subagents > 0 ? `+${item.subagents} ${item.subagents === 1 ? "agent" : "agents"}` : null;
  return (
    <RowShell
      {...props}
      label={[word, item.title, line, also, low, heavyNote ? `working your Mac hard: ${heavyNote}` : null, helpers].filter(Boolean).join(", ")}
      indent
      className="h-[40px] rounded-r-lg border-l-2 py-1 pl-2.5 pr-2"
      style={{ borderLeftColor: edge ?? "var(--color-hairline)" }}
    >
      <span aria-hidden="true" className="flex min-w-0 flex-1 items-center gap-2">
        <span className={"size-1.5 shrink-0 rounded-full " + tone.dot + (item.state === "working" ? " animate-breathe" : "")} />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-[12.5px] font-semibold text-ink">{item.title}</span>
          <span className="truncate text-[10.5px] text-muted">
            {line}
            {also && <span data-testid="row-also"> · {also}</span>}
            {helpers && <span> · {helpers}</span>}
          </span>
        </span>
        <span className="flex shrink-0 flex-col items-end">
          <span className={"text-[10.5px] font-medium " + tone.text} data-testid="state-word">
            {word}
          </span>
          {low && <span className="text-[10px] text-warning">{low}</span>}
          {!low && heavy && <span className="text-[10px] text-warning" data-testid="rail-heavy">{heavy}</span>}
        </span>
      </span>
    </RowShell>
  );
}

/** A place's header: app, profile, count, Open; or, folded, "Work · 4 idle". */
export function PlaceHeader({
  place,
  label,
  count,
  needs,
  folded,
  onToggle,
  onOpen,
  shortcut,
  children,
}: {
  place: RailPlace | null;
  label: string;
  count: number;
  needs: number;
  folded: string | null;
  onToggle: () => void;
  onOpen?: () => void;
  /** Its ⌘ digit, when it has one. */
  shortcut?: number;
  /** A header of another lens (an account's avatar). */
  children?: ReactNode;
}): JSX.Element {
  const name = [label, place?.profile].filter(Boolean).join(" · ");
  return (
    <div className="flex h-[30px] min-w-0 items-center gap-1 pl-1 pr-1" data-testid="place-header" data-place={place?.key}>
      <button
        type="button"
        onClick={onToggle}
        data-rail-nav
        aria-expanded={folded === null}
        aria-keyshortcuts={shortcut ? `Meta+${shortcut}` : undefined}
        className="flex min-w-0 flex-1 items-center gap-1.5 rounded-md px-1 py-1 text-left outline-none hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        <span className="sr-only">{folded ?? `${name}, ${count} open${needs > 0 ? `, ${needs} need you` : ""}`}</span>
        <span aria-hidden="true" className="flex min-w-0 flex-1 items-center gap-1.5">
          {children ?? (place && <AppIcon place={place} size={16} />)}
          <span className="shrink-0 truncate text-[11.5px] font-semibold text-ink">{label}</span>
          {place?.profile && <ProfileChip name={place.profile} />}
          {folded !== null && <span className="min-w-0 truncate text-[10.5px] text-muted">{folded.slice(folded.indexOf("·") + 1).trim()}</span>}
          {needs > 0 && <span className="shrink-0 rounded-full bg-warning-soft px-1 text-[10px] font-semibold text-warning">{needs}</span>}
          {folded === null && <span className="shrink-0 text-[10px] tabular-nums text-faint">{count}</span>}
        </span>
      </button>
      {onOpen && (
        <button
          type="button"
          onClick={onOpen}
          aria-label={`Open ${name}`}
          title={shortcut ? `Bring it forward (⌘${shortcut})` : "Bring it forward"}
          data-testid="place-open"
          className="flex shrink-0 items-center gap-0.5 rounded-md border border-hairline px-1.5 py-0.5 text-[10.5px] font-medium text-muted outline-none hover:border-accent/40 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          Open
          <IconUpRight width={11} height={11} />
        </button>
      )}
    </div>
  );
}

/** The left edge a place's rows carry: its profile's colour. */
export function edgeOf(place: RailPlace | null): string | null {
  return place?.profile ? profileColor(place.profile) : null;
}
