/**
 * AJ-E2104 — the rail's own icons: its View button, Open now's window,
 * a place's Open, History, and the app kinds a place can be; D-47's drag
 * handle and the three orders.
 */
import type { SVGProps } from "react";
import { base } from "./icons";

/** D-47: a drag handle (six dots): a row or place is dragged by it. */
export function IconGrip(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M9 6h.01M15 6h.01M9 12h.01M15 12h.01M9 18h.01M15 18h.01" strokeWidth={3} />
    </svg>
  );
}

/** Your order: a list arranged by hand. */
export function IconOrderYours(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M10 6h10M10 12h10M10 18h10" />
      <path d="M4.5 6h.01M4.5 12h.01M4.5 18h.01" strokeWidth={3} />
    </svg>
  );
}

/** Latest activity: a clock. */
export function IconOrderLatest(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 8v4l2.5 2.5" />
    </svg>
  );
}

/** Name: A to Z. */
export function IconOrderName(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M3.5 18 7 8l3.5 10M4.8 14.5h4.4" />
      <path d="M14 8h6l-6 10h6" />
    </svg>
  );
}

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
