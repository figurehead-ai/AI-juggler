/**
 * 0.4.6 (AJ-E3011 §1): the canvas's setup checklist, a side card in the
 * inspector — never over the canvas, so no member card hides it, and it
 * never vanishes when members are added. It keeps the setup's order, a
 * tick on each step done, and each step opens the same control as the
 * setup (the scope sheet, the palette, or the pool's own section in the
 * inspector). Round 4 (#2, #3): a step left at its default counts as
 * done ("default ✓"); with who's in set and the team tools on it says the
 * pool is ready, and what happens next, in one line (the shared
 * sentences'). Where something else is selected it shrinks to one line
 * that opens it again.
 */
import type { JSX } from "react";

import { IconCheck } from "../../../components/icons-controls";
import { SELECTED_FILL } from "../../../components/controls/selected";
import type { SetupRow } from "../../../lib/pool-setup";
import { STEP_WORDS, type SETUP_STEPS } from "../../../lib/pool-words";

type Step = (typeof SETUP_STEPS)[number];

export function PoolChecklist({
  rows,
  onOpen,
  compact = false,
  ready = false,
  next: nextWords = null,
  toolsLeft = null,
}: {
  rows: readonly SetupRow[];
  onOpen: (step: Step) => void;
  compact?: boolean;
  /** Who's in set and the team tools on (`setupReady`). */
  ready?: boolean;
  /** What happens next, once ready (`nextLine`). */
  next?: string | null;
  /** Every step done, the team tools not on: what's left (`toolsLeftLine`). */
  toolsLeft?: string | null;
}): JSX.Element | null {
  const done = rows.filter((row) => row.done).length;
  if (ready) return <Ready next={nextWords} compact={compact} />;
  // Every step done but the team tools off: the list stays, all ticked,
  // and says what's left (the question or its Turn on is at the top).
  const next = rows.find((row) => !row.done);
  if (compact) {
    return (
      <button
        type="button"
        onClick={() => onOpen(next?.step ?? "who")}
        className="mx-4 mb-2 flex items-center gap-2 rounded-lg bg-surface-2 px-3 py-1.5 text-left text-meta text-ink-2 outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60"
        data-testid="pool-checklist-line"
      >
        <span className="section-label">Setup</span>
        <span className="min-w-0 flex-1 truncate">
          {done} of {rows.length} done · {next ? `next: ${STEP_WORDS[next.step].title}` : "ready once the team tools are on"}
        </span>
      </button>
    );
  }
  return (
    <section aria-labelledby="pool-checklist-title" className="mx-4 mb-3 flex flex-col gap-2 rounded-xl bg-surface-2 p-3" data-testid="pool-checklist">
      <div className="flex items-baseline justify-between gap-2">
        <h4 id="pool-checklist-title" className="text-body font-semibold text-ink">
          Set up this pool
        </h4>
        <span className="text-meta text-muted" data-testid="pool-checklist-count">
          {done} of {rows.length} done
        </span>
      </div>
      <ol className="flex flex-col gap-0.5">
        {rows.map((row, at) => {
          const words = STEP_WORDS[row.step];
          const isNext = row.step === next?.step;
          return (
            <li key={row.step}>
              <button
                type="button"
                onClick={() => onOpen(row.step)}
                aria-current={isNext ? "step" : undefined}
                className={"grid w-full grid-cols-[20px_1fr] items-start gap-x-2.5 rounded-lg px-2 py-1.5 text-left outline-none transition-colors hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60 " + (isNext ? SELECTED_FILL : "")}
                data-testid="pool-checklist-step"
                data-step={row.step}
                data-done={row.done}
              >
                <span
                  aria-hidden="true"
                  className={"mt-0.5 flex size-5 items-center justify-center rounded-full text-label font-semibold " + (row.done ? "bg-surface-3 text-ink-2" : isNext ? "bg-accent text-white" : "shadow-[inset_0_0_0_1px_var(--color-hairline)] text-muted")}
                >
                  {row.done ? <IconCheck width={11} height={11} strokeWidth={2.5} /> : at + 1}
                </span>
                <span className="flex min-w-0 flex-col">
                  <span className={"text-meta " + (row.done ? "text-ink-2" : "font-medium text-ink")}>
                    {words.title}
                    {row.byDefault && (
                      <span className="ml-1.5 text-label text-muted" data-testid="pool-checklist-default">
                        default
                      </span>
                    )}
                    <span className="sr-only">{row.byDefault ? ", left at its default" : row.done ? ", done" : ", to do"}</span>
                  </span>
                  {!row.done && <span className="text-label text-muted">{words.why}</span>}
                </span>
              </button>
            </li>
          );
        })}
      </ol>
      {!next && toolsLeft && (
        <p className="px-2 text-meta text-ink-2" data-testid="pool-checklist-tools-left">
          {toolsLeft}
        </p>
      )}
    </section>
  );
}

/** Ready: the pool works as it is; what happens next, in one line. */
function Ready({ next, compact }: { next: string | null; compact: boolean }): JSX.Element {
  if (compact)
    return (
      <p className="mx-4 mb-2 flex items-center gap-2 rounded-lg bg-surface-2 px-3 py-1.5 text-meta text-ink-2" data-testid="pool-checklist-line">
        <span className="section-label">Setup</span>
        <span className="min-w-0 flex-1 truncate">Ready</span>
      </p>
    );
  return (
    <section aria-labelledby="pool-ready-title" className="mx-4 mb-3 flex flex-col gap-1 rounded-xl bg-surface-2 p-3" data-testid="pool-ready">
      <h4 id="pool-ready-title" className="flex items-center gap-1.5 text-body font-semibold text-ink">
        <IconCheck aria-hidden="true" width={14} height={14} strokeWidth={2.5} className="text-success" />
        This pool is ready
      </h4>
      {next && (
        <p className="text-meta text-ink-2" data-testid="pool-ready-next">
          <span className="font-medium text-ink">What happens next:</span> {next}.
        </p>
      )}
    </section>
  );
}
