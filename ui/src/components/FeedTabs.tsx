/**
 * 0.5.2 (PR #257's Happo review): the feed's tabs on one row, always. At
 * 1200 and 1024 wide "This Mac" wrapped to a line of its own under the
 * other six. The row measures its own width (a ResizeObserver, never the
 * window's) and the tabs that don't fit go into the toolbar's ⋯ menu,
 * from the end ("This Mac" first, as the story-feed mock orders them),
 * never wrapping. The chosen tab always stays in the row: when it would be
 * one to go, the last tab that fits gives its place.
 *
 * PR #257's second Happo check: at chrome-min (1024) the row read "This
 * Ma", cut against the ⋯. The cause: Happo draws the page's captured DOM in
 * its own browser, with its own system fonts (the app uses the native font
 * stack), and runs none of its script, so a row fitted to the capturing
 * browser's narrower glyphs overflowed there, and the row's
 * `overflow: hidden` on one line cut the last tab in half. Now the row
 * wraps inside a box exactly one tab high: a tab that doesn't fit, however
 * the fonts measure, drops whole onto the hidden second line, never partly
 * shown; the script still fits the row (after the fonts load, on every
 * resize, whole pixels with a pixel spare) and lists every tab not on the
 * first line in ⋯, a wrapped one too.
 */
import { useLayoutEffect, useRef, useState, type JSX, type RefObject } from "react";
import { SELECTED } from "./controls/selected";
import { IconAlert, IconSessions, IconStateQuota } from "./icons";
import { IconComputer, IconPool } from "./icons-actions";
import { IconBoard, IconTasks } from "./icons-places";
import { TABS, type Tab } from "../lib/feed-chips";

/** Each tab's icon, for its line in the ⋯ menu (every menu line carries
 * its icon, F60). */
export const TAB_ICONS: Record<Tab, typeof IconAlert> = {
  all: IconBoard,
  needs: IconAlert,
  sessions: IconSessions,
  tasks: IconTasks,
  pools: IconPool,
  switches: IconStateQuota,
  mac: IconComputer,
};

const TAB = "shrink-0 rounded-full px-[11px] py-[5px] text-meta outline-none focus-visible:ring-1 focus-visible:ring-accent/60";
/** The row: one tab high (the meta line and the tab's padding) plus 2 px
 * all round for a focus ring, drawn 2 px out so the tabs sit where they
 * did; what wraps past it is out of sight. */
const ROW = "relative -mx-0.5 -my-0.5 flex h-[calc(var(--text-meta)*var(--text-meta--line-height)+14px)] min-w-0 flex-1 flex-wrap content-start items-start gap-1.5 overflow-hidden border-0 p-0.5";
/** The row's gap between tabs, in px (`gap-1.5`). */
const GAP = 6;

/** Which tabs show (in order) for `room` px, each `widths[i]` wide, the
 * chosen one (`chosen`, an index) always among them; the rest go to ⋯. */
export function fitTabs(widths: readonly number[], room: number, chosen: number): number[] {
  const span = (indices: readonly number[]) => indices.reduce((sum, i) => sum + (widths[i] ?? 0), 0) + GAP * Math.max(0, indices.length - 1);
  const all = widths.map((_, i) => i);
  for (let count = all.length; count > 0; count -= 1) {
    const first = all.slice(0, count);
    const shown = first.includes(chosen) || chosen < 0 ? first : [...all.slice(0, count - 1), chosen];
    if (span(shown) <= room) return shown;
  }
  return chosen >= 0 ? [chosen] : [];
}

/** Each tab's width, measured in an invisible copy of the row as it is
 * drawn (the chosen one semibold), and the room the row has; measured
 * again when either resizes (another tab chosen changes the copy). */
function useFit(row: RefObject<HTMLElement | null>, measure: RefObject<HTMLElement | null>): { widths: number[]; room: number } {
  const [fit, setFit] = useState<{ widths: number[]; room: number }>({ widths: [], room: Number.POSITIVE_INFINITY });
  useLayoutEffect(() => {
    const read = () => {
      // Whole pixels, a pixel spare: a sub-pixel overhang hides the tab.
      const widths = [...(measure.current?.children ?? [])].map((child) => Math.ceil(child.getBoundingClientRect().width));
      const box = row.current;
      const room = box ? Math.floor(box.clientWidth - 4) - 1 : Number.POSITIVE_INFINITY;
      setFit((was) => (was.room === room && was.widths.join() === widths.join() ? was : { widths, room }));
    };
    read();
    // The fonts' own widths once they've loaded (a fallback's first).
    void document.fonts?.ready.then(read);
    if (typeof ResizeObserver === "undefined" || !row.current) return undefined;
    const observer = new ResizeObserver(read);
    observer.observe(row.current);
    if (measure.current) observer.observe(measure.current);
    return () => observer.disconnect();
  }, [row, measure]);
  return fit;
}

/** The tab row; `onHidden` hears which tabs went to the ⋯ menu. */
export function FeedTabs({ tab, onTab, onHidden }: { tab: Tab; onTab: (value: Tab) => void; onHidden: (hidden: Tab[]) => void }): JSX.Element {
  const row = useRef<HTMLFieldSetElement | null>(null);
  const measure = useRef<HTMLDivElement | null>(null);
  const chosen = TABS.findIndex((one) => one.value === tab);
  const { widths, room } = useFit(row, measure);
  // Not laid out yet (or no layout at all, as in a test's DOM): every tab.
  const measured = widths.length === TABS.length && Number.isFinite(room) && room > 0 && widths.some((w) => w > 0);
  const shown = measured ? fitTabs(widths, room, chosen) : TABS.map((_, i) => i);
  // A tab the row wrapped (its fonts wider than measured) is in ⋯ too.
  const [wrapped, setWrapped] = useState<Tab[]>([]);
  const shownKey = shown.join();
  useLayoutEffect(() => {
    const tabs = [...(row.current?.querySelectorAll<HTMLElement>("button[data-tab]") ?? [])];
    const top = tabs[0]?.offsetTop ?? 0;
    const off = shownKey && room > 0 ? TABS.filter((one) => tabs.some((button) => button.dataset["tab"] === one.value && button.offsetTop > top)).map((one) => one.value) : [];
    if (off.join() !== wrapped.join()) setWrapped(off);
  }, [shownKey, room, wrapped]);
  const hidden = TABS.filter((one, i) => !shown.includes(i) || wrapped.includes(one.value)).map((one) => one.value);
  const key = hidden.join();
  const told = useRef("");
  useLayoutEffect(() => {
    if (told.current === key) return;
    told.current = key;
    onHidden(hidden);
  });
  return (
    <fieldset ref={row} className={ROW} data-testid="feed-chip" data-hidden={key}>
      <legend className="sr-only">Show in the feed</legend>
      {shown.map((i) => {
        const one = TABS[i]!;
        return (
          <button
            key={one.value}
            type="button"
            aria-pressed={tab === one.value}
            onClick={() => onTab(one.value)}
            // PAT-SURF-3: the chosen tab wears the selected look (the
            // attention budget's rule: a selection is never a loud fill).
            className={TAB + (tab === one.value ? ` ${SELECTED}` : " text-muted hover:bg-surface-2 hover:text-ink")}
            data-testid={`feed-chip-${one.value}`}
            data-tab={one.value}
          >
            {one.label}
          </button>
        );
      })}
      {/* Every tab as the row would draw it, unseen: their widths. */}
      <div ref={measure} aria-hidden="true" className="pointer-events-none invisible absolute top-0 left-0 flex gap-1.5">
        {TABS.map((one) => (
          <span key={one.value} className={TAB + (one.value === tab ? " font-semibold" : "")}>
            {one.label}
          </span>
        ))}
      </div>
    </fieldset>
  );
}
