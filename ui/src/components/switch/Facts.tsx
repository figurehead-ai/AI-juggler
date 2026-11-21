/**
 * 0.5.0 (D-142, DESIGN-SYSTEM §8, PAT-SER-5): the switch's summary panel,
 * what a choice does, said as a few fact rows on a tinted panel: each an
 * icon, a quiet label and the value, the loudest words in the row (COL-3).
 * The Move, Copy, wizard and done screens all say their facts with it.
 */
import type { JSX, ReactNode, SVGProps } from "react";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

/** The panel: accent-tinted, the rows apart by a hairline. */
export function FactPanel({ children, testId, label }: { children: ReactNode; testId?: string; label?: string }): JSX.Element {
  return (
    <dl aria-label={label} className="flex min-w-0 flex-col divide-y divide-hairline/70 rounded-card bg-accent-soft px-4 py-1 inset-ring-1 inset-ring-accent/15" data-testid={testId}>
      {children}
    </dl>
  );
}

/** One fact: icon, label, value. `tone` colours the icon only (the words
 * say it): `done` for what went as planned, `warn` for what needs a look. */
export function Fact({ icon: Glyph, label, children, testId, tone = "plain" }: { icon: Icon; label: string; children: ReactNode; testId?: string; tone?: "plain" | "done" | "warn" }): JSX.Element {
  const ink = tone === "done" ? "text-success" : tone === "warn" ? "text-warning" : "text-accent-hi";
  return (
    <div className="grid grid-cols-[16px_6.5rem_minmax(0,1fr)] items-start gap-x-3 py-2.5" data-testid={testId}>
      <Glyph aria-hidden="true" width={16} height={16} className={`mt-0.5 ${ink}`} />
      <dt className="text-meta text-muted">{label}</dt>
      <dd className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-body text-ink">{children}</dd>
    </div>
  );
}
