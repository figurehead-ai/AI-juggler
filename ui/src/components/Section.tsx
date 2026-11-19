/**
 * 0.4.6 (D-126) — one Section for every screen: its heading on the type
 * scale's middle step (15 px semibold ink, `.type-section`), one line of
 * help beside it (13 px muted), an action at the end when it has one, and
 * its content below. Promoted from the pool inspector's own (AJ-E3011
 * round 3), which keeps its look through `divided`: a hairline between
 * sections, padded inside the inspector.
 *
 * `FoldSection`: a section's details behind a fold, closed at first; its
 * title and one plain line on what it controls stay in view.
 */
import type { JSX, ReactNode } from "react";

import { Disclosure } from "./controls";

type Heading = "h2" | "h3" | "h4";

export function Section({
  title,
  help,
  action,
  children,
  testId,
  divided = false,
  as: Tag = "h3",
  className = "",
}: {
  title: ReactNode;
  /** One line on what the section is for, beside its heading. */
  help?: ReactNode;
  /** One action at the heading's end (a text button, a Change). */
  action?: ReactNode;
  children?: ReactNode;
  testId?: string;
  /** The inspector's look: a hairline above (not on the first), padded. */
  divided?: boolean;
  /** The heading's level in its page's outline. */
  as?: Heading;
  className?: string;
}): JSX.Element {
  const box = divided ? "gap-2.5 border-t border-hairline/70 px-4 py-3.5 first:border-t-0" : "gap-3.5";
  return (
    <section className={`flex min-w-0 flex-col ${box} ${className}`} data-testid={testId}>
      <div className="flex min-w-0 flex-wrap items-baseline gap-x-2.5 gap-y-0.5">
        <Tag className="type-section">{title}</Tag>
        {help && <p className="type-help min-w-0">{help}</p>}
        {action && <span className="ml-auto shrink-0 self-center">{action}</span>}
      </div>
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
