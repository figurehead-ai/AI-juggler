/**
 * A page section's small heading (Home's Needs you, Running, Runway…;
 * the welcome's). Moved out of Home.tsx (the size budget).
 */
import type { JSX } from "react";

export function SectionTitle({ id, children }: { id: string; children: string }): JSX.Element {
  return (
    <h2 id={id} className="text-[11px] font-semibold uppercase tracking-wider text-muted">
      {children}
    </h2>
  );
}
