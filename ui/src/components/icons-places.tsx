/**
 * AJ-E2104 — the rail's own icons: its View button, Open now's window,
 * a place's Open, History, and the app kinds a place can be.
 */
import type { SVGProps } from "react";
import { base } from "./icons";

/** The View: lines of three lengths (Linear's display options). */
export function IconView(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 6h16M7 12h10M10 18h4" />
    </svg>
  );
}

/** Open now: a window. */
export function IconWindow(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="3.5" y="5" width="17" height="14" rx="2" />
      <path d="M3.5 9h17M6.5 7h.01M8.5 7h.01" />
    </svg>
  );
}

/** Open: up and out, to the app. */
export function IconUpRight(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M8 16 16 8M9 8h7v7" />
    </svg>
  );
}

/** History: a clock with the hand going back. */
export function IconHistory(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3L4.5 9" />
      <path d="M4.5 4.5V9H9M12 8v4l3 2" />
    </svg>
  );
}

/** An editor: code brackets. */
export function IconCode(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m8 7-5 5 5 5M16 7l5 5-5 5" />
    </svg>
  );
}
