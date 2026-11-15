/**
 * 0.4.6 (AJ-E3011 §1): the canvas's setup checklist, a side card in the
 * inspector — never over the canvas, so no member card hides it, and it
 * never vanishes when members are added. It keeps the setup's order, a
 * tick on each step done, and each step opens the same control as the
 * setup (the scope sheet, the palette, or the pool's own section in the
 * inspector). It goes once every step is done. Where something else is
 * selected it shrinks to one line that opens it again.
 */
import type { JSX } from "react";

import { IconCheck } from "../../../components/icons-controls";
import type { SetupRow } from "../../../lib/pool-setup";
import { STEP_WORDS, type SETUP_STEPS } from "../../../lib/pool-words";
import { PoolToolsCard } from "./PoolToolsCard";

type Step = (typeof SETUP_STEPS)[number];

export function PoolChecklist({ rows, onOpen, compact = false }: { rows: readonly SetupRow[]; onOpen: (step: Step) => void; compact?: boolean }): JSX.Element | null {
  const done = rows.filter((row) => row.done).length;
  if (done === rows.length) return null;
  const next = rows.find((row) => !row.done)!;
  if (compact) {
    return (
      <button
        type="button"
        onClick={() => onOpen(next.step)}
        className="mx-4 mb-2 flex items-center gap-2 rounded-lg bg-surface-2 px-3 py-1.5 text-left text-meta text-ink-2 outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60"
        data-testid="pool-checklist-line"
      >
        <span className="section-label">Setup</span>
        <span className="min-w-0 flex-1 truncate">
          {done} of {rows.length} done · next: {STEP_WORDS[next.step].title}
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
          const isNext = row.step === next.step;
          return (
            <li key={row.step}>
              <button
                type="button"
                onClick={() => onOpen(row.step)}
                aria-current={isNext ? "step" : undefined}
                className={"grid w-full grid-cols-[20px_1fr] items-start gap-x-2.5 rounded-lg px-2 py-1.5 text-left outline-none transition-colors hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60 " + (isNext ? "bg-surface-3" : "")}
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
                    <span className="sr-only">{row.done ? ", done" : ", to do"}</span>
                  </span>
                  {!row.done && <span className="text-label text-muted">{words.why}</span>}
                </span>
              </button>
            </li>
          );
        })}
      </ol>
      <PoolToolsCard testId="pool-checklist-tools" />
    </section>
  );
}
