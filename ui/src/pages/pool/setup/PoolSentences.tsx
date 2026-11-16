/**
 * AJ-E3011 round 3: a pool in sentences — New pool's Review and the pool's
 * own inspector on the diagram show this one list, from the same
 * functions (lib/pool-sentences), each row with its Change.
 */
import type { JSX, ReactNode } from "react";

import { everyoneLines, leadLine, ownLine, sealLine, talkSentence, whoSentence, type PoolShape } from "../../../lib/pool-sentences";
import type { TeamTools } from "../../../lib/team-tools";
import { TeamToolsFix } from "./TeamTools";

export type SentenceRow = "who" | "lead" | "everyone" | "talk" | "own" | "sealed" | "tools";

export const SENTENCE_TITLES: Record<SentenceRow, string> = {
  who: "Who's in",
  lead: "Who leads",
  everyone: "Everyone else",
  talk: "Who may message whom",
  own: "On their own",
  sealed: "Kept apart",
  tools: "Team tools",
};

export function PoolSentences({
  shape,
  where,
  rows,
  tools,
  first,
  changeOf,
  compact = false,
  testId,
}: {
  /** The inspector's narrow column: each title over its sentence. */
  compact?: boolean;
  shape: PoolShape;
  /** Its projects in words ("web", "every project"). */
  where: string;
  rows: readonly SentenceRow[];
  /** Where the team tools stand (null: still reading). */
  tools?: TeamTools | null;
  /** A row before them (Review's name and where). */
  first?: ReactNode;
  /** A row's Change, where it has one: back to its step, or its section. */
  changeOf?: (row: SentenceRow) => (() => void) | undefined;
  testId: string;
}): JSX.Element {
  const text = (row: SentenceRow): ReactNode => {
    switch (row) {
      case "who":
        return whoSentence(shape);
      case "lead":
        return leadLine(shape, where);
      case "everyone": {
        const [join, ...each] = everyoneLines(shape, where);
        return (
          <>
            {join}
            {each.map((line) => (
              <span key={line} className="block" data-testid={`${testId}-role`}>
                {line}
              </span>
            ))}
          </>
        );
      }
      case "talk":
        return talkSentence(shape);
      case "own":
        return ownLine(shape);
      case "sealed":
        return sealLine(shape);
      default:
        // Round 4: never "can't message" without its Turn on beside it.
        return tools ? <TeamToolsFix state={tools} testId={`${testId}-tools`} /> : "Reading…";
    }
  };
  return (
    <dl className="flex flex-col gap-1" data-testid={testId}>
      {first}
      {rows.map((row) => (
        <Sentence key={row} title={SENTENCE_TITLES[row]} onChange={changeOf?.(row)} testId={testId} row={row} compact={compact}>
          {text(row)}
        </Sentence>
      ))}
    </dl>
  );
}

/** One sentence: its title, its words, and its Change. */
export function Sentence({ title, children, onChange, testId, row, compact = false }: { title: string; children: ReactNode; onChange?: () => void; testId: string; row?: string; compact?: boolean }): JSX.Element {
  const change = onChange && (
    <button type="button" onClick={onChange} className="btn-ghost rounded-md px-1.5 py-0.5 text-meta outline-none focus-visible:ring-2 focus-visible:ring-accent/60" data-testid={`${testId}-change`}>
      Change<span className="sr-only"> {title.toLowerCase()}</span>
    </button>
  );
  if (compact)
    return (
      <div className="flex flex-col gap-0.5 py-1" data-row={row}>
        <dt className="flex items-center justify-between gap-2">
          <span className="section-label">{title}</span>
          {change}
        </dt>
        <dd className="text-meta text-ink">{children}</dd>
      </div>
    );
  return (
    <div className="grid grid-cols-1 items-start gap-x-4 gap-y-0.5 rounded-lg px-1 py-1.5 sm:grid-cols-[11rem_1fr_auto]" data-row={row}>
      <dt className="section-label pt-0.5">{title}</dt>
      <dd className="text-body text-ink">{children}</dd>
      {change && <dd>{change}</dd>}
    </div>
  );
}
