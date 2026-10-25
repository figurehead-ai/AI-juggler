/**
 * D-38 with AJ-E2104 — a profile's badge, in the profile's own colour
 * (lib/profile-marks: one luminance, hues kept away from the status
 * colours), always beside an app's icon and never over it:
 *
 * - `dot`: a small dot, beside a name, or at the icon's corner with a
 *   ring in the colour of what it sits on (`PlaceMark`);
 * - `pill`: the profile's name on its colour, 12 px, white (4.5:1 or
 *   better on every profile colour), where a header names it.
 *
 * Decorative but for the pill's words: the row or header names the
 * profile anyway.
 */
import type { JSX } from "react";
import { profileColor } from "../lib/profile-marks";

export type ProfileBadgeVariant = "dot" | "pill";

export function ProfileBadge({
  name,
  variant = "dot",
  size = 8,
  ring,
}: {
  name: string;
  variant?: ProfileBadgeVariant;
  /** A dot's diameter (6 to 8 px on an icon's corner). */
  size?: number;
  /** A dot's 1.5 px ring: the colour of the surface under it. */
  ring?: string;
}): JSX.Element {
  if (variant === "pill") {
    return (
      <span
        data-testid="profile-badge"
        data-variant="pill"
        className="inline-flex min-w-0 shrink items-center rounded-full px-1.5 py-px text-[12px] font-semibold leading-[16px] text-white"
        style={{ background: profileColor(name) }}
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
      className="inline-block shrink-0 rounded-full"
      style={{ width: size, height: size, background: profileColor(name), boxShadow: ring ? `0 0 0 1.5px ${ring}` : undefined }}
    />
  );
}
