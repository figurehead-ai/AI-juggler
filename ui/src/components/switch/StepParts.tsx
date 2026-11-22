/**
 * The parts every switch and move screen shares while it runs and once it's
 * done (DLG-10): the named steps, done, now and next, and the success head.
 * This Mac's switch, a switch to a paired Mac and Move to account all say
 * them the same way.
 */
import type { JSX, ReactNode } from "react";
import { IconCheck } from "../icons-controls";

/** The steps as a list: those before `at` done (a tick), `at` now (a
 * spinner), the rest next. `testId` marks each row. */
export function StepList({ steps, at, testId }: { steps: readonly string[]; at: number; testId?: string }): JSX.Element {
  return (
    <ol className="flex flex-col gap-1.5 text-body">
      {steps.map((step, index) => (
        <li key={step} className={"flex items-center gap-2.5 " + (index < at ? "text-ink-2" : index === at ? "font-semibold text-ink" : "text-muted")} data-testid={testId} data-state={index < at ? "done" : index === at ? "now" : "next"}>
          <span aria-hidden="true" className="flex size-5 shrink-0 items-center justify-center">
            {index < at ? (
              <IconCheck width={14} height={14} strokeWidth={2.5} className="text-success" />
            ) : index === at ? (
              <span className="inline-block size-3.5 rounded-full border-2 border-accent border-t-transparent motion-safe:animate-spin" />
            ) : (
              <span className="size-2 rounded-full inset-ring-[1.5px] inset-ring-faint" />
            )}
          </span>
          {step}
        </li>
      ))}
    </ol>
  );
}

/** A done screen's head: the success tick, the title and one line under it. */
export function DoneHead({ title, help }: { title: ReactNode; help: ReactNode }): JSX.Element {
  return (
    <div className="flex items-center gap-3">
      <span aria-hidden="true" className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-success-soft text-success">
        <IconCheck width={20} height={20} strokeWidth={2.5} />
      </span>
      <div className="flex min-w-0 flex-col gap-0.5">
        <h3 className="type-section">{title}</h3>
        <p className="type-help">{help}</p>
      </div>
    </div>
  );
}
