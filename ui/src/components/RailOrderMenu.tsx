/**
 * D-47 — Open now's order menu, on its header ("Your order ▾"): Your
 * order, Latest activity or Name, each with what it does, then Reset my
 * order. AJ-E2106: an icon on every line, two groups. Settings › General
 * offers the same choice (`RailOrderChoice`).
 */
import { useState, type JSX } from "react";
import { IconOrderLatest, IconOrderName, IconOrderYours } from "./icons-places";
import { IconReplay } from "./icons-actions";
import { MenuItem, MenuSeparator, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { showToast } from "./Toast";
import { ORDER_MODES, ORDER_WORDS, type OrderMode } from "../lib/rail-order";
import { resetMyOrder, restoreOrder, setOrderMode, useRailOrder } from "../lib/rail-order-store";

export const ORDER_ICONS: Record<OrderMode, typeof IconOrderYours> = {
  yours: IconOrderYours,
  latest: IconOrderLatest,
  name: IconOrderName,
};

/** Reset my order, with Undo (AJ-E1503). */
export function resetWithUndo(): void {
  const before = resetMyOrder();
  showToast({
    id: "rail-order-reset",
    message: "Your order is reset.",
    detail: "Rows are in the order they opened; places in their usual order.",
    undo: { onUndo: () => restoreOrder(before) },
    testId: "rail-order-undo",
  });
}

export function RailOrderMenu(): JSX.Element {
  const order = useRailOrder();
  const [at, setAt] = useState<MenuAt | null>(null);
  const label = ORDER_WORDS[order.mode].label;
  const pick = (mode: OrderMode) => {
    setAt(null);
    setOrderMode(mode);
  };
  return (
    <>
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={at !== null}
        aria-label={`Open now's order: ${label}`}
        data-testid="rail-order-button"
        onClick={(event) => setAt(menuPoint(event))}
        className="flex min-h-6 min-w-0 items-center gap-1 rounded-md px-1.5 text-label font-medium text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        <span className="truncate">{label}</span>
        <svg aria-hidden="true" width="8" height="8" viewBox="0 0 10 10" className="shrink-0">
          {/* A filled ▾ (a menu), unlike the fold chevron beside it. */}
          <path d="M2 3.5 H8 L5 7 Z" fill="currentColor" />
        </svg>
      </button>
      {at && (
        <RowMenu heading="Order of Open now" ariaLabel="Order of Open now" at={at} onClose={() => setAt(null)}>
          {ORDER_MODES.map((mode) => {
            const Icon = ORDER_ICONS[mode];
            const words = ORDER_WORDS[mode];
            const chosen = order.mode === mode;
            return (
              <MenuItem key={mode} checked={chosen} testId={`order-${mode}`} onClick={() => pick(mode)}>
                <span className="flex items-start gap-2">
                  <Icon width={14} height={14} aria-hidden="true" className="mt-0.5 shrink-0" />
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span className={chosen ? "font-semibold" : undefined}>{words.label}</span>
                    <span className="text-label leading-snug text-muted">{words.what}</span>
                  </span>
                  <span aria-hidden="true" className={"mt-0.5 shrink-0 text-accent-hi " + (chosen ? "" : "invisible")}>
                    ✓
                  </span>
                </span>
              </MenuItem>
            );
          })}
          <MenuSeparator />
          <MenuItem
            testId="order-reset"
            onClick={() => {
              setAt(null);
              resetWithUndo();
            }}
          >
            <span className="flex items-center gap-2">
              <IconReplay width={14} height={14} aria-hidden="true" className="shrink-0" />
              <span className="min-w-0 flex-1">Reset my order</span>
            </span>
          </MenuItem>
        </RowMenu>
      )}
    </>
  );
}
