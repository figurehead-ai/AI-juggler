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
import { useState, type JSX } from "react";
import { AppIcon } from "./AppIcon";
import { IconAlert } from "./icons";
import { ProfileBadge } from "./ProfileBadge";
import { SectionTitle } from "./SectionTitle";
import { showToast } from "./Toast";
import { conversationsBringForward } from "../lib/ipc/conversations";
import { placesOpen, type KnownPlaceInfo } from "../lib/ipc/places";
import { placeFor, placeWords, useKnownPlaces } from "../lib/places";
import { groupItems, type RailGroup } from "../lib/rail-places";
import { knownAppWord } from "../lib/rail-apps";
import { appName } from "../lib/app-name";
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
    (outcome) => showToast({ id: "go-to", message: outcome.outcome }),
    () => showToast({ id: "go-to", message: "Couldn't bring it forward" }),
  );
}

/** 0.3.3: an app or profile kept alive (0.3.4: any remembered one) that
 * no session tile shows: down (why, and Start) or running with nothing
 * open yet (Open). */
function KeptTile({ place }: { place: KnownPlaceInfo }): JSX.Element {
  const running = place.app_pid !== null;
  // The open tiles' word for it ("Claude app"), D-64.
  const app = knownAppWord(place);
  const name = appName(app, place.instance ?? place.profile);
  const verb = running ? "Open" : "Start";
  const start = () => {
    showToast({ id: "go-to", message: running ? `Bringing ${name} forward…` : `Starting ${name}…` });
    placesOpen(place.id).then(
      (outcome) => showToast({ id: "go-to", message: outcome.outcome }),
      () => showToast({ id: "go-to", message: running ? `Couldn't bring ${name} forward` : `Couldn't start ${name}` }),
    );
  };
  return (
    <li
      className={"flex min-w-0 flex-col gap-3 rounded-card border border-hairline bg-surface p-4 " + (running ? "shadow-card" : "border-dashed")}
      data-testid={running ? "home-app-kept" : "home-app-closed"}
      data-place={place.id}
    >
      <div className="flex min-w-0 items-start gap-3">
        <AppIcon appId={place.app_id} appPid={null} name={app} size={32} />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="truncate text-body font-semibold text-ink">{app}</span>
          {place.profile && (
            <span className="flex min-w-0 items-center gap-1 text-meta font-semibold text-ink">
              <ProfileBadge name={place.profile} />
              <span className="truncate">{place.profile}</span>
            </span>
          )}
        </div>
      </div>
      <span className={"text-meta " + (place.paused ? "text-warning" : "text-muted")}>{placeWords(place)}</span>
      <div className="mt-auto flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={start}
          disabled={!running && place.cannot_start !== null}
          title={running ? undefined : (place.cannot_start ?? undefined)}
          aria-label={`${verb} ${name}`}
          className="rounded-md border border-hairline px-2.5 py-1 text-meta font-medium text-ink outline-none hover:border-accent/40 focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {verb} {app}
        </button>
      </div>
    </li>
  );
}

function Tile({ group, runway, kept }: { group: RailGroup; runway: Runway | null; kept: boolean }): JSX.Element {
  const place = group.place!;
  const needs = group.items.filter((item) => item.need !== null).length;
  const count = group.items.length;
  const low = runway?.left !== null && runway?.left !== undefined && runway.left <= 25;
  const name = [place.app, place.profile].filter(Boolean).join(" · ");
  return (
    <li className="flex min-w-0 flex-col gap-3 rounded-card border border-hairline bg-surface p-4 shadow-card" data-testid="home-app-tile" data-place={place.key}>
      <div className="flex min-w-0 items-start gap-3">
        <AppIcon appId={place.appId} appPid={place.appPid} name={place.app} size={32} />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="truncate text-body font-semibold text-ink">{place.app}</span>
          {/* The profile's badge beside its name, never on the icon
              (D-38); the count moves under it rather than being cut. */}
          <span className="flex min-w-0 flex-wrap items-center gap-x-1 gap-y-0.5 text-meta text-muted">
            {place.profile && (
              <span className="flex min-w-0 max-w-full items-center gap-1 font-semibold text-ink">
                <ProfileBadge name={place.profile} />
                <span className="truncate">{place.profile}</span>
              </span>
            )}
            {place.profile ? <span>·</span> : null}
            <span className="whitespace-nowrap">{count === 1 ? "1 session" : `${count} sessions`}</span>
            {/* A pill, not "· kept alive": wrapped under the count, a line
                that starts with a dot read as a fragment. */}
            {kept && (
              <span className="whitespace-nowrap rounded-full bg-accent/10 px-1.5 text-label font-medium text-accent-hi" data-testid="home-app-kept-mark" title="Keep alive: AI Juggler starts it again if it stops">
                kept alive
              </span>
            )}
          </span>
        </div>
      </div>
      {runway && (
        <div className="flex flex-col gap-1">
          <span className={"truncate text-meta " + (low ? "text-warning" : "text-muted")}>{runway.words}</span>
          {runway.left !== null && (
            <div className="h-1.5 overflow-hidden rounded-full bg-surface-3" role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={runway.left} aria-label={`${name}: ${runway.words}`}>
              <div className={"h-full rounded-full " + (low ? "bg-warning" : "bg-accent")} style={{ width: `${runway.left}%` }} />
            </div>
          )}
        </div>
      )}
      {/* Open on the left; what the amber count is, in words, on the
          right (the review of 0.1.4), never squeezing the app's name. */}
      {(place.appPid !== null || needs > 0) && (
        <div className="mt-auto flex flex-wrap items-center justify-between gap-2">
          {place.appPid !== null && (
            <button
              type="button"
              onClick={() => bringForward(group)}
              className="rounded-md border border-hairline px-2.5 py-1 text-meta font-medium text-ink outline-none hover:border-accent/40 focus-visible:ring-1 focus-visible:ring-accent/60"
            >
              Open {place.app}
              <span className="sr-only"> · {place.profile}</span>
            </button>
          )}
          {needs > 0 && (
            <span className="ml-auto inline-flex shrink-0 items-center gap-1 rounded-full bg-warning-soft px-2 py-0.5 text-label font-semibold text-warning" data-testid="home-app-needs" title="Listed under Needs you">
              <IconAlert width={11} height={11} aria-hidden="true" />
              {needs} {needs === 1 ? "needs" : "need"} you
            </span>
          )}
        </div>
      )}
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
    .sort((a, b) => Number(b.keep_alive) - Number(a.keep_alive) || b.seen_ms - a.seen_ms);
  const tiles: ({ group: RailGroup } | { kept: KnownPlaceInfo })[] = [...places.map((group) => ({ group })), ...kept.map((place) => ({ kept: place }))];
  if (tiles.length === 0) return null;
  // Never a tile that hides a single app behind "+1" (the 0.2 walk, F18):
  // seven show as seven; past that, six and a tile that says how many more.
  const cut = tiles.length > TILES_SHOWN + 1;
  const shown = all || !cut ? tiles : tiles.slice(0, TILES_SHOWN);
  const more = tiles.length - TILES_SHOWN;
  return (
    <section aria-labelledby="home-apps" className="flex flex-col gap-2.5" data-testid="home-apps">
      <SectionTitle id="home-apps">Your apps</SectionTitle>
      <ul className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,210px),1fr))] gap-grid">
        {shown.map((tile) =>
          "group" in tile ? (
            <Tile key={tile.group.key} group={tile.group} runway={runwayOf(tile.group.items[0]?.accountId ?? null)} kept={placeFor(known, tile.group.place?.appPid ?? null)?.keep_alive === true} />
          ) : (
            <KeptTile key={tile.kept.id} place={tile.kept} />
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
