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
import { MenuItem, MenuLine, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { ProfileBadge } from "./ProfileBadge";
import { AppIcon } from "./AppIcon";
import { AppKindBadge } from "./AppKindBadge";
import { useFitWhole } from "./use-fit-whole";
import { appKindOfKey } from "../lib/app-kind";
import { FoldButton, HeadNeeds, headNeedsWords } from "./rail-groups";
import { IconHistory, IconUpRight } from "./icons-places";
import { IconFolderOpen, IconPage, IconPlus, IconReplay } from "./icons-actions";
import { IconShield } from "./icons";
import { IconChevronDown } from "./icons-controls";
import { appTone, type AttentionLevel } from "../lib/attention";
import { DROP, Grip, type Arrangeable } from "./RowTools";
import type { RailPlace } from "../lib/rail-items";
import { NOT_SHOWN, toneTip, unlinked } from "../lib/app-folder";
import type { PlaceFolderInfo } from "../types/generated/PlaceFolderInfo";

/** A place's header: the fold chevron; app, profile, count (its page);
 * Open. Folded, it reads "Work · 4 idle". */

/** AJ-E1554 R1: the App's icon gives way to the grip on hover and focus. */
const GIVES_WAY = " transition-opacity group-focus-within/head:opacity-0 group-hover/head:opacity-0 motion-reduce:transition-none";

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
  roomApp = null,
  onWarm,
  foldable = true,
  level = 0,
  probably = false,
  lit = false,
  broken = [],
  folder = null,
  onChooseFolder,
  cluster,
}: {
  /** AJ-E1554 R2: its counts (working, needs you, to review, subagents),
   * each a way to exactly those, in place of the bare count. */
  cluster?: ReactNode;
  /** AJ-E2327: its tool folder (what told), or why none is linked: an App
   * whose sessions can't be shown says so, never just a grey icon. */
  folder?: PlaceFolderInfo | null;
  /** AJ-E2327: Choose its folder… (the system's picker), in its menu. */
  onChooseFolder?: () => void;
  /** D-117: each L3 thing's word here, said in its name (its red dot is
   * decoration): "2 need you, 1 stopped with an error". */
  broken?: readonly string[];
  /** D-117: the highest level of what needs you here (its count's mark),
   * a ring when every one is a guess; `lit`: it is current (a row of it
   * is open). At L2/L3 or lit, its icon is in full colour. */
  level?: AttentionLevel;
  probably?: boolean;
  lit?: boolean;
  /** 0.4.3: unfolding it shows a row; when it wouldn't, no fold (a
   * chevron that opened nothing was the 0.4.2 rail's dead end). */
  foldable?: boolean;
  /** X2: pointed at or focused, what opening it reads ahead (a pool's
   * members and canvas, `warmPool`). */
  onWarm?: () => void;
  /** 0.4.1: the folder whose Account's room it marks when out or low
   * ("Out · 9:06 PM", `RoomChip`). */
  roomOf?: string | null;
  /** AJ-E3210: the desktop App the header is (its mark is its own
   * Account's, not its folder's CLI's). */
  roomApp?: string | null;
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
  // AJ-E2327: no folder linked: its sessions can't be shown, and it says
  // why (the row's words and its icon's tooltip), not "nothing open".
  const notShown = empty && unlinked(folder) ? folder.why : null;
  const tip = toneTip(level >= 2 || lit, folder);
  const mark = useRoomMark(roomOf, [], roomApp);
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
      className={"tone-host group/head relative flex h-[30px] min-w-0 items-center gap-0.5 rounded-md pr-1" + (arrange?.dragging ? " opacity-50" : "") + (arrange?.drop ? DROP[arrange.drop] : "")}
      data-testid="place-header"
      data-lit={level >= 2 || lit ? "true" : undefined}
      data-level={needs > 0 ? level : undefined}
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
        <span className="sr-only">{[folded ?? `${name}, ${notShown ? `${NOT_SHOWN}: ${notShown}` : empty ? "nothing open" : `${count} open`}${needs > 0 ? `, ${headNeedsWords(needs, broken)}` : ""}`, mark?.title].filter(Boolean).join(", ")}</span>
        <span aria-hidden="true" className="flex min-w-0 flex-1 items-center gap-[3px]">
          {/* D-100: its kind beside its icon (a window, or >_), never over it. */}
          {children ?? (place && (
            <>
              <AppIcon appId={place.appId} appPid={place.appPid} name={place.app} size={16} className={appTone(level >= 2 || lit) + (arrange?.enabled ? GIVES_WAY : "")} tip={tip} />
              <AppKindBadge kind={appKindOfKey(place.key)} />
            </>
          ))}
          {place?.profile && <ProfileBadge app={place.key} name={place.profile} size={6} className={appTone(level >= 2 || lit)} />}
          {/* D-118's mock (round 3): the account, then the app, always
              ("Work Max Claude app", "Work Max Claude Code CLI"): two Apps
              on one Account read "Work Max" twice when the app word gave
              way, told apart by their icons alone. Both take their own
              width (cut at their end, the whole name on hover, only when
              nothing else is left to give). An App with no profile
              (ChatGPT) is its name alone. */}
          <span className={"ml-0.5 min-w-0 truncate text-label font-semibold leading-[18px] text-ink" + (place?.profile ? " shrink-0 max-w-[55%]" : "")} data-testid={place?.profile ? "place-profile" : undefined} title={name}>
            {place?.profile ?? label}
          </span>
          {place?.profile && (
            // AJ-E3603: the app word gives way first: the profile is whole
            // up to half the line or so (max 55 %), so a cut header reads
            // "Work Max Claude Cod…", never "Work … Claude Cod…" (the 0.4.1
            // gate's "Wor…").
            <span className="min-w-0 truncate whitespace-nowrap text-label leading-[18px] text-muted" data-testid="place-app-word" title={name}>
              {label}
            </span>
          )}
          {/* D-100: what's left after the name holds the rest, each whole or
              hidden (its spoken name says them all): its needs' mark first
              (D-117; the loud copy is Needs you's row), "nothing open"
              (0.4.4), a folded App's summary, then its Out or low mark
              (0.4.1; the count steps aside for it) or its count, at the
              end. */}
          {/* AJ-E3603 (R15): its needs' mark always has its room: the names
              give way for it (cut at their end, whole on hover), as the
              mock draws "Work Max Claude Code ○ 1 3"; at the rail's 264 px
              "Personal Claude Code CLI" left it none, so a group that
              needed you never said so. */}
          <span ref={extra} style={needs > 0 ? { minWidth: `calc(19px + ${String(needs).length}ch)` } : undefined} className={"flex h-[18px] min-w-0 flex-1 basis-0 items-center gap-x-[3px] overflow-hidden text-label tabular-nums" + (folded === null && !empty ? " justify-end" : "")} data-testid="place-name-line">
            {folded !== null && needs === 0 && <span className="shrink-0 whitespace-nowrap text-label leading-[18px] text-muted">{folded.slice(folded.indexOf("·") + 1).trim()}</span>}
            {empty && !notShown && <span className="shrink-0 whitespace-nowrap text-label leading-[18px] text-muted" data-testid="place-nothing-open" data-fit="first">nothing open</span>}
            {notShown && <span className="shrink-0 whitespace-nowrap text-label leading-[18px] text-muted" data-testid="place-not-shown" data-fit="first" title={notShown}>{NOT_SHOWN}</span>}
            {needs > 0 && cluster === undefined && (
              <span className="flex shrink-0" data-fit="first">
                <HeadNeeds needs={needs} level={level} probably={probably} />
              </span>
            )}
            {room ? <span className="flex shrink-0">{room}</span> : cluster === undefined && folded === null && !empty && <span className="shrink-0 text-label tabular-nums text-muted" title={`${count} open here`}>{count}</span>}
          </span>
        </span>
      </button>
      {/* AJ-E1554 R1: the grip in its icon's place on hover and focus (the
          icon gives way), so it takes no room and moves nothing. Still
          ⌥⌘↑ and ⌥⌘↓ from the keyboard. */}
      {arrange?.enabled && (
        <span className={"absolute left-[30px] top-1/2 flex -translate-y-1/2 " + (arrange.dragging ? "" : "opacity-0 transition-opacity group-focus-within/head:opacity-100 group-hover/head:opacity-100 motion-reduce:transition-none")} data-testid="place-grip">
          <Grip arrange={arrange} label={name} />
        </span>
      )}
      {/* R2: its counts, always shown, each its own way in. */}
      {cluster}
      {/* R1: its ↗ in a slot of its own, as wide at rest as under the
          pointer (nothing shifts or covers its name), shown on hover and
          focus. PAT-SURF: a tool's hover is a tone, never a line. */}
      {onOpen && (
        <button
          type="button"
          onClick={onOpen}
          aria-label={`Open ${name}`}
          title={shortcut ? `${openWords} (${placeKey(shortcut).words})` : openWords}
          data-testid="place-open"
          className="flex size-6 shrink-0 items-center justify-center rounded-md text-muted opacity-0 outline-none transition-opacity hover:bg-hover hover:text-ink focus-visible:opacity-100 focus-visible:ring-1 focus-visible:ring-accent/60 group-focus-within/head:opacity-100 group-hover/head:opacity-100 motion-reduce:transition-none"
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
              <MenuLine icon={IconPage} label="Open its page" />
            </MenuItem>
          )}
          {onOpen && (
            <MenuItem onClick={run(onOpen)} testId="place-header-menu-open">
              <MenuLine icon={IconUpRight} label={openWords} />
            </MenuItem>
          )}
          {onNew && (
            <MenuItem onClick={run(onNew)} testId="place-header-menu-new">
              <MenuLine icon={IconPlus} label="New session" />
            </MenuItem>
          )}
          {onRecent && (
            <MenuItem onClick={run(onRecent)} testId="place-header-menu-recent">
              <MenuLine icon={IconHistory} label="Show recent" />
            </MenuItem>
          )}
          {onChooseFolder && (
            <MenuItem onClick={run(onChooseFolder)} testId="place-header-menu-folder">
              <MenuLine icon={IconFolderOpen} label="Choose its folder…" />
            </MenuItem>
          )}
          {onRestart && (
            <MenuItem onClick={run(onRestart)} testId="place-header-menu-restart">
              <MenuLine icon={IconReplay} label="Restart…" />
            </MenuItem>
          )}
          {keepAlive && (
            <MenuItem onClick={run(keepAlive.toggle)} checked={keepAlive.on} multi testId="place-header-menu-keep">
              {/* Its tick drawn as the rail's order menu draws one. */}
              <MenuLine icon={IconShield} label="Keep alive" tick={keepAlive.on} />
            </MenuItem>
          )}
          {foldable && (
            <MenuItem onClick={run(onToggle)} testId="place-header-menu-fold">
              <MenuLine icon={IconChevronDown} label={folded === null ? "Fold" : "Unfold"} />
            </MenuItem>
          )}
        </RowMenu>
      )}
    </div>
  );
}
