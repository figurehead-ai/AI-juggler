/**
 * A box that scrolls under a fixed head, with the app's edge between them
 * (styles.css .scroll-edge: a hairline and a soft shadow at the top of its
 * view, shown only while it's scrolled down; the rail's list has the
 * same). Without it, a pool inspector's sections slid under its title and
 * status pill with nothing between, and the top line read as clipped.
 */
import { useState, type HTMLAttributes, type JSX, type ReactNode } from "react";

export function ScrollEdge({ children, className = "", ...rest }: { children: ReactNode; className?: string } & Omit<HTMLAttributes<HTMLDivElement>, "onScroll" | "className" | "children">): JSX.Element {
  const [down, setDown] = useState(false);
  return (
    <div {...rest} className={"scroll-edge " + className} data-scrolled={down ? "" : undefined} onScroll={(event) => setDown(event.currentTarget.scrollTop > 0)}>
      {children}
    </div>
  );
}
