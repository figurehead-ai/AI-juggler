/**
 * 0.1.5 — the usage pages' icons, the ones their section titles carry
 * (the approved canvas: icons in headers). Their segmented choices (the
 * range, what a chart splits by, tokens or cost) are the shared
 * `controls/Segmented` (D-126: one look).
 */
import type { SVGProps } from "react";

function icon(d: string) {
  return function Icon(props: SVGProps<SVGSVGElement>) {
    return (
      <svg width={16} height={16} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...props}>
        <path d={d} />
      </svg>
    );
  };
}

export const IconChart = icon("M4 19V5M4 19h16M8 15l4-4 3 3 5-6");
export const IconDonut = icon("M12 3a9 9 0 1 0 9 9h-9zM14 3.3A9 9 0 0 1 20.7 10H14z");
export const IconGauge = icon("M12 14l4-4M4 18a9 9 0 1 1 16 0");
export const IconClock = icon("M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3 2");
export const IconFolder = icon("M3 7h6l2 2h10v10H3z");
export const IconList = icon("M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01");
export const IconRoute = icon("M6 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM18 9a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM6 15V9a4 4 0 0 1 4-4h6M18 9v6a4 4 0 0 1-4 4H8");
export const IconTokens = icon("M4 7h16M4 12h10M4 17h7");
export const IconMoney = icon("M12 3v18M17 7H9.5a3 3 0 0 0 0 6h5a3 3 0 0 1 0 6H6");
export const IconPlay = icon("M8 5l11 7-11 7z");
/** An Account: what you pay for (a card), never a person or an App. */
export const IconAccount = icon("M3 7a1 1 0 0 1 1-1h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1zM3 10h18M7 15h3");
export const IconUser = icon("M16 19v-1a4 4 0 0 0-8 0v1M12 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6z");
export const IconRules = icon("M9 12h6M9 16h6M7 3h7l5 5v13H7z");
export const IconGit = icon("M6 3v12M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM18 9a9 9 0 0 1-9 9");
export const IconLayers = icon("M12 3l9 5-9 5-9-5zM3 13l9 5 9-5");
export const IconSettings = icon("M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19 12h2M3 12h2M12 3v2M12 19v2M17 7l1.5-1.5M5.5 18.5L7 17M17 17l1.5 1.5M5.5 5.5L7 7");
