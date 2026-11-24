/**
 * AJ-E2104, D-47 and E31 (AJ-E3103) — what makes a session's row a way
 * in, the same wherever it shows (the rail, Home, every page's lists):
 *
 * - a button the size of the row: a click or ↩ opens the session's page
 *   here, whichever app runs it (↑↓ move between rows);
 * - the visible lines over it, pointer-free but for the project's name
 *   (its own way in: the project's page);
 * - its ↗ (Go to it, ⌘↩: the app and profile it runs in), its ⋯ (the
 *   menu, also on a right-click or ⇧F10), and in the rail's Your order
 *   the drag handle and the pin, on hover and focus.
 *
 * Labels lead with the state (screen readers hear "Asks you, …" first).
 * Message text never shows here (D-37).
 */
import type { CSSProperties, JSX, ReactNode } from "react";
import { useActionHost, useItemMenu } from "../ItemMenu";
import { ActionIcon } from "../ActionIcon";
import { DROP, Grip, type Arrangeable } from "../RowTools";
import { goToWhere, type RailItem } from "../../lib/rail-items";
import { markSeen } from "../../lib/seen";
import { togglePin } from "../../lib/rail-order-store";
import { SELECTED_FILL } from "../controls/selected";

/** Opens a row's page here (a session's, or a conversation's another app runs). */
export function useOpenItem(onSelect: (id: string) => void): (item: RailItem) => void {
  const host = useActionHost();
  return (item) => {
    const c = item.conversation;
    if (item.session) onSelect(item.session.id);
    else if (c) {
      markSeen(c.account_id, c.id);
      host.openConversation?.(c);
    }
  };
}

/** Its primary (a click, ↩): its page here, whichever app runs it. */
export function usePrimary(item: RailItem, onSelect: (id: string) => void): () => void {
  const open = useOpenItem(onSelect);
  return () => open(item);
}

/** Where its tools sit: over the row's end in the rail, in a gutter of
 * their own in a page's list. */
export type ShellLayout = "rail" | "list";

export interface ShellProps {
  item: RailItem;
  selected: boolean;
  onSelect: (id: string) => void;
  /** What the row says, for a screen reader (the lines are hidden from it). */
  label: string;
  children: ReactNode;
  className: string;
  /** Rounds both layers. */
  shape: string;
  style?: CSSProperties;
  indent?: boolean;
  arrange?: Arrangeable | null;
  /** The rail's Your order: the handle and the pin on hover and focus. */
  tools?: boolean;
  /** 0.3.9: a session started from another: one step in per level. */
  depth?: number;
  layout?: ShellLayout;
  /** What sits under the lines (History's matching words). */
  below?: ReactNode;
  /** The rail: its gutter's mark (the state's dot), and what its end says
   * at rest (its state's word), swapped for its tools on hover and focus. */
  lead?: ReactNode;
  rest?: ReactNode;
  /** A list of ended rows (Home's): its gutter holds the ⋯ alone. */
  slim?: boolean;
  /** Which menu it has: an ended session's in a list is its history
   * entry's (AJ-E1537: Replay, Archive, Delete). */
  menuKind?: "session" | "history" | "conversation";
  /** The row's own attributes (a list's test ids, Home's keys). */
  data?: Record<string, string | undefined>;
  /** The button's own attributes. */
  buttonData?: Record<string, string | undefined>;
}

/** Shown under the pointer, or when the keyboard is in the row: never
 * left up by the focus a click leaves on the row (the maintainer's
 * report, 2026-11-03: the selected row kept its tools). */
const ON_HOVER = "opacity-0 transition-opacity group-has-[:focus-visible]:opacity-100 group-hover:opacity-100 motion-reduce:transition-none";

/**
 * AJ-E1554 R1: a rail row's fixed slots. Its gutter holds the state's dot,
 * or (Your order) the grip in the same place on hover and focus; its end
 * holds the state's word, which hover and focus swap for its tools (pin,
 * ↗, ⋯) in the same box. Both of a slot's layers are always drawn, in one
 * grid cell, so the slot is as wide at rest as under the pointer: the
 * title never moves, nothing covers it, the dot never moves (the 0.5.0
 * Nightly's "actions over the title, a handle under the dot").
 */
const SLOT = "grid shrink-0 items-center [&>*]:col-start-1 [&>*]:row-start-1";
/** The layer at rest: gone under the pointer or the keyboard. */
const AT_REST = "transition-opacity group-has-[:focus-visible]:opacity-0 group-hover:opacity-0 motion-reduce:transition-none";

export function RowShell({
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
  layout = "rail",
  below,
  lead,
  rest,
  slim = false,
  menuKind,
  data,
  buttonData,
  depth = 0,
}: ShellProps): JSX.Element {
  const primary = usePrimary(item, onSelect);
  // One hook either way: the session's menu, or the conversation's. A
  // session another app runs is pinned in the rail's own record (D-47).
  const local = item.session ? undefined : { pinConversation: () => togglePin(item.key), isConversationPinned: () => item.pinned };
  const menu = useItemMenu(menuKind ?? (item.session ? "session" : "conversation"), (item.session ?? item.conversation) as never, item.title, local);
  const where = goToWhere(item);
  const list = layout === "list";
  // Needs you: its ↗ on hover, focus or the selected row (D-117, R10);
  // elsewhere on hover and focus. The G key and the menu still go there.
  const shown = item.need !== null && !tools && !list && selected;
  // The rail's sits in its row's end (shown with its tools); a page's list keeps its own look.
  const goTo = where === null ? null : menu.goTo({ where, shown: !list });
  // AJ-E1502: the pin, on hover and focus, runs the menu's own Pin.
  // D-47: Your order's grip, in the gutter in the dot's place.
  const grip = tools && arrange?.enabled ? arrange : null;
  const pin = tools ? menu.actions.find((action) => action.id === "pin" && !action.reason) : undefined;
  // A page's row is named as its lists always named it.
  const menuLabel = list ? `Actions for ${item.title}` : `${item.session ? "Session" : "Conversation"} actions for ${item.title}`;
  return (
    <div
      // `tone-host`: hovered or focused, its App's mark is in full colour (D-117).
      className={"tone-host group relative " + (list ? "@container " : "") + (indent ? "ml-3" : "") + (arrange?.dragging ? " opacity-50" : "") + (arrange?.drop ? DROP[arrange.drop] : "")}
      data-rail-row={list ? undefined : ""}
      // A session started from another: one step in per level, under it.
      style={depth > 0 ? { marginLeft: `${0.75 + depth}rem` } : undefined}
      data-depth={depth > 0 ? depth : undefined}
      data-order-row={arrange ? item.key : undefined}
      data-order-group={arrange?.group}
      {...data}
      {...menu.triggerProps}
    >
      <button
        type="button"
        data-rail-item={list ? undefined : item.key}
        aria-keyshortcuts={arrange?.enabled ? "Alt+Meta+ArrowUp Alt+Meta+ArrowDown" : undefined}
        data-instance-id={item.session?.id}
        data-conversation-id={item.conversation?.id}
        aria-current={selected ? "true" : undefined}
        data-rail-selected={selected ? "true" : undefined}
        onClick={primary}
        {...buttonData}
        className={
          "absolute inset-0 w-full text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          // D-117 (selection): the selected row is SELECTED_FILL (a 3:1 edge on
          // the raised fill); its title turns semibold ink (SessionRow).
          (list ? "group-hover:bg-surface-2 focus-visible:ring-inset " : selected ? SELECTED_FILL + " " : "group-hover:bg-surface/60 ") +
          shape
        }
      >
        <span className="sr-only">{label}</span>
      </button>
      {list ? (
        <>
          <div style={style} className={"pointer-events-none relative flex w-full min-w-0 items-center " + (slim || !goTo ? "pr-10 " : "pr-[4.25rem] ") + shape + " " + className}>
            {children}
          </div>
          {below}
          {/* A page's row: ↗ and ⋯ in their own gutter, never over the state;
              one with no ↗ (an ended session) keeps only the ⋯'s room. */}
          <span className="absolute right-2 top-1/2 z-10 flex -translate-y-1/2 items-center gap-1" data-testid="row-tools">
            {goTo && !slim && <span className={ON_HOVER}>{goTo}</span>}
            {menu.button({ label: menuLabel })}
          </span>
        </>
      ) : (
        <>
          <div style={style} className={"pointer-events-none relative flex w-full min-w-0 items-center pr-1 " + shape + " " + className} data-testid="row-slots">
            {/* AJ-E1554 R1: the gutter: the dot, or the grip in its place. */}
            <span className={SLOT + " w-4 justify-items-center"} data-testid="row-gutter">
              <span className={"flex " + (grip ? (grip.dragging ? "opacity-0" : AT_REST) : "")}>{lead}</span>
              {grip && <span className={"pointer-events-auto flex " + (grip.dragging ? "" : ON_HOVER)}><Grip arrange={grip} label={item.title} /></span>}
            </span>
            {children}
            {/* Its end: the state at rest, its tools under the pointer. */}
            <span className={SLOT + " justify-items-end"} data-testid="row-end">
              <span className={"flex items-center gap-2 " + (shown ? "opacity-0" : AT_REST)}>{rest}</span>
              <span className={"flex items-center [&>*]:pointer-events-auto " + (shown ? "" : ON_HOVER)} data-testid="row-tools">
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
                    // PAT-SURF: a tool's hover is a tone, never a line.
                    className={"flex size-6 shrink-0 items-center justify-center rounded-md outline-none transition-colors hover:bg-hover hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60 " + (item.pinned ? "text-accent-hi" : "text-muted")}
                  >
                    <ActionIcon name="pin" size={13} />
                  </button>
                )}
                {goTo}
                {menu.button({ label: menuLabel, shown: true })}
              </span>
            </span>
          </div>
          {below}
        </>
      )}
      {menu.element}
    </div>
  );
}
