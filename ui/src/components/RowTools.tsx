/**
 * D-45 — what a row or place of Open now shows to be arranged in Your
 * order: the drag handle (on hover and focus), and the line where a drag
 * would land.
 */
import type { JSX, PointerEvent as ReactPointerEvent } from "react";
import { IconGrip } from "./icons-places";

/** D-45: a row or place that can be arranged (Your order), and how a drag
 * over it looks. */
export interface Arrangeable {
  /** The group (rows) or its own key (places). */
  group: string;
  /** Your order is on: the handle shows (otherwise ⌥⌘↑/↓ only say how). */
  enabled: boolean;
  grip: (event: ReactPointerEvent<HTMLElement>) => void;
  /** A drag would land just before or after it. */
  drop: "before" | "after" | null;
  dragging: boolean;
}

export const DROP: Record<"before" | "after", string> = {
  before: " shadow-[inset_0_2px_0_0_var(--color-accent)]",
  after: " shadow-[inset_0_-2px_0_0_var(--color-accent)]",
};

/** The handle a row or place is dragged by: the pointer's only; the
 * keyboard's twin is ⌥⌘↑ and ⌥⌘↓ on the focused one. */
export function Grip({ arrange, label }: { arrange: Arrangeable; label: string }): JSX.Element {
  return (
    <span
      aria-hidden="true"
      title={`Drag to move ${label} (or ⌥⌘↑ and ⌥⌘↓)`}
      data-testid="order-grip"
      onPointerDown={arrange.grip}
      className="pointer-events-auto flex h-6 w-4 shrink-0 cursor-grab touch-none items-center justify-center rounded-md border border-hairline bg-surface text-muted hover:bg-surface-3 hover:text-ink active:cursor-grabbing"
    >
      <IconGrip width={12} height={12} />
    </span>
  );
}
