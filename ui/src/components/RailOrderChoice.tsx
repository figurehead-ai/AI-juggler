/**
 * D-47 — Settings › General › Appearance: Open now's order, the same
 * choice as the menu on its header, and Reset my order.
 */
import type { JSX } from "react";
import { Button } from "./primitives";
import { ORDER_ICONS, resetWithUndo } from "./RailOrderMenu";
import { ORDER_MODES, ORDER_WORDS } from "../lib/rail-order";
import { setOrderMode, useRailOrder } from "../lib/rail-order-store";

export function RailOrderChoice(): JSX.Element {
  const order = useRailOrder();
  return (
    <fieldset className="mt-2 flex flex-col gap-2" data-testid="settings-rail-order">
      <legend className="mb-1 text-xs font-medium text-ink">Order of Open now</legend>
      <div className="flex flex-wrap gap-2">
        {ORDER_MODES.map((mode) => {
          const Icon = ORDER_ICONS[mode];
          return (
            <label
              key={mode}
              className={"flex max-w-[16rem] items-start gap-2 rounded-lg border px-3 py-1.5 text-sm " + (order.mode === mode ? "border-accent/60 bg-accent-soft" : "border-hairline")}
            >
              <input type="radio" name="rail-order" className="mt-1" checked={order.mode === mode} onChange={() => setOrderMode(mode)} data-testid={`settings-order-${mode}`} />
              <Icon width={14} height={14} aria-hidden="true" className="mt-1 shrink-0 text-muted" />
              <span className="flex flex-col">
                <span>{ORDER_WORDS[mode].label}</span>
                <span className="text-label text-muted">{ORDER_WORDS[mode].what}</span>
              </span>
            </label>
          );
        })}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="secondary" onClick={resetWithUndo} data-testid="settings-order-reset">
          Reset my order
        </Button>
        <span className="text-label text-muted">Rows go back to the order they opened in; pins stay.</span>
      </div>
    </fieldset>
  );
}
