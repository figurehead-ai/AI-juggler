import type { JSX } from "react";

/** The rail's fold chevron: one look for every disclosure, the app's
 * Disclosure (a button), turned while `open`. No native `<details>` is
 * left to turn it (the native-controls lint keeps them out). Turned at
 * once, never by a transition: WebKit pauses one in a window not shown,
 * and the fold would read closed while open (as the tabs did, 0407f5105). */
export function FoldChevron({ open }: { open: boolean }): JSX.Element {
  return (
    <svg aria-hidden="true" width="12" height="12" viewBox="0 0 10 10" className={`shrink-0 text-muted ${open ? "rotate-90" : ""}`}>
      <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
