/**
 * AJ-E2104 — the new rail's rows and place headers.
 *
 * - A **Needs you** row: the app's icon (D-38) with the profile's dot, the
 *   title in bold, then "project · app · profile" and why it needs you
 *   in amber; its ↗ always shows.
 * - An **Open now** row, under its place: indented, with a left edge in
 *   the profile's colour, a state dot, the title, "project · viewed N min
 *   ago", and the state word in its colour. Runway shows only when low.
 * - A **place header**: a fold chevron, the app's icon and name, the
 *   profile's badge and name, and a count (a click opens the profile's page here), and
 *   its ↗ (Open), which brings the running app forward (never launches it).
 * - D-47: in Your order, a row and a place show a drag handle on hover
 *   and focus (⌥⌘↑ and ⌥⌘↓ are its twin); a row shows its pin there too.
 *   A row that needs you stays in its place, its state in amber.
 * - E31 (AJ-E3103): an Open now row is the one session row
 *   (`row/SessionRow`, the rail's densities), the same as on Home and
 *   every page's lists; its shell and parts are `row/`.
 *
 * The maintainer's review of 0.1.4: everything here is a way into AI
 * Juggler. A row's click or ↩ opens the session's page; its project's
 * name opens the project's page; its ↗ (or ⌘↩) is the one way out, to
 * the app and profile it runs in. Right-click, ⋯ or ⇧F10 open its menu,
 * which repeats these. Labels lead with the state (screen readers hear
 * "Asks you, …" first). Message text never shows here (D-37).
 */
import type { JSX } from "react";
import { PlaceMark } from "./PlaceMarks";
import { RowShell } from "./row/RowShell";
import { RowProject } from "./row/RowParts";

export { PlaceHeader } from "./PlaceHeader";
export type { Arrangeable } from "./RowTools";
export { usePrimary } from "./row/RowShell";
export { RowProject } from "./row/RowParts";
import { placeWords, type RailItem, type RailPlace } from "../lib/rail-items";
import { needWord } from "../lib/rail-items";
import { appColor } from "../lib/marks";
export { needWord } from "../lib/rail-items";

/** Where it is, in words: "project · Claude Code · Work · in Terminal". */
export function whereWords(item: RailItem): string {
  return [item.project, ...placeWords(item.place)].filter(Boolean).join(" · ");
}

interface RowProps {
  item: RailItem;
  selected: boolean;
  onSelect: (id: string) => void;
  now: number;
  /** Settings › Appearance › Density is Compact (AJ-E403): fewer lines,
   * the same words in its name. */
  compact?: boolean;
  /** 0.3.9: how deep it sits under the session it was started from. */
  depth?: number;
  /** That session's title, for a screen reader. */
  parent?: string | null;
}

/** A Needs you row. */
export function NeedRow({ compact = false, ...props }: RowProps): JSX.Element {
  const { item } = props;
  const word = needWord(item);
  const where = whereWords(item);
  if (compact) {
    // Two lines: what, and why it needs you (where is in its name).
    return (
      <RowShell {...props} label={`${word}, ${item.title}, ${where}`} shape="rounded-lg" className="h-[44px] items-center gap-2 pl-2">
        <span aria-hidden="true" className="flex shrink-0">
          <PlaceMark place={item.place} profile={item.accountLabel} size={18} />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span aria-hidden="true" className="truncate text-meta font-semibold leading-[18px] text-ink">
            {item.title}
          </span>
          <span aria-hidden="true" className="truncate text-label font-semibold leading-4 text-warning" data-testid="need-word">
            {word}
          </span>
        </span>
      </RowShell>
    );
  }
  return (
    <RowShell {...props} label={`${word}, ${item.title}, ${where}`} shape="rounded-lg" className="h-[62px] items-start gap-2.5 pb-2.5 pl-2 pt-2">
      <span aria-hidden="true" className="mt-0.5 flex shrink-0">
        <PlaceMark place={item.place} profile={item.accountLabel} size={24} />
      </span>
      {/* Three lines, each its own width (the review of 0.1.4: the state
          beside the place cut both to "ai-jug… · T…"): what, why it
          needs you, where. */}
      <span className="flex min-w-0 flex-1 flex-col">
        <span aria-hidden="true" className="truncate text-meta font-semibold leading-[18px] text-ink">
          {item.title}
        </span>
        <span aria-hidden="true" className="truncate text-label font-semibold leading-4 text-warning" data-testid="need-word">
          {word}
        </span>
        <RowProject item={item} rest={placeWords(item.place).join(" · ")} />
      </span>
    </RowShell>
  );
}

/** The left edge a place's rows carry: its App's colour. */
export function edgeOf(place: RailPlace | null): string | null {
  return place?.profile ? appColor(place.key) : null;
}
