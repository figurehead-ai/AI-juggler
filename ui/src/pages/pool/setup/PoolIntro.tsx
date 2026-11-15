/**
 * 0.4.6 (AJ-E3011 §1): a short intro at the top of the setup: what a pool
 * is, why make one, and what happens once it's made, in three lines with
 * a small diagram (a lead handing work to members, a reviewer checking
 * it). Full on the first pool; one line afterwards, a chevron opening it.
 */
import type { JSX } from "react";

import { Disclosure } from "../../../components/controls";
import { INTRO } from "../../../lib/pool-words";

/** A lead handing work to two members, a reviewer checking it: shapes in
 * the surface and text colours only (decorative; the lines say it). */
function Diagram(): JSX.Element {
  const node = (x: number, y: number, label: string, strong = false) => (
    <g>
      <rect x={x} y={y} width="70" height="24" rx="8" className={strong ? "fill-surface-3" : "fill-canvas"} />
      <text x={x + 35} y={y + 16} textAnchor="middle" fontSize="11" className={strong ? "fill-ink font-semibold" : "fill-ink-2"}>
        {label}
      </text>
    </g>
  );
  return (
    <svg aria-hidden="true" width="250" height="92" viewBox="0 0 250 92" className="shrink-0" data-testid="pool-intro-diagram">
      <defs>
        <marker id="pool-intro-arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto">
          <path d="M0 0 L8 4 L0 8 z" className="fill-muted" />
        </marker>
      </defs>
      <g className="stroke-muted" strokeWidth="1.25" fill="none" markerEnd="url(#pool-intro-arrow)">
        <path d="M74 44 C88 44 86 18 98 18" />
        <path d="M74 48 C88 48 86 74 98 74" />
        <path d="M170 18 C184 18 182 40 176 44" />
        <path d="M170 74 C184 74 182 52 176 48" />
      </g>
      {node(2, 34, "Lead", true)}
      {node(100, 6, "Member")}
      {node(100, 62, "Member")}
      {node(178, 34, "Reviewer")}
    </svg>
  );
}

export function PoolIntro({ first, testId = "pool-intro" }: { first: boolean; testId?: string }): JSX.Element {
  const words = (
    <div className="flex flex-wrap items-center gap-4 rounded-xl bg-surface-2 px-4 py-3">
      <Diagram />
      <ul className="flex min-w-0 flex-1 flex-col gap-1 text-meta text-ink-2" data-testid={`${testId}-lines`}>
        <li>
          <span className="font-medium text-ink">{INTRO.what}</span>
        </li>
        <li>{INTRO.why}</li>
        <li>{INTRO.after}</li>
      </ul>
    </div>
  );
  if (first) {
    return (
      <section aria-label="What a pool is" data-testid={testId} data-open="true">
        {words}
      </section>
    );
  }
  return (
    <Disclosure summary={`What a pool is: ${INTRO.what.replace(/^A pool is /, "").replace(/\.$/, "")}`} testId={testId} className="flex flex-col gap-2">
      <div className="mt-2">{words}</div>
    </Disclosure>
  );
}
