/**
 * What goes along with a switch, in the backend's plain words, each with
 * its line on hover, on focus and under "What does this mean?" (the
 * maintainer's walk, 2026-10-27: "rebuilt by target is too vague, what
 * target"). Only what stays behind is a warning.
 */
import { useId, type JSX } from "react";
import type { CarryItemData } from "../../lib/bridge";
import { GUIDE } from "../../lib/switch-words";
import { Explain } from "./Explain";

function Items({ items, testId, tone = "text-ink" }: { items: CarryItemData[]; testId: string; tone?: string }): JSX.Element {
  const id = useId();
  return (
    <ul className={"flex flex-wrap gap-1.5 " + tone} data-testid={testId}>
      {items.map((item, index) => (
        <li key={item.label}>
          <span
            tabIndex={0}
            title={item.hint}
            aria-describedby={`${id}-${index}`}
            className="inline-flex min-h-6 items-center rounded-md border border-hairline bg-surface-2 px-2 text-meta outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            {item.label}
          </span>
          <span id={`${id}-${index}`} className="sr-only">
            {item.hint}
          </span>
        </li>
      ))}
    </ul>
  );
}

export function CarryList({ items, target, tool }: { items: CarryItemData[]; target: string; tool: string }): JSX.Element {
  const moves = items.filter((item) => item.how === "moves");
  const recreated = items.filter((item) => item.how === "recreated");
  const stays = items.filter((item) => item.how === "stays");
  return (
    <div className="flex flex-col gap-3" data-testid="switch-carry">
      {moves.length > 0 && (
        <section className="flex flex-col gap-1.5" data-testid="switch-carry-carried">
          <h4 className="text-body font-semibold text-ink">Goes along</h4>
          <Items items={moves} testId="switch-carry-moves" />
        </section>
      )}
      {recreated.length > 0 && (
        <section className="flex flex-col gap-1.5" data-testid="switch-carry-rebuilt">
          <h4 className="text-body font-semibold text-ink">
            Recreated on {target} ({tool}), from its latest version
          </h4>
          <Items items={recreated} testId="switch-carry-recreated" />
        </section>
      )}
      {stays.length > 0 && (
        <section className="flex flex-col gap-1.5 rounded-lg border border-warning/40 px-3 py-2" data-testid="switch-carry-dropped">
          <h4 className="text-body font-semibold text-warning">Stays behind: the new session starts without it</h4>
          <Items items={stays} testId="switch-carry-stays" />
        </section>
      )}
      <Explain guide={GUIDE.carried} testId="switch-carry-explain" tips={items.map((item) => `${item.label}: ${item.hint}`)} />
    </div>
  );
}
