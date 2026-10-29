/**
 * D-47 — Settings › General › Appearance: Open now's order, the same
 * choice as the menu on its header, and Reset my order (its Undo toast
 * says what it did). 0.3: equal cards on one grid.
 */
import type { JSX } from "react";
import { Button } from "./primitives";
import { ORDER_ICONS, resetWithUndo } from "./RailOrderMenu";
import { ChoiceOption } from "./controls";
import { ChoiceGroup } from "./AppearanceCard";
import { ORDER_MODES, ORDER_WORDS } from "../lib/rail-order";
import { setOrderMode, useRailOrder } from "../lib/rail-order-store";

export function RailOrderChoice(): JSX.Element {
  const order = useRailOrder();
  return (
    <ChoiceGroup
      legend="Order of Open now"
      hint="Reset puts rows back in the order they opened; pins stay."
      testId="settings-rail-order"
      end={
        <Button size="sm" variant="secondary" onClick={resetWithUndo} data-testid="settings-order-reset">
          Reset my order
        </Button>
      }
    >
      {ORDER_MODES.map((mode) => (
        <ChoiceOption
          key={mode}
          type="radio"
          look="card"
          name="rail-order"
          icon={ORDER_ICONS[mode]}
          label={ORDER_WORDS[mode].label}
          hint={ORDER_WORDS[mode].what}
          checked={order.mode === mode}
          onChange={() => setOrderMode(mode)}
          data-testid={`settings-order-${mode}`}
        />
      ))}
    </ChoiceGroup>
  );
}
