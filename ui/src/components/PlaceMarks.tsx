/**
 * AJ-E2104 with D-38 — a place's marks, one set everywhere a session or a
 * place shows: the app's own icon as installed on this Mac (`AppIcon`),
 * and the profile it runs in as its own badge (`ProfileBadge`): a dot at
 * the icon's corner, ringed in the colour of what it sits on, or a pill
 * beside the name. The badge never covers the icon; one app's profiles
 * share its icon. Decorative: the row or header names both in words.
 */
import type { JSX } from "react";
import { AppIconTile } from "./AppIcon";
import { ProfileBadge } from "./ProfileBadge";
import { OWN_APP, OWN_PLACE, type RailPlace } from "../lib/rail-items";
import type { PlaceInfo } from "../lib/ipc/conversations";

export { ProfileBadge } from "./ProfileBadge";

/** A place to mark from where a session runs: its first place, else
 * AI Juggler's own (a session AI Juggler runs), else an app not named. */
export function markPlace(place: PlaceInfo | undefined, own: boolean): RailPlace {
  if (place) return { key: `${place.app}|${place.instance ?? place.profile ?? ""}`, app: place.app || "An app", appId: place.app_id ?? null, kind: place.kind ?? null, profile: place.profile ?? place.instance, appPid: place.app_pid ?? null };
  return own ? { key: OWN_PLACE, app: OWN_APP, appId: OWN_PLACE, kind: "own", profile: null, appPid: null } : { key: "", app: "An app", appId: null, kind: null, profile: null, appPid: null };
}

/** The corner dot's diameter for an icon of `size`. */
function dotFor(size: number): number {
  return size >= 32 ? 8 : size >= 20 ? 7 : 6;
}

/** The app's icon with the profile's dot at its corner; `profile` when
 * the place doesn't name one (AI Juggler's own sessions: the profile each
 * runs on). `ring` is the colour under the mark (the row's surface). */
export function PlaceMark({ place, profile, size = 24, ring = "var(--color-canvas)" }: { place: RailPlace; profile?: string | null; size?: number; ring?: string }): JSX.Element {
  const name = place.profile ?? profile ?? null;
  const dot = dotFor(size);
  // The dot sits on the icon's corner, where a macOS icon is clear.
  const offset = -Math.round(dot / 3);
  return (
    <span className="relative inline-flex shrink-0" style={{ width: size, height: size }} data-testid="place-mark">
      <AppIconTile appId={place.appId} appPid={place.appPid} name={place.app} size={size} />
      {name && (
        <span className="absolute flex" style={{ right: offset, bottom: offset }} data-testid="place-mark-profile">
          <ProfileBadge name={name} size={dot} ring={ring} />
        </span>
      )}
    </span>
  );
}
