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
import { OWN_APP, OWN_PLACE, SOME_APP, cliPlace, type RailPlace } from "../lib/rail-items";
import { desktopAppKey, desktopAppWord } from "../lib/app-name";
import type { PlaceInfo } from "../lib/ipc/conversations";

export { ProfileBadge } from "./ProfileBadge";

/** The App to mark a session with (D-62): a desktop app in its instance;
 * else tool `tool`'s CLI App on folder `folder`, its window in the
 * terminal it runs in (or AI Juggler's own). With no tool (a task's
 * sessions, which AI Juggler runs), AI Juggler itself. */
export function markPlace(place: PlaceInfo | undefined, own: boolean, tool: string | null = null, folder: string | null = null): RailPlace {
  if (place?.kind === "desktop") return { key: desktopAppKey(place.app_id ?? null, place.app, place.instance), app: desktopAppWord(place.app, tool), appId: place.app_id ?? null, kind: "desktop", profile: place.instance ?? null, appPid: place.app_pid ?? null, host: null };
  if (tool !== null) return cliPlace(tool, null, folder ?? place?.profile ?? "", { name: own && !place ? OWN_APP : place?.app || null, kind: own && !place ? "own" : place?.kind ?? null, pid: place?.app_pid ?? null });
  return own ? { key: OWN_PLACE, app: OWN_APP, appId: OWN_PLACE, kind: "own", profile: null, appPid: null, host: OWN_APP } : { key: "", app: SOME_APP, appId: null, kind: null, profile: null, appPid: null, host: null };
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
