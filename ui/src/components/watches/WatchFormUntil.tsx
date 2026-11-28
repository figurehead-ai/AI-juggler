/**
 * AJ-E3816 — the form's Until section (split from `WatchForm.tsx`): the
 * lifetime with its icon, its dates for Until…, and the line that says
 * what it does. AJ-E3830 (0.5.2): when Bot couldn't read how long, no
 * segment is chosen and the line asks for one.
 */
import type { JSX } from "react";
import { DateField, Segmented, TimeField, type Segment } from "../controls";
import { IconCalendar } from "../icons-controls";
import { whenWords } from "../../lib/time-words";
import type { CatalogueEntry } from "../../lib/ipc/watch-view";
import { lifeAsked, lifeOf, lifeWaits, untilMs, type FormState, type LifeKind } from "./watch-form-state";
import { targetKey, untilEndsOffered, type TargetOption } from "./watches-view";
import { BOT_FILL, FORM, LIFE } from "./watches-words";
import { LIFETIME_ICONS } from "./watch-icons";

/** The line under Until: what the chosen lifetime does. */
function lifeLine(life: LifeKind, at: number | null, past: boolean, names: string[], now: number): string {
  if (life === "once") return LIFE.onceLine;
  if (life === "until_ends") return LIFE.untilEndsLine(names.length > 1 ? `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}` : (names[0] ?? "it"));
  if (life === "today") return LIFE.todayLine;
  if (life === "always") return LIFE.alwaysLine;
  if (past) return LIFE.past;
  return at === null ? LIFE.untilPick : LIFE.untilLine(whenWords(at, now));
}

/** Section 3: the lifetime, its dates for Until…, and what it does. */
export function UntilChoice({ state, set, offered, book, now }: { state: FormState; set: (change: Partial<FormState>) => void; offered: CatalogueEntry[]; book: Map<string, TargetOption>; now: number }): JSX.Element {
  const life = lifeOf(state);
  // AJ-E3830: Bot couldn't read how long: nothing is chosen for the user.
  const asked = lifeAsked(state);
  // 0.5.2 G60: Bot left what to watch (or what for) and how long is
  // unsaid: the suggestion waits for them (none shown chosen until then).
  const waits = lifeWaits(state);
  // AJ-E1558 (D-126): each lifetime with its icon, as the watch list shows it.
  const segments: Segment<LifeKind>[] = [
    { value: "once", label: LIFE.once, icon: LIFETIME_ICONS.once },
    ...(untilEndsOffered(state.targets) ? [{ value: "until_ends" as const, label: state.targets.length > 1 ? LIFE.untilTheyEnd : LIFE.untilEnds, icon: LIFETIME_ICONS.until_ends }] : []),
    { value: "today", label: LIFE.today, icon: LIFETIME_ICONS.until },
    { value: "until", label: LIFE.until, icon: IconCalendar },
    { value: "always", label: LIFE.always, icon: LIFETIME_ICONS.always },
  ];
  const at = untilMs(state.untilDay, state.untilTime);
  const past = life === "until" && at !== null && at <= now;
  const names = state.targets.flatMap((t) => {
    const name = book.get(targetKey(t))?.name;
    return name ? [`“${name}”`] : [];
  });
  const first = offered.find((e) => e.code === state.codes[0])?.label.toLowerCase();
  return (
    <>
      <div className="flex flex-wrap items-center gap-3">
        <Segmented label={FORM.until} value={asked || waits !== null ? null : life} onChange={(next) => set({ life: next })} segments={segments} testId="watch-lifetime-choice" className="w-fit" />
        {state.life === null && !asked && first && <span className="text-label text-muted">{LIFE.suggested(first)}</span>}
      </div>
      {life === "until" && (
        <div className="flex flex-wrap items-center gap-2">
          <DateField value={state.untilDay} onCommit={(day) => set({ untilDay: day })} ariaLabel={LIFE.day} testId="watch-until-day" />
          <TimeField value={state.untilTime} onCommit={(time) => set({ untilTime: time })} ariaLabel={LIFE.time} testId="watch-until-time" />
        </div>
      )}
      <p className={"text-meta " + (past ? "text-danger" : asked ? "text-ink-2" : "text-muted")} data-testid="watch-lifetime-line" data-past={past} data-asked={asked || undefined} data-waits={waits ?? undefined}>
        {asked ? BOT_FILL.lifeLeft : waits !== null ? BOT_FILL.lifeFollows(waits) : lifeLine(life, at, past, names, now)}
      </p>
    </>
  );
}
