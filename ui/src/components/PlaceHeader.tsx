/**
 * AJ-E2104 — a place's header in the rail: the fold chevron, the app's
 * icon and name, the profile's badge and name, a count (a click opens the
 * profile's page here), and its ↗ (Open), which brings the running app
 * forward (never launches it). D-47: in Your order its handle shows on
 * hover and focus; ⌥⌘↑ and ⌥⌘↓ move it. 0.3.2: a right-click (or
 * Shift+F10) opens its own menu: its page, Bring it forward, Fold — never
 * the web view's (Reload). 0.3.3: a desktop app's menu also turns its Keep
 * alive on and off.
 */
import { useState, type JSX, type MouseEvent, type ReactNode } from "react";
import { MenuItem, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { ProfileBadge } from "./ProfileBadge";
import { AppIcon } from "./AppIcon";
import { FoldButton } from "./rail-groups";
import { IconUpRight } from "./icons-places";
import { IconAlert } from "./icons";
import { DROP, Grip, type Arrangeable } from "./RowTools";
import type { RailPlace } from "../lib/rail-items";

/** A place's header: the fold chevron; app, profile, count (its page);
 * Open. Folded, it reads "Work · 4 idle". */
export function PlaceHeader({
  place,
  label,
  count,
  needs,
  folded,
  onToggle,
  onOpen,
  openWords = "Bring it forward",
  onPage,
  shortcut,
  arrange = null,
  children,
  keepAlive = null,
}: {
  place: RailPlace | null;
  label: string;
  count: number;
  needs: number;
  folded: string | null;
  onToggle: () => void;
  onOpen?: () => void;
  /** What Open does, when it isn't bringing an app forward (a pool's page). */
  openWords?: string;
  /** Its page here: the profile's (Home for AI Juggler's own place). */
  onPage?: () => void;
  /** Its ⌘ digit, when it has one. */
  shortcut?: number;
  /** D-47: Your order is on: it can be dragged (its handle on hover and focus). */
  arrange?: Arrangeable | null;
  /** A header of another lens (an account's avatar). */
  children?: ReactNode;
  /** 0.3.3: a desktop app AI Juggler remembers: its Keep alive. */
  keepAlive?: { on: boolean; toggle: () => void } | null;
}): JSX.Element {
  const name = [label, place?.profile].filter(Boolean).join(" · ");
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const run = (action: () => void) => () => (setMenu(null), action());
  return (
    <div
      onContextMenu={(event: MouseEvent<HTMLDivElement>) => {
        event.preventDefault();
        setMenu(menuPoint(event));
      }}
      onKeyDown={(event) => {
        if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
          event.preventDefault();
          const box = event.currentTarget.getBoundingClientRect();
          setMenu({ x: box.left + 8, y: box.bottom + 4 });
        }
      }}
      className={"group/head flex h-[30px] min-w-0 items-center gap-0.5 rounded-md pr-1" + (arrange?.dragging ? " opacity-50" : "") + (arrange?.drop ? DROP[arrange.drop] : "")}
      data-testid="place-header"
      data-place={place?.key}
      data-order-head={arrange?.group}
    >
      <FoldButton open={folded === null} label={name} onToggle={onToggle} />
      <button
        type="button"
        onClick={onPage ?? onToggle}
        data-rail-nav
        data-testid="place-page"
        aria-keyshortcuts={[shortcut ? `Meta+${shortcut}` : null, arrange?.enabled ? "Alt+Meta+ArrowUp Alt+Meta+ArrowDown" : null].filter(Boolean).join(" ") || undefined}
        title={onPage ? `Open ${name}'s page` : undefined}
        className="flex min-h-6 min-w-0 flex-1 items-center gap-1.5 rounded-md px-1 py-1 text-left outline-none hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        <span className="sr-only">{folded ?? `${name}, ${count} open${needs > 0 ? `, ${needs} need you` : ""}`}</span>
        <span aria-hidden="true" className="flex min-w-0 flex-1 items-center gap-1">
          {children ?? (place && <AppIcon appId={place.appId} appPid={place.appPid} name={place.app} size={16} />)}
          <span className="ml-0.5 shrink-0 truncate text-label font-semibold text-ink">{label}</span>
          {/* The profile: its dot and name, beside the icon (D-38). The
              folded summary gives way first. */}
          {place?.profile && (
            <span className="flex min-w-[4ch] shrink items-center gap-[3px] text-label font-medium text-muted" data-testid="place-profile">
              <ProfileBadge name={place.profile} size={6} />
              <span className="truncate">{place.profile}</span>
            </span>
          )}
          {/* Folded, its summary ("4 idle"), unless the amber chip needs
              the room (the profile's name keeps it). */}
          {folded !== null && needs === 0 && <span className="min-w-0 shrink-[8] truncate text-label text-muted">{folded.slice(folded.indexOf("·") + 1).trim()}</span>}
          {/* What the amber count is, on the chip itself (the review of
              0.1.4: an unexplained "1"): the Needs you mark and a tooltip. */}
          {needs > 0 && (
            <span title={`${needs} here ${needs === 1 ? "needs" : "need"} you: ${needs === 1 ? "it's" : "they're"} at the top, under Needs you`} className="inline-flex shrink-0 items-center gap-0.5 rounded-full bg-warning-soft px-1.5 text-label font-semibold leading-[18px] tabular-nums text-warning" data-testid="place-needs">
              <IconAlert width={10} height={10} aria-hidden="true" />
              {needs}
            </span>
          )}
          {folded === null && <span className="shrink-0 text-label tabular-nums text-muted" title={`${count} open here`}>{count}</span>}
        </span>
      </button>
      {arrange?.enabled && (
        <span className={arrange.dragging ? "" : "opacity-0 transition-opacity group-focus-within/head:opacity-100 group-hover/head:opacity-100 motion-reduce:transition-none"}>
          <Grip arrange={arrange} label={name} />
        </span>
      )}
      {onOpen && (
        <button
          type="button"
          onClick={onOpen}
          aria-label={`Open ${name}`}
          title={shortcut ? `${openWords} (⌘${shortcut})` : openWords}
          data-testid="place-open"
          className="flex size-6 shrink-0 items-center justify-center rounded-md border border-hairline text-muted outline-none hover:border-accent/40 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          {/* Its ↗, as a row's: the way out to the app. The words are its
              name; the room goes to the place's app and profile. */}
          <IconUpRight width={12} height={12} />
        </button>
      )}
      {menu && (
        <RowMenu heading={name} ariaLabel={`Actions for ${name}`} at={menu} onClose={() => setMenu(null)} testId="place-header-menu">
          {onPage && (
            <MenuItem onClick={run(onPage)} testId="place-header-menu-page">
              Open its page
            </MenuItem>
          )}
          {onOpen && (
            <MenuItem onClick={run(onOpen)} testId="place-header-menu-open">
              {openWords}
            </MenuItem>
          )}
          {keepAlive && (
            <MenuItem onClick={run(keepAlive.toggle)} checked={keepAlive.on} multi testId="place-header-menu-keep">
              {/* Its tick drawn as the rail's order menu draws one. */}
              <span className="flex items-center gap-2">
                <span className="flex-1">Keep alive</span>
                <span aria-hidden="true" className={"shrink-0 text-accent-hi " + (keepAlive.on ? "" : "invisible")}>
                  ✓
                </span>
              </span>
            </MenuItem>
          )}
          <MenuItem onClick={run(onToggle)} testId="place-header-menu-fold">
            {folded === null ? "Fold" : "Unfold"}
          </MenuItem>
        </RowMenu>
      )}
    </div>
  );
}
