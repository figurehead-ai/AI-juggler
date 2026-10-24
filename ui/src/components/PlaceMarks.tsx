/**
 * AJ-E2104 — a place's marks: the app's icon (by its kind, from the host
 * registry's data: a desktop app's monogram, a terminal, an editor,
 * AI Juggler's own), the profile's badge on its corner, and the profile
 * chip a place header shows. Decorative: the row or header names both in
 * words.
 */
import type { JSX } from "react";
import { IconJuggler, IconTerminal } from "./icons";
import { IconCode } from "./icons-places";
import { profileColor, profileMonogram } from "../lib/profile-marks";
import type { RailPlace } from "../lib/rail-items";

/** The app's two letters ("Cl", "Ch"): enough to tell two apps apart. */
function appLetters(app: string): string {
  const letters = [...app.replace(/[^\p{L}\p{N}]/gu, "")];
  return `${(letters[0] ?? "?").toUpperCase()}${(letters[1] ?? "").toLowerCase()}`;
}

export function AppIcon({ place, size = 18 }: { place: RailPlace; size?: number }): JSX.Element {
  const glyph = Math.round(size * 0.62);
  return (
    <span
      aria-hidden="true"
      title={place.app}
      data-testid="app-icon"
      className="inline-flex shrink-0 items-center justify-center rounded-[28%] border border-hairline bg-surface-3 font-semibold leading-none text-ink"
      style={{ width: size, height: size, fontSize: Math.max(8, Math.round(size * 0.42)) }}
    >
      {place.kind === "own" ? (
        <IconJuggler width={glyph} height={glyph} className="text-accent-hi" />
      ) : place.kind === "terminal" ? (
        <IconTerminal width={glyph} height={glyph} />
      ) : place.kind === "editor" ? (
        <IconCode width={glyph} height={glyph} />
      ) : (
        appLetters(place.app)
      )}
    </span>
  );
}

/** The profile's monogram on its colour (white letters, 4.5:1 or better). */
export function ProfileBadge({ name, size = 12 }: { name: string; size?: number }): JSX.Element {
  return (
    <span
      aria-hidden="true"
      title={name}
      data-testid="profile-badge"
      className="inline-flex shrink-0 items-center justify-center rounded-[4px] font-bold leading-none text-white ring-1 ring-canvas"
      style={{ width: size, height: size, background: profileColor(name), fontSize: Math.max(7, Math.round(size * 0.58)) }}
    >
      {profileMonogram(name)}
    </span>
  );
}

/** The app's icon with the profile's badge on its corner. */
export function PlaceIcon({ place, size = 22 }: { place: RailPlace; size?: number }): JSX.Element {
  return (
    <span className="relative inline-flex shrink-0" style={{ width: size, height: size }}>
      <AppIcon place={place} size={size} />
      {place.profile && (
        <span className="absolute -bottom-1 -right-1">
          <ProfileBadge name={place.profile} size={Math.round(size * 0.55)} />
        </span>
      )}
    </span>
  );
}

/** The profile as a chip in its colour (a place header, a Home tile). */
export function ProfileChip({ name }: { name: string }): JSX.Element {
  return (
    <span
      data-testid="profile-chip"
      className="inline-flex min-w-0 shrink items-center rounded-full px-1.5 py-px text-[10px] font-semibold leading-tight text-white"
      style={{ background: profileColor(name) }}
    >
      <span className="truncate">{name}</span>
    </span>
  );
}
