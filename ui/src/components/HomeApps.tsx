/**
 * AJ-E2105 — Home's "Your apps": one tile per running app and profile
 * (a place): the app's own icon (D-38) and the profile's badge, with how many sessions are open there, a needs-you badge,
 * the runway of the account it runs on, and Open (the app forward). 0.3.3:
 * an app or profile kept alive has a tile too when no session tile shows
 * it: down (why, and Start, which starts it) or running with nothing open
 * yet (Open). 0.3.4 (D-64): every remembered app or profile with no
 * session tile has one, after them (kept alive first, then the most
 * recently seen), so an app that quits is one click from starting again.
 * At most six, then "+n".
 */
import { ProfileRoomChip } from "./RoomChip";
import { useState, type JSX } from "react";
import { AppIcon } from "./AppIcon";
import { AppKindBadge } from "./AppKindBadge";
import { appKindOfKey } from "../lib/app-kind";
import { IconAlert } from "./icons";
import { IconReplay } from "./icons-actions";
import { ProfileBadge } from "./ProfileBadge";
import { SectionTitle } from "./SectionTitle";
import { showToast } from "./Toast";
import { showGoTo } from "../lib/go-to";
import { conversationsBringForward } from "../lib/ipc/conversations";
import { placesOpen, type KnownPlaceInfo } from "../lib/ipc/places";
import { placeFor, placeWords, useKnownPlaces } from "../lib/places";
import { groupItems, type RailGroup } from "../lib/rail-places";
import { appFolderOf, knownAppWord } from "../lib/rail-apps";
import { appIdOfRailKey, openApp } from "../lib/apps";
import { appName, desktopProfile } from "../lib/app-name";
import type { RailItem } from "../lib/rail-items";

/** Tiles shown before "+n". */
export const TILES_SHOWN = 6;

export interface Runway {
  left: number | null;
  words: string;
}

/** The places with something open, AI Juggler's own left out (it's here). */
export function appPlaces(items: RailItem[]): RailGroup[] {
  return groupItems(
    items.filter((item) => item.live && item.place.kind !== "own"),
    "place",
  );
}

function bringForward(group: RailGroup): void {
  const pid = group.place?.appPid;
  if (pid === null || pid === undefined) return;
  showToast({ id: "go-to", message: "Bringing it forward…" });
  conversationsBringForward(pid).then(
    showGoTo,
    () => showToast({ id: "go-to", message: "Couldn't bring it forward" }),
  );
}

/** 0.3.3: an app or profile kept alive (0.3.4: any remembered one) that
 * no session tile shows: down (why, and Start) or running with nothing
 * open yet (Open). */
function KeptTile({ place, rows }: { place: KnownPlaceInfo; rows: Rows }): JSX.Element {
  const running = place.app_pid !== null;
  // The open tiles' word for it ("Claude app"), D-64.
  const app = knownAppWord(place);
  const profile = desktopProfile(place.instance, place.profile);
  const name = appName(app, profile);
  const verb = running ? "Open" : "Start";
  const start = () => {
    showToast({ id: "go-to", message: running ? `Bringing ${name} forward…` : `Starting ${name}…` });
    placesOpen(place.id).then(
      showGoTo,
      () => showToast({ id: "go-to", message: running ? `Couldn't bring ${name} forward` : `Couldn't start ${name}` }),
    );
  };
  return (
    <li
      className={"@container flex h-full min-w-0 flex-col gap-3 rounded-card border border-hairline bg-surface p-4 " + (running ? "shadow-card" : "border-dashed")}
      data-testid={running ? "home-app-kept" : "home-app-closed"}
      data-place={place.id}
    >
      <div className="flex min-w-0 items-start gap-3">
        <AppIcon appId={place.app_id} appPid={null} name={app} size={32} />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          {/* D-100: its name, then its kind (a remembered app is a desktop one). */}
          <span className="flex min-w-0 items-center gap-2">
            <AppName id={place.id} name={name}>
              {app}
            </AppName>
            <AppKindBadge kind="desktop" variant="chip" wordClass="hidden @[15rem]:inline" />
          </span>
          {/* Always two lines, as a running card's: the profile, or none. */}
          <span className="flex min-w-0 items-center gap-1 text-meta font-semibold text-ink">
            {profile ? (
              <>
                <ProfileBadge app={place.colour_slot ?? place.id} name={profile} />
                <span className="truncate" title={profile}>
                  {profile}
                </span>
              </>
            ) : (
              "\u00a0"
            )}
          </span>
        </div>
      </div>
      {/* 0.4.1: its Account out of room or low, as on every card. */}
      <span className="flex empty:hidden">
        <ProfileRoomChip profileId={place.account_id} testId="home-app-room" />
      </span>
      {/* Its state where a running card's account line is. */}
      <div className="flex flex-col gap-1">
        <span className={"truncate text-meta " + (place.paused || place.lost ? "text-warning" : "text-muted")} title={placeWords(place)}>
          {placeWords(place)}
        </span>
        {rows.bar && <div className="h-1.5" aria-hidden="true" />}
      </div>
      <Actions needs={0}>
        <button
          type="button"
          onClick={start}
          disabled={!running && place.cannot_start !== null}
          title={running ? undefined : (place.cannot_start ?? undefined)}
          aria-label={`${verb} ${name}`}
          className="rounded-md border border-hairline px-2.5 py-1 text-meta font-medium text-ink outline-none hover:border-accent/40 focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {/* The tile names the App already: "Start" (design review #48). */}
          {verb}
        </button>
      </Actions>
    </li>
  );
}

/** 0.4.1: which rows the cards in view keep, so every card lines up: the
 * account line when any card has one (a down app's state stands there),
 * the usage bar when any card has a measured one; none of them, no gap. */
interface Rows {
  words: boolean;
  bar: boolean;
}

/** 0.4.1: a tile's account line and usage bar, the rows the cards in view
 * keep (an empty one keeps its height): one line, cut with the whole words
 * in its tooltip. */
function AccountRows({ words, left, label, low, rows }: { words: string | null; left: number | null; label: string; low: boolean; rows: Rows }): JSX.Element | null {
  if (!rows.words && !rows.bar) return null;
  return (
    <div className="flex flex-col gap-1" data-testid="home-app-account">
      {rows.words && (
        <span className={"truncate text-meta " + (low ? "text-warning" : "text-muted")} title={words ?? undefined}>
          {words ?? "\u00a0"}
        </span>
      )}
      {left !== null ? (
        <div className="h-1.5 overflow-hidden rounded-full bg-surface-3" role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={left} aria-label={`${label}: ${words ?? ""}`}>
          <div className={"h-full rounded-full " + (low ? "bg-warning" : "bg-accent")} style={{ width: `${left}%` }} />
        </div>
      ) : rows.bar ? (
        <div className="h-1.5" aria-hidden="true" />
      ) : null}
    </div>
  );
}

/** 0.4.1: the bottom row, in the same place on every card: the action on
 * the left, what needs you (when anything does) on the right; one line. */
function Actions({ children, needs }: { children: JSX.Element | null; needs: number }): JSX.Element {
  return (
    <div className="mt-auto flex min-h-7 flex-nowrap items-center justify-between gap-2">
      {children ?? <span />}
      {needs > 0 && (
        <span className="inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded-full bg-warning-soft px-2 py-0.5 text-label font-semibold text-warning" data-testid="home-app-needs" title="Listed under Needs you">
          <IconAlert width={11} height={11} aria-hidden="true" />
          {needs} {needs === 1 ? "needs" : "need"} you
        </span>
      )}
    </div>
  );
}

/** D-62: a tile's app name opens its App page. */
function AppName({ id, name, children }: { id: string | null; name: string; children: string }): JSX.Element {
  if (id === null) return <span className="truncate text-body font-semibold text-ink">{children}</span>;
  return (
    <button
      type="button"
      onClick={() => openApp(id)}
      title={`Open ${name}'s page`}
      className="truncate rounded text-left text-body font-semibold text-ink outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="home-app-page"
    >
      {children}
      <span className="sr-only">, {name}'s page</span>
    </button>
  );
}

function Tile({ group, runway, kept, appId, rows, folder }: { group: RailGroup; runway: Runway | null; kept: boolean; appId: string | null; rows: Rows; folder: string | null }): JSX.Element {
  const place = group.place!;
  const needs = group.items.filter((item) => item.need !== null).length;
  const count = group.items.length;
  const low = runway?.left !== null && runway?.left !== undefined && runway.left <= 25;
  const name = [place.app, place.profile].filter(Boolean).join(" · ");
  return (
    <li className="@container flex h-full min-w-0 flex-col gap-3 rounded-card border border-hairline bg-surface p-4 shadow-card" data-testid="home-app-tile" data-place={place.key}>
      <div className="flex min-w-0 items-start gap-3">
        <AppIcon appId={place.appId} appPid={place.appPid} name={place.app} size={32} />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          {/* The app's name, then the kept-alive mark beside it (0.4.1:
              never squeezing the profile): its words on a wide card, its
              mark alone on a narrow one, so the name is never cut. */}
          <span className="flex min-w-0 items-center gap-2">
            <AppName id={appId} name={name}>
              {place.app}
            </AppName>
            {/* D-100: Desktop or CLI, by icon and word, beside its name. */}
            <AppKindBadge kind={appKindOfKey(place.key)} variant="chip" wordClass="hidden @[15rem]:inline" />
            {kept && (
              <span className="inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded-full bg-accent/10 px-1.5 py-0.5 text-label font-medium text-accent-hi" data-testid="home-app-kept-mark" title="Kept alive: AI Juggler starts it again if it stops">
                <IconReplay width={11} height={11} aria-hidden="true" />
                <span className="sr-only @[16rem]:not-sr-only">kept alive</span>
              </span>
            )}
          </span>
          {/* One line (0.4.1): the profile's badge beside its name, never
              on the icon (D-38), then the count; the profile is cut only
              when its name alone is too long, whole in its tooltip. */}
          <span className="flex min-w-0 flex-nowrap items-center gap-x-1 text-meta text-muted">
            {place.profile && (
              <span className="flex min-w-0 items-center gap-1 font-semibold text-ink">
                <ProfileBadge app={place.key} name={place.profile} />
                <span className="truncate" title={place.profile}>
                  {place.profile}
                </span>
              </span>
            )}
            {place.profile ? <span className="shrink-0">·</span> : null}
            {/* Beside a profile on a narrow card the count is a bare number,
                as in the rail, so the profile keeps its name; the words stay
                in its tooltip. */}
            <span className="shrink-0 whitespace-nowrap" title={count === 1 ? "1 session" : `${count} sessions`}>
              {count}
              <span className={place.profile ? "sr-only @[12rem]:not-sr-only" : undefined}> {count === 1 ? "session" : "sessions"}</span>
            </span>
          </span>
        </div>
      </div>
      {/* 0.4.1: out of room or low, unmistakably: the mark every screen
          shows for its Account (lib/room-mark), above its runway. */}
      <span className="flex empty:hidden">
        <ProfileRoomChip profileId={folder} testId="home-app-room" />
      </span>
      <AccountRows words={runway?.words ?? null} left={runway?.left ?? null} label={name} low={low} rows={rows} />
      {/* The action on the left, what needs you on the right, on every card
          in the same place (0.4.1); "Open": the card names the app. */}
      <Actions needs={needs}>
        {place.appPid !== null ? (
          <button
            type="button"
            onClick={() => bringForward(group)}
            aria-label={`Open ${name}`}
            className="rounded-md border border-hairline px-2.5 py-1 text-meta font-medium text-ink outline-none hover:border-accent/40 focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            Open
          </button>
        ) : null}
      </Actions>
    </li>
  );
}

export function HomeApps({ items, runwayOf }: { items: RailItem[]; runwayOf: (accountId: string | null) => Runway | null }): JSX.Element | null {
  const [all, setAll] = useState(false);
  const known = useKnownPlaces();
  const places = appPlaces(items);
  // Remembered with no session tile (down, starting, paused, or running
  // with nothing open yet): after the session tiles, kept alive first,
  // then the most recently seen; never gone after a start.
  const shownPids = new Set(places.map((group) => group.place?.appPid ?? null));
  const kept = (known ?? [])
    .filter((place) => place.app_pid === null || !shownPids.has(place.app_pid))
    // Kept alive first, then the most recently seen; a profile only found
    // by its folder (AJ-E3203) after the ones you've used.
    .sort((a, b) => Number(b.keep_alive) - Number(a.keep_alive) || Number(a.found) - Number(b.found) || b.seen_ms - a.seen_ms);
  const tiles: ({ group: RailGroup } | { kept: KnownPlaceInfo })[] = [...places.map((group) => ({ group })), ...kept.map((place) => ({ kept: place }))];
  if (tiles.length === 0) return null;
  // Never a tile that hides a single app behind "+1" (the 0.2 walk, F18):
  // seven show as seven; past that, six and a tile that says how many more.
  const cut = tiles.length > TILES_SHOWN + 1;
  const shown = all || !cut ? tiles : tiles.slice(0, TILES_SHOWN);
  const more = tiles.length - TILES_SHOWN;
  // 0.4.3: a tile's Account is its App's own (its remembered folder), not
  // its first session's: one of another Account's in it showed that one's.
  const folderOf = (group: RailGroup) => appFolderOf(group, placeFor(known, group.place?.appPid ?? null));
  const runways = shown.map((tile) => ("group" in tile ? runwayOf(folderOf(tile.group)) : null));
  const rows: Rows = {
    words: runways.some((runway) => runway !== null) || shown.some((tile) => "kept" in tile),
    bar: runways.some((runway) => runway?.left !== null && runway?.left !== undefined),
  };
  return (
    <section aria-labelledby="home-apps" className="flex flex-col gap-2.5" data-testid="home-apps">
      <SectionTitle id="home-apps">Your apps</SectionTitle>
      <ul className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,210px),1fr))] gap-grid">
        {shown.map((tile, at) =>
          "group" in tile ? (
            <Tile key={tile.group.key} group={tile.group} runway={runways[at] ?? null} rows={rows} folder={folderOf(tile.group)} kept={placeFor(known, tile.group.place?.appPid ?? null)?.keep_alive === true} appId={tile.group.place ? appIdOfRailKey(tile.group.place.key, folderOf(tile.group), known) : null} />
          ) : (
            <KeptTile key={tile.kept.id} place={tile.kept} rows={rows} />
          ),
        )}
        {cut && (
          <li className="flex">
            <button
              type="button"
              aria-expanded={all}
              onClick={() => setAll((open) => !open)}
              data-testid="home-apps-more"
              className="flex min-h-16 flex-1 items-center justify-center rounded-card border border-dashed border-hairline text-body text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
            >
              {all ? "Show fewer apps" : `Show ${more} more apps`}
            </button>
          </li>
        )}
      </ul>
    </section>
  );
}
