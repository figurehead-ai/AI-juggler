/**
 * What goes along with a switch, in the backend's plain words, each with
 * its line on hover, on focus and under "What does this mean?" (the
 * maintainer's walk, 2026-10-27: "rebuilt by target is too vague, what
 * target"). Only what stays behind is a warning.
 */
import { useId, type JSX } from "react";
import type { CarryItemData } from "../../lib/bridge";
import type { SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { GUIDE } from "../../lib/switch-words";
import { Explain } from "./Explain";
import { IN_HANDOVER, NOTES_KIND } from "./NotesRow";

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

/** The set-up check's offer kinds behind a carry item. */
const SETUP_KINDS: Record<string, string[]> = { mcp: ["mcp-server"], skills: ["skill", "command"] };

/** A carry item the set-up check speaks for (the 0.2 walk, F12: "connect
 * again on the new account" beside "9 aren't on the other account"): its
 * line says what the check found, and one this conversation used that the
 * other account lacks is "Not on {target} yet", never "Goes along". */
export function withSetup(
  items: CarryItemData[],
  setup: SetupCheckInfo | null | undefined,
  target: string,
  tool = "its tool",
  picked?: ReadonlySet<string>,
): { items: CarryItemData[]; missing: CarryItemData[] } {
  if (!setup) return { items, missing: [] };
  const missing: CarryItemData[] = [];
  const kept = items.flatMap((item) => {
    const kinds = SETUP_KINDS[item.kind ?? ""];
    if (!kinds || item.how !== "moves") return [item];
    const lacks = setup.offers.filter((offer) => kinds.includes(offer.kind));
    const used = lacks.filter((offer) => offer.used !== false);
    if (lacks.length === 0) return [{ ...item, hint: `${target} has the same ones set up.` }];
    if (used.length === 0) {
      return [{ ...item, hint: `${target} lacks ${lacks.length} of this account's, none of them used in this conversation.` }];
    }
    missing.push({ ...item, hint: `${used.length} this conversation used ${used.length === 1 ? "isn't" : "aren't"} set up on ${target} yet: pick ${used.length === 1 ? "it" : "them"} on the last step to set ${used.length === 1 ? "it" : "them"} up first.` });
    return [];
  });
  // D-51: this project's notes, when the check found any to speak for:
  // where they go follows the tick, so this list and the last step agree.
  const notes = setup.offers.find((offer) => offer.kind === NOTES_KIND);
  const count = notes?.count ?? 0;
  const of = (n: number) => (n === 1 ? `1 of ${setup.from}'s notes for this project` : `${n} of ${setup.from}'s notes for this project`);
  const stay = setup.notes_stay ?? 0;
  const staying = (n: number) =>
    `${setup.from}'s ${n === 1 ? "note" : `${n} notes`} for this project stay${n === 1 ? "s" : ""} with ${setup.from}: ${tool} doesn't keep notes like these.`;
  if (notes && picked?.has(notes.id)) {
    const left = stay - count;
    const rest = notes.verdict === IN_HANDOVER && left > 0 ? ` ${left === 1 ? "The other one stays" : `The other ${left} stay`} with ${setup.from}.` : "";
    kept.push({
      label: NOTES_LABEL,
      how: "moves",
      kind: NOTES_KIND,
      hint: notes.verdict === IN_HANDOVER ? `${of(count)} go in the handover ${target} reads first.${rest}` : `${of(count)} are copied to ${target} when you switch.`,
    });
  } else if (notes && notes.verdict === IN_HANDOVER) {
    kept.push({ label: NOTES_LABEL, how: "stays", kind: NOTES_KIND, hint: `${staying(stay)} Tick them under Before you switch to include them in the handover.` });
  } else if (notes) {
    missing.push({
      label: NOTES_LABEL,
      how: "moves",
      kind: NOTES_KIND,
      hint: `${count === 1 ? `${of(1)} isn't` : `${of(count)} aren't`} on ${target} yet: tick them under Before you switch to copy them.`,
    });
  } else if (stay) {
    kept.push({ label: NOTES_LABEL, how: "stays", kind: NOTES_KIND, hint: staying(stay) });
  }
  return { items: kept, missing };
}

/** This project's notes, in the carry list (D-51). */
const NOTES_LABEL = "This project's notes";

export function CarryList({
  items: given,
  target,
  tool,
  setup,
  picked,
}: {
  items: CarryItemData[];
  target: string;
  tool: string;
  setup?: SetupCheckInfo | null;
  /** What the user ticked on the last step (this project's notes follow it). */
  picked?: ReadonlySet<string>;
}): JSX.Element {
  const { items, missing } = withSetup(given, setup, target, tool, picked);
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
      {missing.length > 0 && (
        <section className="flex flex-col gap-1.5" data-testid="switch-carry-missing">
          <h4 className="text-body font-semibold text-ink">Not on {target} yet: set up on the last step</h4>
          <Items items={missing} testId="switch-carry-not-there" />
        </section>
      )}
      {stays.length > 0 && (
        <section className="flex flex-col gap-1.5 rounded-lg border border-warning/40 px-3 py-2" data-testid="switch-carry-dropped">
          <h4 className="text-body font-semibold text-warning">Stays behind: the new session starts without it</h4>
          <Items items={stays} testId="switch-carry-stays" />
        </section>
      )}
      <Explain guide={GUIDE.carried} testId="switch-carry-explain" tips={[...items, ...missing].map((item) => `${item.label}: ${item.hint}`)} />
    </div>
  );
}
