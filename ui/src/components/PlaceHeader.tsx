/**
 * AJ-E2104 — a place's header in the rail: the fold chevron, the app's
 * icon and name, the profile's badge and name, a count (a click opens the
 * profile's page here), and its ↗ (Open), which brings the running app
 * forward (never launches it). D-47: in Your order its handle shows on
 * hover and focus; ⌥⌘↑ and ⌥⌘↓ move it. 0.3.2: a right-click (or
 * Shift+F10) opens its own menu: its page, Bring it forward, Fold — never
 * the web view's (Reload). 0.3.3: a desktop app's menu also turns its Keep
 * alive on and off. 0.3.4 (D-64): a CLI app's menu starts a New session
 * on its folder.
 */
import { RoomChip, useRoomMark } from "./RoomChip";
import { useRef, useState, type JSX, type MouseEvent, type ReactNode } from "react";
import { ariaOf, placeKey } from "../lib/shortcuts";
import { MenuItem, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { ProfileBadge } from "./ProfileBadge";
import { AppIcon } from "./AppIcon";
import { AppKindBadge } from "./AppKindBadge";
import { useFitWhole } from "./use-fit-whole";
import { appKindOfKey } from "../lib/app-kind";
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
  onNew,
  onRecent,
  onRestart,
  roomOf,
  onWarm,
  foldable = true,
  recent = null,
}: {
  /** 0.4.3: an App with something open: its past ones' count, shown on
   * hover and focus ("+3 recent", muted, as its count); a click shows them
   * under it, in place, and it stays shown while they do. */
  recent?: { count: number; open: boolean; onToggle: () => void } | null;
  /** 0.4.3: unfolding it shows a row; when it wouldn't, no fold (a
   * chevron that opened nothing was the 0.4.2 rail's dead end). */
  foldable?: boolean;
  /** X2: pointed at or focused, what opening it reads ahead (a pool's
   * members and canvas, `warmPool`). */
  onWarm?: () => void;
  /** 0.4.1: the folder whose Account's room it marks when out or low
   * ("Out · 9:06 PM", `RoomChip`). */
  roomOf?: string | null;
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
  /** 0.3.4: a CLI app: New session on its folder. */
  onNew?: () => void;
  /** D-100: its sessions that aren't open (its page's Sessions): Open now
   * no longer lists them under it. */
  onRecent?: () => void;
  /** D-71: a running desktop app: Restart…, after its confirm. */
  onRestart?: () => void;
}): JSX.Element {
  const name = [label, place?.profile].filter(Boolean).join(" · ");
  // ADR-0032: an App that runs with nothing open in it says so, folded or
  // not (its last ones may show under it, 0.4.3).
  const empty = count === 0 && folded === null;
  const mark = useRoomMark(roomOf);
  const room = mark ? <RoomChip mark={mark} iconOnly testId="place-room" /> : null;
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const extra = useRef<HTMLSpanElement | null>(null);
  useFitWhole(extra); // D-100: its secondary words whole or not at all
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
      onPointerEnter={onWarm}
      onFocus={onWarm}
      className={"group/head flex h-[30px] min-w-0 items-center gap-0.5 rounded-md pr-1" + (arrange?.dragging ? " opacity-50" : "") + (arrange?.drop ? DROP[arrange.drop] : "")}
      data-testid="place-header"
      data-place={place?.key}
      data-order-head={arrange?.group}
    >
      {foldable ? <FoldButton open={folded === null} label={name} onToggle={onToggle} /> : <span aria-hidden="true" className="w-6 shrink-0" data-testid="place-no-fold" />}
      <button
        type="button"
        onClick={onPage ?? onToggle}
        data-rail-nav
        data-testid="place-page"
        aria-keyshortcuts={[shortcut ? placeKey(shortcut).aria : null, arrange?.enabled ? ariaOf("rail-move") : null].filter(Boolean).join(" ") || undefined}
        title={onPage ? `Open ${name}'s page` : undefined}
        className="relative flex min-h-6 min-w-0 flex-1 items-center gap-1.5 rounded-md px-1 py-1 text-left outline-none hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        {/* 0.4.3: its Out or low mark in its name too (the mark itself is drawn aria-hidden). */}
        <span className="sr-only">{[folded ?? `${name}, ${empty ? "nothing open" : `${count} open`}${needs > 0 ? `, ${needs} need you` : ""}`, mark?.title].filter(Boolean).join(", ")}</span>
        <span aria-hidden="true" className="flex min-w-0 flex-1 items-center gap-[3px]">
          {/* D-100: its kind beside its icon (a window, or >_), never over it. */}
          {children ?? (place && (
            <>
              <AppIcon appId={place.appId} appPid={place.appPid} name={place.app} size={16} />
              <AppKindBadge kind={appKindOfKey(place.key)} />
            </>
          ))}
          {place?.profile && <ProfileBadge app={place.key} name={place.profile} size={6} />}
          {/* D-100 (the 0.4.2 review): the App's own name gives way last.
              It takes its own width (cut, at its end, its full name on
              hover, only when nothing else is left to give); its secondary
              words (its app's word, a folded App's summary) take what's
              left, each whole or hidden (the kind mark
              already says Desktop or CLI), "nothing open" before the app
              word; the marks after it give way first, to their icon. */}
          <span className="ml-0.5 min-w-0 truncate text-label font-semibold leading-[18px] text-ink" data-testid={place?.profile ? "place-profile" : undefined} title={name}>
            {place?.profile ?? label}
          </span>
          <span ref={extra} className="flex h-[18px] min-w-0 flex-1 basis-0 items-center gap-x-[3px] overflow-hidden" data-testid="place-name-line">
            {place?.profile && <span className="shrink-0 whitespace-nowrap text-label leading-[18px] text-muted" data-testid="place-app-word">{label}</span>}
            {folded !== null && needs === 0 && <span className="shrink-0 whitespace-nowrap text-label leading-[18px] text-muted">{folded.slice(folded.indexOf("·") + 1).trim()}</span>}
            {/* 0.4.4: placed first, so it wins over the app word (the kind mark and icon say the app). */}
            {empty && <span className="shrink-0 whitespace-nowrap text-label leading-[18px] text-muted" data-testid="place-nothing-open" data-fit="first">nothing open</span>}
          </span>
          {/* What the amber count is, on the chip itself (the review of
              0.1.4: an unexplained "1"): the Needs you mark and a tooltip. */}
          {needs > 0 && (
            <span title={`${needs} here ${needs === 1 ? "needs" : "need"} you: ${needs === 1 ? "it's" : "they're"} at the top, under Needs you`} className="inline-flex min-w-[20px] shrink-[1000] items-center gap-0.5 overflow-hidden rounded-full bg-warning-soft px-1 text-label font-semibold leading-[18px] tabular-nums text-warning" data-testid="place-needs">
              <IconAlert width={10} height={10} aria-hidden="true" />
              {needs}
            </span>
          )}
          {/* 0.4.1: out of room or low, its mark; the count steps aside
              for it (its rows show what's open; the name above keeps it),
              so the profile's name keeps its room. */}
          {room ?? (folded === null && !empty && <span className="min-w-0 shrink-[1000] overflow-hidden text-label tabular-nums text-muted" title={`${count} open here`}>{count}</span>)}
        </span>
      </button>
      {arrange?.enabled && (
        // No room of its own until hover or focus (it held ~20 px the App's
        // name needs: "Wor…" with the Out mark at 1280, the 0.4.1 gate);
        // still in the Tab order, and focus inside the header widens it.
        <span className={arrange.dragging ? "" : "w-0 overflow-hidden opacity-0 transition-opacity group-focus-within/head:w-auto group-focus-within/head:opacity-100 group-hover/head:w-auto group-hover/head:opacity-100 motion-reduce:transition-none"}>
          <Grip arrange={arrange} label={name} />
        </span>
      )}
      {recent && (
        <button
          type="button"
          onClick={recent.onToggle}
          aria-expanded={recent.open}
          data-rail-nav
          title={recent.open ? `Hide ${name}'s recent sessions` : `Show ${name}'s ${recent.count} recent ${recent.count === 1 ? "session" : "sessions"} here`}
          data-testid="place-recent"
          className={(recent.open ? "flex" : "hidden focus-visible:flex group-hover/head:flex group-has-[:focus-visible]/head:flex") + " h-6 shrink-0 items-center rounded-md px-1 text-label tabular-nums text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"}
        >
          +{recent.count} recent
        </button>
      )}
      {onOpen && (
        <button
          type="button"
          onClick={onOpen}
          aria-label={`Open ${name}`}
          title={shortcut ? `${openWords} (${placeKey(shortcut).words})` : openWords}
          data-testid="place-open"
          // On hover and keyboard focus, as the rows' tools (the 0.4
          // review: always shown, it took the room the App's name needs).
          className="hidden size-6 shrink-0 items-center justify-center rounded-md border border-hairline text-muted outline-none hover:border-accent/40 hover:text-ink focus-visible:flex focus-visible:ring-1 focus-visible:ring-accent/60 group-hover/head:flex group-has-[:focus-visible]/head:flex"
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
          {onNew && (
            <MenuItem onClick={run(onNew)} testId="place-header-menu-new">
              New session
            </MenuItem>
          )}
          {onRecent && (
            <MenuItem onClick={run(onRecent)} testId="place-header-menu-recent">
              Show recent
            </MenuItem>
          )}
          {onRestart && (
            <MenuItem onClick={run(onRestart)} testId="place-header-menu-restart">
              Restart…
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
          {foldable && (
            <MenuItem onClick={run(onToggle)} testId="place-header-menu-fold">
              {folded === null ? "Fold" : "Unfold"}
            </MenuItem>
          )}
        </RowMenu>
      )}
    </div>
  );
}
