/**
 * A session page's card title: sentence case, 15 px semibold, ink (the
 * readability scale; no small grey capitals).
 */
import type { JSX, ReactNode } from "react";

export function CardTitle({ children, id }: { children: ReactNode; id?: string }): JSX.Element {
  return (
    <h3 id={id} className="text-[15px] font-semibold text-ink">
      {children}
    </h3>
  );
}
