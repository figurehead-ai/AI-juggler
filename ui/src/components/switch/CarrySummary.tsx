/**
 * 0.3.5 — what goes along, in one coloured line on the last step
 * ("Everything goes along · 8 items"), that opens to the whole list (the
 * maintainer, 2026-11-02: "keep the transfer simple but give options to
 * read things in details"). Closed at first; its tint says whether
 * anything is left behind (warning) or all of it goes (success), and the
 * words say it too.
 */
import type { JSX, ReactNode } from "react";
import type { CarryItemData, PreflightInfoData } from "../../lib/ipc/switch-preflight";
import type { SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { IconAlert } from "../icons";
import { IconCheck } from "../icons-controls";
import { Disclosure } from "../controls";
import { CarryList, withSetup } from "./CarryList";
import { Section } from "../Section";

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

/** The words as a tinted line: an icon chip (✓ all goes; ! something
 * doesn't) and the count, the words saying it too. */
function Headline({ words, warn, testId }: { words: string; warn: boolean; testId?: string }): JSX.Element {
  const Mark = warn ? IconAlert : IconCheck;
  return (
    <span className="inline-flex min-w-0 items-center gap-2 font-medium text-ink">
      <span aria-hidden="true" className={"flex size-6 shrink-0 items-center justify-center rounded-md " + (warn ? "bg-warning-soft text-warning" : "bg-success-soft text-success")}>
        <Mark width={14} height={14} />
      </span>
      <span data-testid={testId}>{words}</span>
    </span>
  );
}

/** The confirm's "Carries" fact: the count, opening to the whole list
 * (closed at first). */
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
  return (
    // The app's disclosure, a button an accessibility press opens (the
    // 0.4.6 rc2 walk P9): its chevron, then the line.
    <Disclosure summary={<Headline words={words} warn={warn} testId="switch-carry-words" />} testId="switch-carry-summary">
      <div className="mt-3">
        <CarryList items={items} target={target} tool={tool} setup={setup} picked={picked} />
      </div>
    </Disclosure>
  );
}

/** The wizard's "What's carried" step: the count on the tinted panel, then
 * every thing as a tagged row, then the keys question when it applies. */
export function CarriedStep({ preflight, target, tool, setup, picked, keys }: { preflight: PreflightInfoData; target: string; tool: string; setup: SetupCheckInfo | null; picked: ReadonlySet<string>; keys: ReactNode }): JSX.Element {
  const items = preflight.carry_items ?? [];
  const { words, warn } = carryWords(items, setup, target, tool, picked);
  return (
    <div className="flex flex-col gap-6" data-testid="switch-step-carried">
      <Section title="What goes along">
        {items.length > 0 && (
          <div className="flex min-w-0 flex-col gap-0.5 rounded-card bg-accent-soft px-4 py-3 inset-ring-1 inset-ring-accent/15" data-testid="switch-carry-headline">
            <Headline words={words} warn={warn} />
            <span className="pl-8 text-meta text-ink-2">
              {warn ? `The new session on ${target} starts without what stays behind. What isn't there yet, you can set up on the last step.` : `The new session on ${target} picks up with all of it.`}
            </span>
          </div>
        )}
        <CarryList items={items} target={target} tool={tool} setup={setup} picked={picked} />
      </Section>
      {keys}
    </div>
  );
}
