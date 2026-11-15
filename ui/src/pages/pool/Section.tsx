/**
 * The pool inspector's sections: a titled one, and (AJ-E3011 round 3) a
 * fold, closed at first, whose title and one plain line on what it
 * controls stay in view. The pool's inspector, its setup sections and
 * Settings › Agent access use these.
 */
import type { JSX, ReactNode } from "react";

import { Disclosure } from "../../components/controls";

export function Section({ title, children, testId }: { title: string; children: ReactNode; testId?: string }): JSX.Element {
  return (
    <section className="flex flex-col gap-2.5 border-t border-hairline/70 px-4 py-3.5 first:border-t-0" data-testid={testId}>
      <h4 className="text-meta font-semibold text-ink">{title}</h4>
      {children}
    </section>
  );
}

/** A section's details behind a fold, closed at first (round 3, #2): its
 * title and one plain line on what it controls stay in view. */
export function FoldSection({ title, line, children, testId, open = false }: { title: string; line: string; children: ReactNode; testId?: string; open?: boolean }): JSX.Element {
  return (
    <section className="border-t border-hairline/70 px-4 py-3 first:border-t-0" data-testid={testId}>
      <Disclosure
        tone="heading"
        open={open}
        summary={
          <span className="flex flex-col">
            <span>{title}</span>
            <span className="text-meta font-normal text-ink-2">{line}</span>
          </span>
        }
        summaryClassName="items-start"
        testId={testId ? `${testId}-fold` : undefined}
      >
        <div className="mt-2.5 flex flex-col gap-2.5 pl-5">{children}</div>
      </Disclosure>
    </section>
  );
}
