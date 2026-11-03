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
  layout?: ShellLayout;
  /** What sits under the lines (History's matching words). */
  below?: ReactNode;
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
  slim = false,
  menuKind,
  data,
  buttonData,
}: ShellProps): JSX.Element {
  const primary = usePrimary(item, onSelect);
  // One hook either way: the session's menu, or the conversation's. A
  // session another app runs is pinned in the rail's own record (D-47).
  const local = item.session ? undefined : { pinConversation: () => togglePin(item.key), isConversationPinned: () => item.pinned };
  const menu = useItemMenu(menuKind ?? (item.session ? "session" : "conversation"), (item.session ?? item.conversation) as never, item.title, local);
  const where = goToWhere(item);
  const list = layout === "list";
  // Needs you: its ↗ always shows; elsewhere on hover and focus.
  const shown = item.need !== null && !tools && !list;
  const goTo = where === null ? null : menu.goTo({ where, shown, className: tools || list ? "" : "absolute right-1 top-1/2 -translate-y-1/2" });
  // AJ-E1502: the pin, on hover and focus, runs the menu's own Pin.
  const pin = tools ? menu.actions.find((action) => action.id === "pin" && !action.reason) : undefined;
  // A page's row is named as its lists always named it.
  const menuLabel = list ? `Actions for ${item.title}` : `${item.session ? "Session" : "Conversation"} actions for ${item.title}`;
  return (
    <div
      className={"group relative " + (list ? "@container " : "") + (indent ? "ml-3" : "") + (arrange?.dragging ? " opacity-50" : "") + (arrange?.drop ? DROP[arrange.drop] : "")}
      data-rail-row={list ? undefined : ""}
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
          (list ? "group-hover:bg-surface-2 focus-visible:ring-inset " : selected ? "bg-surface " : "group-hover:bg-surface/60 ") +
          shape
        }
      >
        <span className="sr-only">{label}</span>
      </button>
      <div style={style} className={"pointer-events-none relative flex w-full min-w-0 items-center " + (list ? (slim || !goTo ? "pr-10 " : "pr-[4.25rem] ") : goTo && shown ? "pr-9 " : "pr-2 ") + shape + " " + className}>
        {children}
      </div>
      {below}
      {list ? (
        // A page's row: ↗ and ⋯ in their own gutter, never over the state;
        // one with no ↗ (an ended session) keeps only the ⋯'s room.
        <span className="absolute right-2 top-1/2 z-10 flex -translate-y-1/2 items-center gap-1" data-testid="row-tools">
          {goTo && !slim && <span className={ON_HOVER}>{goTo}</span>}
          {menu.button({ label: menuLabel })}
        </span>
      ) : tools ? (
        <span className={"absolute right-1 top-1/2 z-10 flex -translate-y-1/2 items-center gap-0.5 rounded-md " + (arrange?.dragging ? "" : ON_HOVER)} data-testid="row-tools">
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
      {!list && menu.button({ label: menuLabel, className: "absolute bottom-0 left-0" })}
      {menu.element}
    </div>
  );
}
