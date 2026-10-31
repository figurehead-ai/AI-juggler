/**
 * 0.1.5 — the one page frame every screen sits in (the readability
 * pass): centred, at most 1440 px (`max-w-content`), one title block
 * (a 22 px title, a line of what the page is, its actions on the right),
 * then the page's sections 20 px apart. `PageGrid` lays cards out sized
 * to what they hold, as many to a row as fit, so a wide window fills
 * rather than leaving a strip with an empty side.
 *
 * `width="reading"` keeps a page of settings or sentences to a readable
 * measure (Settings, Privacy), still centred and titled the same way.
 */
import type { JSX, ReactNode } from "react";

export function Page({
  title,
  subtitle,
  actions,
  badge,
  width = "wide",
  children,
  testId,
  className = "",
  headingLevel = 2,
}: {
  title?: ReactNode;
  /** One line of what the page is: never a paragraph. */
  subtitle?: ReactNode;
  /** Its buttons, on the title's right. */
  actions?: ReactNode;
  /** A chip beside the title ("Beta"). */
  badge?: ReactNode;
  width?: "wide" | "reading";
  children: ReactNode;
  testId?: string;
  className?: string;
  headingLevel?: 1 | 2;
}): JSX.Element {
  const Heading = headingLevel === 1 ? "h1" : "h2";
  return (
    <div className={"mx-auto flex w-full flex-col gap-5 pb-10 " + (width === "wide" ? "max-w-content " : "max-w-[1040px] ") + className} data-testid={testId}>
      {title !== undefined && (
        <header className="flex flex-wrap items-end gap-x-6 gap-y-3" data-testid="page-title-block">
          <div className="flex min-w-[16rem] flex-1 flex-col gap-1">
            <div className="flex flex-wrap items-center gap-2">
              <Heading className="text-title text-ink">{title}</Heading>
              {badge}
            </div>
            {subtitle && <p className="max-w-[72ch] text-body text-ink-2">{subtitle}</p>}
          </div>
          {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
        </header>
      )}
      {children}
    </div>
  );
}

/** Cards sized to their content: as many to a row as fit at `min` px
 * each (one at a phone's width), never stretched past their row. */
export function PageGrid({ min = 340, children, className = "", testId }: { min?: number; children: ReactNode; className?: string; testId?: string }): JSX.Element {
  return (
    <div className={"grid items-start gap-grid " + className} style={{ gridTemplateColumns: `repeat(auto-fit, minmax(min(100%, ${min}px), 1fr))` }} data-testid={testId}>
      {children}
    </div>
  );
}

/** A page section: a 15 px title in ink, an optional line of why, and its
 * content. */
export function PageSection({ id, title, sub, actions, children, className = "", testId }: { id: string; title: string; sub?: ReactNode; actions?: ReactNode; children: ReactNode; className?: string; testId?: string }): JSX.Element {
  return (
    <section aria-labelledby={id} className={"flex min-w-0 flex-col gap-3 " + className} data-testid={testId}>
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <div className="flex min-w-0 flex-wrap items-baseline gap-x-2">
          <h3 id={id} className="text-section font-semibold text-ink">
            {title}
          </h3>
          {sub && <span className="text-meta text-muted">{sub}</span>}
        </div>
        {actions}
      </div>
      {children}
    </section>
  );
}
