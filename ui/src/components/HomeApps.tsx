/**
 * AJ-E2105 — Home's "Your apps": one tile per running app and profile
 * (a place): the app's own icon (D-38) and the profile's badge, with how many sessions are open there, a needs-you badge,
 * the runway of the account it runs on, and Open (the app forward, never
 * launched). At most six, then "+n".
 */
import { useState, type JSX } from "react";
import { AppIcon } from "./AppIcon";
import { IconAlert } from "./icons";
import { ProfileBadge } from "./ProfileBadge";
import { SectionTitle } from "./SectionTitle";
import { showToast } from "./Toast";
import { conversationsBringForward } from "../lib/ipc/conversations";
import { groupItems, type RailGroup } from "../lib/rail-places";
import { OWN_PLACE, type RailItem } from "../lib/rail-items";

/** Tiles shown before "+n". */
export const TILES_SHOWN = 6;

export interface Runway {
  left: number | null;
  words: string;
}

/** The places with something open, AI Juggler's own left out (it's here). */
export function appPlaces(items: RailItem[]): RailGroup[] {
  return groupItems(
    items.filter((item) => item.live && item.place.key !== OWN_PLACE),
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

function Tile({ group, runway }: { group: RailGroup; runway: Runway | null }): JSX.Element {
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
  const places = appPlaces(items);
  if (places.length === 0) return null;
  const shown = all ? places : places.slice(0, TILES_SHOWN);
  return (
    <section aria-labelledby="home-apps" className="flex flex-col gap-2.5" data-testid="home-apps">
      <SectionTitle id="home-apps">Your apps</SectionTitle>
      <ul className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,210px),1fr))] gap-grid">
        {shown.map((group) => (
          <Tile key={group.key} group={group} runway={runwayOf(group.items[0]?.accountId ?? null)} />
        ))}
        {places.length > TILES_SHOWN && (
          <li className="flex">
            <button
              type="button"
              aria-expanded={all}
              onClick={() => setAll((open) => !open)}
              data-testid="home-apps-more"
              className="flex min-h-16 flex-1 items-center justify-center rounded-card border border-dashed border-hairline text-body text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
            >
              {all ? "Fewer" : `+${places.length - TILES_SHOWN}`}
            </button>
          </li>
        )}
      </ul>
    </section>
  );
}
