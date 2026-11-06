/**
 * 0.3.5 — what goes along, in one coloured line on the last step
 * ("Everything goes along · 8 items"), that opens to the whole list (the
 * maintainer, 2026-11-02: "keep the transfer simple but give options to
 * read things in details"). Closed at first; its tint says whether
 * anything is left behind (warning) or all of it goes (success), and the
 * words say it too.
 */
import type { JSX } from "react";
import type { CarryItemData } from "../../lib/bridge";
import type { SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { IconAlert } from "../icons";
import { IconCheck } from "../icons-controls";
import { FoldChevron } from "../this-mac-memory";
import { CarryList, withSetup } from "./CarryList";

/** "Everything goes along · 8 items", or what doesn't: "7 go along · 1
 * stays behind", and the turns a cut leaves out ("the first 800 turns
 * stay behind", D-76). `warn` when something stays behind or isn't there
 * yet. */
export function carryWords(
  items: CarryItemData[],
  setup: SetupCheckInfo | null | undefined,
  target: string,
  tool: string,
  picked?: ReadonlySet<string>,
  turnsLeftOut = 0,
): { words: string; warn: boolean } {
  const { items: kept, missing } = withSetup(items, setup, target, tool, picked);
  const going = kept.filter((item) => item.how !== "stays").length;
  const stays = kept.length - going;
  const turns = turnsLeftOut > 0 ? `the first ${turnsLeftOut} ${turnsLeftOut === 1 ? "turn stays" : "turns stay"} behind` : null;
  if (stays === 0 && missing.length === 0 && !turns) return { words: `Everything goes along · ${going} ${going === 1 ? "item" : "items"}`, warn: false };
  const parts = [`${going} ${going === 1 ? "goes" : "go"} along`];
  if (missing.length > 0) parts.push(`${missing.length} to set up`);
  if (stays > 0) parts.push(`${stays} ${stays === 1 ? "stays" : "stay"} behind`);
  if (turns) parts.push(turns);
  return { words: parts.join(" · "), warn: true };
}

export function CarrySummary({
  items,
  target,
  tool,
  setup,
  picked,
  turnsLeftOut = 0,
}: {
  items: CarryItemData[];
  target: string;
  tool: string;
  setup?: SetupCheckInfo | null;
  picked?: ReadonlySet<string>;
  /** Turns the conversation's cut leaves out (D-76). */
  turnsLeftOut?: number;
}): JSX.Element | null {
  if (items.length === 0) return null;
  const { words, warn } = carryWords(items, setup, target, tool, picked, turnsLeftOut);
  const Mark = warn ? IconAlert : IconCheck;
  return (
    // Its own group name: the folds inside it ("What does this mean?")
    // turn their chevrons by their own state, not this one's.
    <details className="group/more" data-testid="switch-carry-summary">
      <summary
        className={
          "inline-flex min-h-7 cursor-pointer list-none items-center gap-1.5 rounded-full border px-2.5 text-meta font-medium outline-none focus-visible:ring-2 focus-visible:ring-accent/60 [&::-webkit-details-marker]:hidden " +
          (warn ? "border-warning/30 bg-warning-soft text-warning" : "border-success/30 bg-success-soft text-success")
        }
      >
        <Mark aria-hidden="true" width={14} height={14} className="shrink-0" />
        <span data-testid="switch-carry-words">{words}</span>
        <FoldChevron group="group/more" />
      </summary>
      <div className="mt-3">
        <CarryList items={items} target={target} tool={tool} setup={setup} picked={picked} />
      </div>
    </details>
  );
}
