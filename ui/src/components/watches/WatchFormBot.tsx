/**
 * AJ-E3830 (0.5.2) — the Watches form as Bot pre-filled it: a calm note
 * at the top saying what Bot filled in and what is left for the user (in
 * the fields' own names, each leaving the list once set), the words it
 * couldn't read, the nearest condition tagged beside the cards (never
 * chosen for the user), and the quiet-hours line asking to check it. Only
 * the user's Watch adds the watch.
 */
import type { JSX } from "react";
import { IconBot } from "../bot/bot-icons";
import { botLeft, leftNow, type FormState } from "./watch-form-state";
import { BOT_FILL, FORM } from "./watches-words";

/** The lines beside Watch what and For what: Bot's "couldn't tell"
 * while it left that field empty, else the form's own. */
export function sectionHelps(state: FormState): { what: string; forWhat: string | undefined } {
  const many = state.targets.length;
  const what = botLeft(state, "target") && many === 0 ? BOT_FILL.targetLeft : FORM.whatHelp;
  const forWhat = many === 0 ? undefined : botLeft(state, "condition") && state.codes.length === 0 ? BOT_FILL.conditionLeft : many === 2 ? FORM.forWhatBoth : FORM.forWhatHelp(many);
  return { what, forWhat };
}

/** What Watch does, beside it: "Pick how long first." while Bot left it. */
export function consequenceOf(mode: "new" | "edit", asked: boolean, count: number, limit: number): string {
  if (asked) return FORM.pickLifeFirst;
  return mode === "new" ? FORM.startsNow(count + 1, limit) : FORM.changesNow;
}

/** The fields left, in a sentence: "How long and quiet hours". */
function leftWords(left: readonly string[]): string {
  const names = left.map((f, i) => {
    const name = BOT_FILL.field[f] ?? f;
    return i === 0 ? name : name.toLowerCase();
  });
  if (names.length <= 1) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} and ${names.at(-1)}`;
}

/** The note at the top of the form, as the form stands now (0.5.2 G68). */
export function BotFillNote({ state }: { state: FormState }): JSX.Element | null {
  const bot = state.bot;
  if (!bot) return null;
  const left = leftNow(state);
  const all = left.length === 0;
  const none = left.includes("target") && left.includes("condition") && left.length === 4;
  const line = all ? BOT_FILL.all : none ? BOT_FILL.none : bot.why === "several" && left.length === 1 && left[0] === "target" ? BOT_FILL.several : BOT_FILL.some(leftWords(left));
  const said = bot.asked ? (bot.why === "cant_watch" ? BOT_FILL.cant(bot.asked) : BOT_FILL.asked(bot.asked)) : null;
  return (
    <div className="flex items-start gap-3 rounded-card bg-[color-mix(in_srgb,var(--color-accent)_6%,var(--color-surface))] px-3.5 py-3 inset-ring-1 inset-ring-hairline/75" role="note" data-testid="watch-bot-fill" data-left={left.join(" ")}>
      <span aria-hidden="true" className="flex size-[30px] shrink-0 items-center justify-center rounded-lg bg-accent-soft text-accent-hi">
        <IconBot width={16} height={16} />
      </span>
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="text-body font-medium text-ink">{BOT_FILL.title}</span>
        <span className="text-meta text-ink-2" data-testid="watch-bot-fill-line">
          {line}
          {said ? ` ${said}` : ""}
        </span>
      </span>
    </div>
  );
}

/** The tag on the nearest condition's card. */
export function NearestTag(): JSX.Element {
  return (
    <span className="mt-2 inline-flex w-fit items-center rounded-full bg-accent-soft px-2 text-label font-medium text-accent-hi" title={BOT_FILL.nearestWhy} data-testid="watch-condition-nearest">
      {BOT_FILL.nearest}
    </span>
  );
}

/** Under the quiet-hours switch: Bot couldn't tell, so it is off; check it. */
export function QuietLeft(): JSX.Element {
  return (
    <p className="px-3 pb-2 text-meta text-ink-2" data-testid="watch-quiet-left">
      {BOT_FILL.quietLeft}
    </p>
  );
}
