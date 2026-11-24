/**
 * 0.5.1 F57 — the top bar keeps one row: the page tabs that don't fit go
 * behind More, from the last (Fleet, History, Usage…), by the room the
 * bar has once its own parts are counted (‹ ›, and the cluster at its
 * smallest: This Mac's words give way before a tab does). The fold point
 * counts the tabs the bar shows, Fleet's too, and the cluster's words
 * (its clean-up count, New session): it wrapped onto a second row at
 * 1024 with Fleet and at 860 with the list open. Measured, not guessed
 * from the window's width, so it holds whatever the fonts, the counts or
 * the user's zoom.
 */
import { useLayoutEffect, useRef, useState, type RefObject } from "react";
import { unshrunk } from "./use-give-way";

/** How many of the tabs (their `widths`, in order, `gap` apart) show in
 * `room` px: all when all fit, else as many as fit beside More (`more`
 * px), never fewer than `keep`. */
export function tabsThatFit(widths: readonly number[], room: number, gap: number, more: number, keep = 1): number {
  const all = widths.reduce((sum, width) => sum + width, 0) + gap * Math.max(0, widths.length - 1);
  // A hundredth of a pixel: what layout rounds.
  if (all <= room + 0.01) return widths.length;
  let used = more;
  let shown = 0;
  for (const width of widths) {
    if (used + gap + width > room + 0.01) break;
    used += gap + width;
    shown += 1;
  }
  return Math.min(widths.length, Math.max(keep, shown));
}

function px(value: string): number {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

/** What the cluster needs with what can give way given way. */
function clusterNeed(cluster: HTMLElement): number {
  let need = unshrunk(cluster);
  const gap = px(getComputedStyle(cluster).columnGap);
  for (const part of cluster.querySelectorAll<HTMLElement>("[data-give-at]:not([data-gone])")) {
    const width = part.getBoundingClientRect().width;
    if (width > 0) need -= width + gap;
  }
  return need;
}

/** The tabs shown (the rest behind More) for the bar `header`, its tabs
 * in `nav` (each `[data-nav-tab]`, a folded one kept measurable but out
 * of the row), More (`[data-nav-more]`) and the cluster `cluster`. */
export function useTabFold(count: number, header: RefObject<HTMLElement | null>, nav: RefObject<HTMLElement | null>, cluster: RefObject<HTMLElement | null>): number {
  const [shown, setShown] = useState(count);
  const moreWidth = useRef(0);
  useLayoutEffect(() => {
    const bar = header.current;
    const tabs = nav.current;
    const rest = cluster.current;
    if (!bar || !tabs || !rest) return undefined;
    const fit = () => {
      const style = getComputedStyle(bar);
      const content = bar.clientWidth - px(style.paddingLeft) - px(style.paddingRight);
      const between = px(style.columnGap);
      // The bar's own parts beside the tabs: ‹ › and the cluster.
      let fixed = clusterNeed(rest) + between;
      for (const child of bar.children) {
        if (!(child instanceof HTMLElement) || child === tabs || child === rest || getComputedStyle(child).position === "absolute") continue;
        fixed += child.getBoundingClientRect().width + between;
      }
      const widths = [...tabs.querySelectorAll<HTMLElement>("[data-nav-tab]")].map((tab) => tab.getBoundingClientRect().width);
      const more = tabs.querySelector<HTMLElement>("[data-nav-more]")?.getBoundingClientRect().width ?? 0;
      if (more > 0) moreWidth.current = more;
      const next = tabsThatFit(widths, content - fixed, px(getComputedStyle(tabs).columnGap), moreWidth.current || 64, 2);
      setShown((was) => (was === next ? was : next));
    };
    fit();
    if (typeof ResizeObserver === "undefined") return undefined;
    const observer = new ResizeObserver(fit);
    observer.observe(bar);
    for (const part of rest.children) observer.observe(part);
    // The cluster's words change (a count shows), or a tab comes or goes
    // (Fleet once a Mac is paired).
    const words = new MutationObserver(fit);
    words.observe(rest, { childList: true, characterData: true, subtree: true });
    words.observe(tabs, { childList: true });
    return () => {
      observer.disconnect();
      words.disconnect();
    };
  }, [header, nav, cluster]);
  return Math.min(shown, count);
}
