import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { IndicatorLegend } from "./indicator";

/** The `?` legend popover button + panel (AJ-E403 Req.6). The panel is
 * fixed-positioned above the button and portalled to the body: the
 * footer row clips overflow and the rail is its own stacking layer, so
 * a panel placed inside it opened out of sight (0.0.6 dogfood). */
export function LegendPopover({
  className = "ml-auto shrink-0",
  placement = "above",
}: {
  className?: string;
  /** "above" for the rail footer; "below" near the top of a screen (Home). */
  placement?: "above" | "below";
}) {
  const [open, setOpen] = useState(false);
  const panelId = `indicator-legend-${useId().replace(/:/g, "")}`;
  const [place, setPlace] = useState<{ left: number; top?: number; bottom?: number; maxHeight: number } | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const toggle = () => {
    if (open) {
      setOpen(false);
      return;
    }
    const rect = buttonRef.current?.getBoundingClientRect();
    if (rect) {
      const width = 320;
      const left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8));
      setPlace(
        placement === "below"
          ? { left, top: rect.bottom + 8, maxHeight: Math.max(160, window.innerHeight - rect.bottom - 16) }
          : { left, bottom: window.innerHeight - rect.top + 8, maxHeight: Math.max(160, rect.top - 16) },
      );
    }
    setOpen(true);
  };
  useEffect(() => {
    if (!open) return;
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!panelRef.current?.contains(target) && !buttonRef.current?.contains(target)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        buttonRef.current?.focus();
      }
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);
  return (
    <span className={className}>
      <button
        ref={buttonRef}
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        aria-label="Indicator legend"
        onClick={toggle}
        // A 24 px target around the 16 px glyph (WCAG 2.5.8), same layout.
        className="group -my-1 flex p-1 outline-none"
      >
        <span className="flex size-4 items-center justify-center rounded-full border border-hairline text-label font-bold text-muted transition-colors group-hover:bg-surface group-hover:text-ink group-focus-visible:ring-1 group-focus-visible:ring-accent/60">
          ?
        </span>
      </button>
      {open && place && createPortal(
        <div
          ref={panelRef}
          id={panelId}
          role="dialog"
          aria-label="What the icons mean"
          data-testid="indicator-legend"
          style={{ left: place.left, top: place.top, bottom: place.bottom, maxHeight: place.maxHeight }}
          className="fixed z-50 w-80 overflow-y-auto rounded-xl border border-hairline bg-surface p-3 shadow-card"
        >
          <IndicatorLegend />
        </div>,
        // At the top of the page: the rail is its own stacking layer, so
        // anything placed inside it paints under the work area.
        document.body,
      )}
    </span>
  );
}
