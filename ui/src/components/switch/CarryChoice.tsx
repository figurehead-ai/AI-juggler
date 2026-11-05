/**
 * D-76 (AJ-E211): for a very large conversation, under the switch's
 * advanced options (closed at first): carry everything (the default), or
 * leave out the turns before a date, or keep only the last N turns. Each
 * choice says what it carries and leaves out, and what the new session
 * won't know (serious choices explained). Shown only where the switch can
 * leave part out (the same tool, one that knows how).
 */
import { useEffect, useMemo, useState, type JSX } from "react";

import { Disclosure, Radio } from "../controls";
import { switchCutPreview, type SwitchCutArgs, type SwitchCutInfo } from "../../lib/ipc/switchCut";

type Kind = "all" | "before" | "last";

const day = (ms: number) => new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
const dateValue = (ms: number) => new Date(ms).toISOString().slice(0, 10);

export interface CarryChoiceState {
  /** What the switch sends (null: everything travels). */
  args: SwitchCutArgs | null;
  kind: Kind;
  setKind: (kind: Kind) => void;
  before: string;
  setBefore: (day: string) => void;
  last: number;
  setLast: (n: number) => void;
  info: SwitchCutInfo | null;
}

/** The choice, and what it would leave out (read again as it changes). */
export function useCarryChoice(sessionId: string, targetProfileId: string): CarryChoiceState {
  const [kind, setKind] = useState<Kind>("all");
  const [before, setBefore] = useState("");
  const [last, setLast] = useState(200);
  const [info, setInfo] = useState<SwitchCutInfo | null>(null);
  const args = useMemo<SwitchCutArgs | null>(() => {
    if (kind === "before" && before) return { before_ms: Date.parse(`${before}T00:00:00`), keep_last: null };
    if (kind === "last" && last > 0) return { before_ms: null, keep_last: last };
    return null;
  }, [kind, before, last]);
  useEffect(() => {
    let alive = true;
    const timer = window.setTimeout(() => {
      void switchCutPreview(sessionId, targetProfileId, args).then(
        (next) => alive && setInfo(next),
        () => alive && setInfo(null),
      );
    }, 150);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [sessionId, targetProfileId, args]);
  return { args: info?.supported ? args : null, kind, setKind, before, setBefore, last, setLast, info };
}

/** Only worth offering for a long conversation. */
const WORTH_IT_TURNS = 40;

export function CarryChoice({ state }: { state: CarryChoiceState }): JSX.Element | null {
  const { info, kind, setKind, before, setBefore, last, setLast } = state;
  if (!info?.supported || info.turns < WORTH_IT_TURNS) return null;
  const first = info.first_ms !== null ? day(info.first_ms) : null;
  return (
    <Disclosure tone="quiet" summary="Very long conversation? Leave out its start" testId="switch-carry-choice">
      <fieldset className="mt-2 flex flex-col gap-2">
        <legend className="sr-only">What the switch carries</legend>
        <label className="flex items-center gap-2 text-body text-ink">
          <Radio name="carry" checked={kind === "all"} onChange={() => setKind("all")} data-testid="carry-all" />
          Everything (as usual)
        </label>
        <label className="flex flex-wrap items-center gap-2 text-body text-ink">
          <Radio name="carry" checked={kind === "before"} onChange={() => setKind("before")} data-testid="carry-before" />
          Leave out the turns before
          <input
            type="date"
            className="rounded-lg border border-hairline bg-canvas px-2 py-0.5 text-meta text-ink"
            value={before}
            min={info.first_ms !== null ? dateValue(info.first_ms) : undefined}
            max={info.last_ms !== null ? dateValue(info.last_ms) : undefined}
            onChange={(event) => {
              setBefore(event.target.value);
              setKind("before");
            }}
            aria-label="The first day to carry"
            data-testid="carry-before-day"
          />
        </label>
        <label className="flex flex-wrap items-center gap-2 text-body text-ink">
          <Radio name="carry" checked={kind === "last"} onChange={() => setKind("last")} data-testid="carry-last" />
          Keep only the last
          <input
            type="number"
            min={1}
            max={info.turns}
            className="w-20 rounded-lg border border-hairline bg-canvas px-2 py-0.5 text-meta text-ink"
            value={last}
            onChange={(event) => {
              setLast(Math.max(1, Number(event.target.value) || 1));
              setKind("last");
            }}
            aria-label="How many turns to keep"
            data-testid="carry-last-n"
          />
          turns
        </label>
        <p className="text-meta text-ink-2" data-testid="carry-words" aria-live="polite">
          {info.words}
          {info.turns_left_out > 0 && info.from_ms !== null && ` It starts on ${day(info.from_ms)}.`}
          {info.turns_left_out === 0 && first && ` Its first turn is from ${first}.`}
        </p>
        {info.loses && (
          <p className="text-meta text-warning" data-testid="carry-loses">
            {info.loses}
          </p>
        )}
      </fieldset>
    </Disclosure>
  );
}
