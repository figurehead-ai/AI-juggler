/**
 * What goes along with a switch, in the backend's plain words, each with
 * its line on hover, on focus and under "What does this mean?" (the
 * maintainer's walk, 2026-10-27: "rebuilt by target is too vague, what
 * target"). 0.5.0: grouped under how each fares, said once per group.
 */
import { useId, type JSX } from "react";
import type { CarryItemData } from "../../lib/ipc/switch-preflight";
import type { SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { GUIDE } from "../../lib/switch-words";
import { Explain } from "./Explain";
import { IN_HANDOVER, NOTES_KIND } from "./NotesRow";
import { CARRY_TONES, CarryIcon, type CarryTone } from "./carry-marks";

/** One group's rows: the kind's icon in a chip (its tint the group's),
 * the thing, and its line. The group's heading says how it fares, once
 * (LST-3), never a tag on every row. Each row is focusable, its line read
 * with it. */
function Items({ items, testId, tone }: { items: CarryItemData[]; testId: string; tone: CarryTone }): JSX.Element {
  const id = useId();
  const look = CARRY_TONES[tone];
  return (
    <ul className="flex flex-col" data-testid={testId}>
      {items.map((item, index) => (
        <li
          key={item.label}
          tabIndex={0}
          aria-describedby={item.hint ? `${id}-${index}` : undefined}
          className="flex min-w-0 items-start gap-3 rounded-[10px] px-3 py-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
        >
          <span aria-hidden="true" className={"flex size-[30px] shrink-0 items-center justify-center rounded-lg " + look.chip}>
            <CarryIcon kind={item.kind} tone={tone} className={look.icon} />
          </span>
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="text-body font-medium text-ink">{item.label}</span>
            {item.hint && (
              <span id={`${id}-${index}`} className="text-meta text-ink-2">
                {item.hint}
              </span>
            )}
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
      return [{ ...item, hint: `${target} lacks ${lacks.length} of this app's, none of them used in this conversation.` }];
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

/** What the list draws in each group: the one count its summary says too
 * (the 0.5.0 walk, F4: "7 go along" over "Goes along · 6", the recreated
 * one counted as going). */
export function carryGroups(
  given: CarryItemData[],
  setup: SetupCheckInfo | null | undefined,
  target: string,
  tool?: string,
  picked?: ReadonlySet<string>,
): { moves: CarryItemData[]; recreated: CarryItemData[]; missing: CarryItemData[]; stays: CarryItemData[] } {
  const { items, missing } = withSetup(given, setup, target, tool, picked);
  return {
    moves: items.filter((item) => item.how === "moves"),
    recreated: items.filter((item) => item.how === "recreated"),
    missing,
    stays: items.filter((item) => item.how === "stays"),
  };
}

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
  const { moves, recreated, missing, stays } = carryGroups(given, setup, target, tool, picked);
  // One card, a short heading over each group: how it fares and how many.
  const group = (tone: CarryTone, title: string, list: CarryItemData[], testId: string, listId: string) =>
    list.length > 0 && (
      <section aria-label={title} className="flex flex-col gap-0.5" data-testid={testId}>
        <h4 className="px-3 pt-2 text-meta font-semibold text-ink">
          {title} <span className="font-normal text-muted">· {list.length}</span>
        </h4>
        <Items items={list} testId={listId} tone={tone} />
      </section>
    );
  return (
    <div className="flex flex-col gap-3" data-testid="switch-carry">
      <div className="flex flex-col gap-1 rounded-card bg-surface p-1.5 inset-ring-1 inset-ring-hairline/75">
        {group("moves", "Goes along", moves, "switch-carry-carried", "switch-carry-moves")}
        {group("recreated", `Recreated on ${target}`, recreated, "switch-carry-rebuilt", "switch-carry-recreated")}
        {group("missing", `Not on ${target} yet`, missing, "switch-carry-missing", "switch-carry-not-there")}
        {group("stays", "Stays behind", stays, "switch-carry-dropped", "switch-carry-stays")}
      </div>
      <Explain
        guide={GUIDE.carried}
        testId="switch-carry-explain"
        tips={[
          ...(recreated.length > 0 ? [`Recreated: ${tool} on ${target} makes it again from its latest version.`] : []),
          "Not there yet: pick it on the last step, and it's set up on the other account before the switch.",
          "Stays behind: the new session starts without it; the original keeps it.",
        ]}
      />
    </div>
  );
}
