/**
 * AJ-E1545 — the app's real icon, drawn by hand as one SVG to match
 * `crates/juggler-app/icons/icon.png` (no bitmap in the bundle): the dark
 * tile, the arc, and three balls on it — lavender on top, indigo left,
 * cyan right — each with its glow. Every large brand mark uses it (the
 * Welcome's hero, the setup wizard's header, Bot's coach marks); small
 * sizes use `IconJuggler`, the same arc and balls without the tile.
 *
 * Decorative: it says nothing a heading beside it doesn't.
 */
import { useId, type JSX } from "react";

export function AppMark({ size = 72, className = "" }: { size?: number; className?: string }): JSX.Element {
  const id = useId().replace(/:/g, "");
  const ref = (name: string) => `url(#${id}-${name})`;
  return (
    <svg aria-hidden="true" width={size} height={size} viewBox="0 0 100 100" className={className} data-testid="app-mark">
      <defs>
        <linearGradient id={`${id}-tile`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="var(--color-brand-tile-hi)" />
          <stop offset="1" stopColor="var(--color-brand-tile-lo)" />
        </linearGradient>
        <linearGradient id={`${id}-arc`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="var(--color-brand-arc)" />
          <stop offset="0.6" stopColor="var(--color-brand-arc)" />
          <stop offset="1" stopColor="var(--color-brand-arc-end)" />
        </linearGradient>
        {(
          [
            ["top", "var(--color-brand-top)", 0.5],
            ["left", "var(--color-brand-left)", 0.42],
            ["right", "var(--color-brand-right)", 0.5],
          ] as const
        ).map(([name, colour, strength]) => (
          <radialGradient key={name} id={`${id}-glow-${name}`}>
            <stop offset="0" stopColor={colour} stopOpacity={strength} />
            <stop offset="0.45" stopColor={colour} stopOpacity={strength * 0.45} />
            <stop offset="1" stopColor={colour} stopOpacity="0" />
          </radialGradient>
        ))}
      </defs>
      {/* The tile: a continuous-corner square, as macOS draws app icons. */}
      <rect x="0" y="0" width="100" height="100" rx="22.5" fill={ref("tile")} />
      {/* Its edge: faint on a large mark; at small sizes, enough to stand off a dark surface. */}
      <rect x="1" y="1" width="98" height="98" rx="21.5" fill="none" stroke="white" strokeOpacity={size <= 40 ? 0.22 : 0.06} strokeWidth={size <= 40 ? 2.5 : 1} />
      {/* The glows, under the arc and the balls. */}
      <circle cx="50" cy="35.8" r="22" fill={ref("glow-top")} />
      <circle cx="25.2" cy="72.6" r="19" fill={ref("glow-left")} />
      <circle cx="74.8" cy="72.6" r="20" fill={ref("glow-right")} />
      <path d="M25.9 66C28.4 46.5 37.6 36 50 35.8C62.4 36 71.6 46.5 74.1 66" fill="none" stroke={ref("arc")} strokeWidth="4.1" strokeLinecap="round" />
      <circle cx="50" cy="35.8" r="9.2" fill="var(--color-brand-top)" />
      <circle cx="25.2" cy="72.6" r="8.5" fill="var(--color-brand-left)" />
      <circle cx="74.8" cy="72.6" r="8.5" fill="var(--color-brand-right)" />
    </svg>
  );
}
