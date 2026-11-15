/**
 * AJ-E2104 — the new rail's rows and place headers.
 *
 * - A **Needs you** row (D-117): the App's dot, the title in bold, why
 *   it needs you in its level's colour, then "project · app · profile";
 *   its ↗ on hover, focus or when selected.
 * - An **Open now** row, under its place: indented, a state mark, the
 *   title, "project · viewed N min ago", and the state word (a need's in
 *   ink beside its amber mark, D-117). Runway shows only when low.
 * - A **place header**: a fold chevron, the app's icon and name, the
 *   profile's badge and name, and a count (a click opens the profile's page here), and
 *   its ↗ (Open), which brings the running app forward (never launches it).
 * - D-47: in Your order, a row and a place show a drag handle on hover
 *   and focus (⌥⌘↑ and ⌥⌘↓ are its twin); a row shows its pin there too.
 *   A row that needs you stays in its place (D-117: its word in ink by
 *   its level's mark; the loud copy is its Needs you row).
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
import { ProfileBadge } from "./ProfileBadge";
import { appTone, itemLevel, levelTone } from "../lib/attention";
import { RowShell } from "./row/RowShell";
import { RowProject } from "./row/RowParts";

export { PlaceHeader } from "./PlaceHeader";
export type { Arrangeable } from "./RowTools";
export { usePrimary } from "./row/RowShell";
export { RowProject } from "./row/RowParts";
import { placeWords, type RailItem } from "../lib/rail-items";
import { needWord } from "../lib/rail-items";
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

/** A Needs you row (D-117): the App's 6 px dot (in full colour while it
 * needs you, muted for a note), the title in semibold ink, why in its
 * level's colour — amber at L2 (a hollow ring before a guess), red with
 * its ✕ on the L3 tint — or a review's ✓ and muted words (L1), then
 * where. Its ↗ shows on hover, focus or when selected. */
export function NeedRow({ compact = false, ...props }: RowProps): JSX.Element {
  const { item } = props;
  const word = needWord(item);
  const where = whereWords(item);
  const level = itemLevel(item);
  const lit = level >= 2 || props.selected;
  const tint = level === 3 ? " bg-danger-soft" : "";
  const dot = (
    <span aria-hidden="true" className={"flex shrink-0 " + (compact ? "" : "mt-[6px]")} data-testid="place-mark">
      <ProfileBadge app={item.place.key} name={item.place.profile ?? item.accountLabel} size={6} className={appTone(lit)} />
    </span>
  );
  const why = (
    <span aria-hidden="true" className={"flex min-w-0 items-center gap-1 truncate text-label leading-4 " + (level >= 2 ? "font-semibold " + levelTone(level).text : "text-muted")} data-testid="need-word" data-level={level}>
      {level === 3 && <span className="shrink-0">✕</span>}
      {level === 2 && item.probably && <span className="level-mark" data-level={2} data-probably="true" />}
      {level < 2 && item.need === "review" && <span className="shrink-0 text-success">✓</span>}
      <span className="truncate">{word}</span>
    </span>
  );
  const data = { "data-level": String(level), "data-need": item.need ?? undefined, "data-state": item.state };
  if (compact) {
    // Two lines: what, and why it needs you (where is in its name).
    return (
      <RowShell {...props} data={data} label={`${word}, ${item.title}, ${where}`} shape="rounded-lg" className={"h-[44px] items-center gap-2 pl-2" + tint}>
        {dot}
        <span className="flex min-w-0 flex-1 flex-col">
          <span aria-hidden="true" className="truncate text-meta font-semibold leading-[18px] text-ink">
            {item.title}
          </span>
          {why}
        </span>
      </RowShell>
    );
  }
  return (
    <RowShell {...props} data={data} label={`${word}, ${item.title}, ${where}`} shape="rounded-lg" className={"h-[62px] items-start gap-2.5 pb-2.5 pl-2.5 pt-2" + tint}>
      {dot}
      {/* Three lines, each its own width (the review of 0.1.4: the state
          beside the place cut both to "ai-jug… · T…"): what, why it
          needs you, where. */}
      <span className="flex min-w-0 flex-1 flex-col">
        <span aria-hidden="true" className="truncate text-meta font-semibold leading-[18px] text-ink">
          {item.title}
        </span>
        {why}
        <RowProject item={item} rest={placeWords(item.place).join(" · ")} />
      </span>
    </RowShell>
  );
}
