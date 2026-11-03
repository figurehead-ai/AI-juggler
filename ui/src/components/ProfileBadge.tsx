/**
 * D-38 with AJ-E2104 and AJ-E3206 — an App's mark: its profile's badge,
 * in the App's one colour (lib/marks: stored once per App, so it is the
 * same on every screen; one luminance, hues kept away from the status
 * colours), always beside an app's icon and never over it:
 *
 * - `dot`: a small dot, beside a name, or at the icon's corner with a
 *   ring in the colour of what it sits on (`PlaceMark`);
 * - `pill`: the profile's name on its colour, 12 px, white (4.5:1 or
 *   better on every colour), where a header names it.
 *
 * An Account is never drawn so: its mark is the round avatar
 * (`AccountAvatar`). Decorative but for the pill's words: the row or
 * header names the profile anyway.
 */
import type { JSX } from "react";
import { appColor, appInk, appSlotKnown, useMarks, type AppMark } from "../lib/marks";

export type ProfileBadgeVariant = "dot" | "pill";

export function ProfileBadge({
  app,
  name,
  variant = "dot",
  size = 8,
  ring,
}: {
  /** The App: its stored slot, or its id or rail key (lib/marks). */
  app: AppMark;
  /** The profile's name (the pill's words, the dot's tooltip). */
  name: string;
  variant?: ProfileBadgeVariant;
  /** A dot's diameter (6 to 8 px on an icon's corner). */
  size?: number;
  /** A dot's 1.5 px ring: the colour of the surface under it. */
  ring?: string;
}): JSX.Element {
  useMarks();
  const color = appColor(app);
  // Which App it was asked for, and whether its colour is its own.
  const mark = { "data-app": String(app), "data-colour-known": appSlotKnown(app) ? "true" : "false" };
  if (variant === "pill") {
    return (
      <span
        data-testid="profile-badge"
        data-variant="pill"
        data-color={color}
        {...mark}
        className="inline-flex min-w-0 shrink items-center rounded-full px-1.5 py-px text-label font-semibold leading-[16px]"
        style={{ background: color, color: appInk(app) }}
      >
        <span className="truncate">{name}</span>
      </span>
    );
  }
  return (
    <span
      aria-hidden="true"
      title={name}
      data-testid="profile-badge"
      data-variant="dot"
      data-color={color}
      {...mark}
      className="inline-block shrink-0 rounded-full"
      style={{ width: size, height: size, background: color, boxShadow: ring ? `0 0 0 1.5px ${ring}` : undefined }}
    />
  );
}
