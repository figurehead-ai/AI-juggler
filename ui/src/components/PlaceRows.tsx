/**
 * AJ-E2104 — the new rail's rows and place headers.
 *
 * - A **Needs you** row: the app's icon (D-38) with the profile's dot, the
 *   title in bold, then "project · app · profile" and why it needs you
 *   in amber; its ↗ always shows.
 * - An **Open now** row, under its place: indented, with a left edge in
 *   the profile's colour, a state dot, the title, "project · viewed N min
 *   ago", and the state word in its colour. Runway shows only when low.
 * - A **place header**: a fold chevron, the app's icon and name, the
 *   profile's badge and name, and a count (a click opens the profile's page here), and
 *   its ↗ (Open), which brings the running app forward (never launches it).
 * - D-45: in Your order, a row and a place show a drag handle on hover
 *   and focus (⌥⌘↑ and ⌥⌘↓ are its twin); a row shows its pin there too.
 *   A row that needs you stays in its place, its state in amber.
 *
 * The maintainer's review of 0.1.4: everything here is a way into AI
 * Juggler. A row's click or ↩ opens the session's page; its project's
 * name opens the project's page; its ↗ (or ⌘↩) is the one way out, to
 * the app and profile it runs in. Right-click, ⋯ or ⇧F10 open its menu,
 * which repeats these. Labels lead with the state (screen readers hear
 * "Asks you, …" first). Message text never shows here (D-37).
 */
import type { JSX, ReactNode } from "react";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { PlaceMark } from "./PlaceMarks";
import { PinMark } from "./RailProjectRows";
import { DROP, Grip, type Arrangeable } from "./RowTools";

export { PlaceHeader } from "./PlaceHeader";
export type { Arrangeable } from "./RowTools";
import { ActionIcon } from "./ActionIcon";
import { lowLeftWords } from "./indicator";
import { shortHeavy, useHeavyNote } from "../lib/machine";
import { STATE_TONE, goToWhere, needWord, stateWord, type RailItem, type RailPlace } from "../lib/rail-items";
import { agoWords } from "../lib/time-words";
import { markSeen } from "../lib/seen";
import { profileColor } from "../lib/profile-marks";
import { togglePin } from "../lib/rail-order-store";

export { needWord } from "../lib/rail-items";

/** Where it is, in words: "project · Claude · Work". */
export function whereWords(item: RailItem): string {
  return [item.project, item.place.app, item.place.profile].filter(Boolean).join(" · ");
}

/** Its primary (a click, ↩): its page here, whichever app runs it. */
export function usePrimary(item: RailItem, onSelect: (id: string) => void): () => void {
  const host = useActionHost();
  return () => {
    const c = item.conversation;
    if (item.session) onSelect(item.session.id);
    else if (c) {
      markSeen(c.account_id, c.id);
      host.openConversation?.(c);
    }
  };
}

/** A row's project, its own way in: the project's page here. Text only
 * when it has no page (a folder no project holds). */
export function RowProject({ item, rest }: { item: RailItem; rest: string }): JSX.Element {
  const host = useActionHost();
  const id = item.projectId;
  const linked = item.project !== null && id !== null && !!host.openProjectById;
  // Its 24 px target reaches down from the line, never up over the row's
  // middle (where the row's own click is).
  return (
    <span className="flex min-w-0 items-start text-label leading-[14px] text-muted">
      {linked ? (
        <button
          type="button"
          data-testid="row-project"
          aria-label={`Open project ${item.project}`}
          onClick={(event) => {
            event.stopPropagation();
            host.openProjectById!(id!);
          }}
          className="pointer-events-auto relative z-10 -mx-0.5 -mb-2.5 min-h-6 max-w-[65%] shrink-0 truncate rounded px-0.5 pb-2.5 text-left leading-[14px] text-muted outline-none hover:bg-surface-3 hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          {item.project}
        </button>
      ) : (
        item.project && (
          <span aria-hidden="true" className="max-w-[65%] shrink-0 truncate">
            {item.project}
          </span>
        )
      )}
      {rest && (
        <span aria-hidden="true" className="min-w-0 truncate whitespace-pre">
          {item.project ? ` · ${rest}` : rest}
        </span>
      )}
    </span>
  );
}

interface RowProps {
  item: RailItem;
  selected: boolean;
  onSelect: (id: string) => void;
  now: number;
  /** "Sub-agents" is on in the View: the row says how many it runs. */
  subagents?: boolean;
}


/**
 * One row: a button the size of the row (its page; ↑↓ move between
 * them), the visible line over it (pointer-free but for the project's
 * name), the ↗ and the ⋯. `shape` rounds both layers.
 */
function RowShell({
  item,
  selected,
  onSelect,
  label,
  children,
  className,
  shape,
  style,
  indent = false,
  arrange = null,
  tools = false,
}: RowProps & { label: string; children: ReactNode; className: string; shape: string; style?: React.CSSProperties; indent?: boolean; arrange?: Arrangeable | null; tools?: boolean }): JSX.Element {
  const primary = usePrimary(item, onSelect);
  // One hook either way: the session's menu, or the conversation's. A
  // session another app runs is pinned in the rail's own record (D-45).
  const local = item.session ? undefined : { pinConversation: () => togglePin(item.key), isConversationPinned: () => item.pinned };
  const menu = useItemMenu(item.session ? "session" : "conversation", (item.session ?? item.conversation) as never, item.title, local);
  const where = goToWhere(item);
  // Needs you: its ↗ always shows; elsewhere on hover and focus.
  const shown = item.need !== null && !tools;
  const goTo = where === null ? null : menu.goTo({ where, shown, className: tools ? "" : "absolute right-1 top-1/2 -translate-y-1/2" });
  // AJ-E1502: the pin, on hover and focus, runs the menu's own Pin.
  const pin = tools ? menu.actions.find((action) => action.id === "pin" && !action.reason) : undefined;
  const hover = "opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100 motion-reduce:transition-none";
  return (
    <div
      className={"group relative " + (indent ? "ml-3" : "") + (arrange?.dragging ? " opacity-50" : "") + (arrange?.drop ? DROP[arrange.drop] : "")}
      data-rail-row
      data-order-row={arrange ? item.key : undefined}
      data-order-group={arrange?.group}
      {...menu.triggerProps}
    >
      <button
        type="button"
        data-rail-item={item.key}
        aria-keyshortcuts={arrange?.enabled ? "Alt+Meta+ArrowUp Alt+Meta+ArrowDown" : undefined}
        data-instance-id={item.session?.id}
        data-conversation-id={item.conversation?.id}
        aria-current={selected ? "true" : undefined}
        data-rail-selected={selected ? "true" : undefined}
        onClick={primary}
        className={
          "absolute inset-0 w-full text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface " : "group-hover:bg-surface/60 ") +
          shape
        }
      >
        <span className="sr-only">{label}</span>
      </button>
      <div style={style} className={"pointer-events-none relative flex w-full min-w-0 items-center " + (goTo && shown ? "pr-9 " : "pr-2 ") + shape + " " + className}>
        {children}
      </div>
      {tools ? (
        <span className={"absolute right-1 top-1/2 z-10 flex -translate-y-1/2 items-center gap-0.5 rounded-md " + (arrange?.dragging ? "" : hover)} data-testid="row-tools">
          {arrange?.enabled && <Grip arrange={arrange} label={item.title} />}
          {pin && (
            <button
              type="button"
              aria-label={`Pin ${item.title}`}
              aria-pressed={item.pinned}
              title={pin.label}
              data-testid="row-pin"
              onClick={(event) => {
                event.stopPropagation();
                void pin.run();
              }}
              className={
                "flex size-6 shrink-0 items-center justify-center rounded-md border border-hairline bg-surface outline-none transition-colors hover:bg-surface-3 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (item.pinned ? "text-accent-hi" : "text-muted")
              }
            >
              <ActionIcon name="pin" size={13} />
            </button>
          )}
          {goTo}
        </span>
      ) : (
        goTo
      )}
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
    <RowShell {...props} label={`${word}, ${item.title}, ${where}`} shape="rounded-lg" className="h-[62px] items-start gap-2.5 pb-2.5 pl-2 pt-2">
      <span aria-hidden="true" className="mt-0.5 flex shrink-0">
        <PlaceMark place={item.place} profile={item.accountLabel} size={24} />
      </span>
      {/* Three lines, each its own width (the review of 0.1.4: the state
          beside the place cut both to "ai-jug… · T…"): what, why it
          needs you, where. */}
      <span className="flex min-w-0 flex-1 flex-col">
        <span aria-hidden="true" className="truncate text-meta font-semibold leading-[18px] text-ink">
          {item.title}
        </span>
        <span aria-hidden="true" className="truncate text-label font-semibold leading-4 text-warning" data-testid="need-word">
          {word}
        </span>
        <RowProject item={item} rest={[item.place.app, item.place.profile].filter(Boolean).join(" · ")} />
      </span>
    </RowShell>
  );
}

/** When it was last looked at or did something, in words. */
function whenWords(item: RailItem, now: number): string {
  if (item.viewedMs !== null) return `viewed ${agoWords(item.viewedMs, now)}`;
  return item.lastMs > 0 ? agoWords(item.lastMs, now) : "";
}

/** An Open now row under its place. One that needs you keeps its place
 * and says its state in amber (D-45); Needs you lists it as well, with
 * why. One vocabulary on every row (AJ-E2104 Req.3). */
export function PlaceRow(props: RowProps & { edge: string | null; arrange?: Arrangeable | null; tools?: boolean }): JSX.Element {
  const { item, now, edge, subagents } = props;
  const needs = item.need !== null;
  const word = stateWord(item.state, item.probably);
  const tone = needs ? { text: "text-warning", dot: "bg-warning" } : STATE_TONE[item.state];
  const low = item.session ? lowLeftWords(item.session) : null;
  const heavyNote = useHeavyNote(item.session?.id ?? "");
  const heavy = heavyNote ? shortHeavy(heavyNote) : null;
  const when = whenWords(item, now);
  const line = [item.project, when].filter(Boolean).join(" · ");
  const also = item.alsoIn.length > 0 ? `also open in ${item.alsoIn.join(" and ")}` : null;
  const helpers = subagents && item.subagents > 0 ? `+${item.subagents} ${item.subagents === 1 ? "agent" : "agents"}` : null;
  // AJ-E1502: a pinned row says so, as the Project lens's row does.
  const pinned = item.pinned;
  return (
    <RowShell
      {...props}
      label={[word, item.title, pinned ? "pinned" : null, line, also, low, heavyNote ? `working your Mac hard: ${heavyNote}` : null, helpers].filter(Boolean).join(", ")}
      indent
      shape="rounded-r-lg"
      className="h-[44px] gap-2 border-l-2 py-1 pl-2.5"
      style={{ borderLeftColor: edge ?? "var(--color-hairline)" }}
    >
      <span aria-hidden="true" data-state-dot={item.state} className={"size-1.5 shrink-0 rounded-full " + tone.dot + (item.state === "working" && !needs ? " animate-breathe" : "")} />
      {/* The title has its line to itself; its state ends the line under
          it, after the project and the time (the review of 0.1.4). */}
      <span className="flex min-w-0 flex-1 flex-col gap-px">
        <span aria-hidden="true" className="flex min-w-0 items-center gap-1">
          <span className="truncate text-meta font-semibold text-ink">{item.title}</span>
          {pinned && <PinMark />}
          {low && <span className="ml-auto shrink-0 text-label text-warning">{low}</span>}
          {!low && heavy && <span className="ml-auto shrink-0 text-label text-warning" data-testid="rail-heavy">{heavy}</span>}
        </span>
        <span className="flex min-w-0 items-center gap-2 text-label text-muted">
          <span className="flex min-w-0 flex-1 items-center">
            <RowProject item={item} rest={when} />
            {also && <span aria-hidden="true" className="shrink-0 whitespace-pre" data-testid="row-also"> · {also}</span>}
            {helpers && <span aria-hidden="true" className="shrink-0 whitespace-pre"> · {helpers}</span>}
          </span>
          <span aria-hidden="true" className={"shrink-0 text-label leading-[14px] " + (needs ? "font-semibold " : "font-medium ") + tone.text} data-testid="state-word">
            {word}
          </span>
        </span>
      </span>
    </RowShell>
  );
}

/** The left edge a place's rows carry: its profile's colour. */
export function edgeOf(place: RailPlace | null): string | null {
  return place?.profile ? profileColor(place.profile) : null;
}
