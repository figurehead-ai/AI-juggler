/**
 * 0.4.6 (AJ-E3011 §6): who may message whom, per role pair, in plain row
 * and column labels ("Lead can message…", no "From ↓ · To →"). Allowed is
 * calm: muted words, no fill. Colour is only for an exception: Ask first
 * is L2 (the amber word and dot), Blocked is L3 (red with a ⊘), so one
 * exception stands out at once. A click moves a cell on: Allowed, Ask
 * first, Blocked. One line above says what it does; one below how to set
 * it for one pair (the canvas's line). The setup and the inspector share
 * it.
 */
import type { JSX } from "react";

import type { PoolRole } from "../../../lib/ipc/pools";
import type { TalkRule } from "../../../lib/ipc/pool-model";
import { TALK_ABOVE, TALK_BELOW, TALK_WORDS, roleCard, talkColumnLabel, talkRowLabel } from "../../../lib/pool-words";
import { CLAUDE_CHANNEL_NOTE } from "../../../lib/pools";
import { TALK_ROLES } from "../../../lib/pool-templates";

/** A cell's words and marks by rule (no fill: an exception's word, its
 * dot or glyph, carries it). */
function Rule({ rule }: { rule: TalkRule }): JSX.Element {
  const words = TALK_WORDS[rule];
  if (words.level === 0) return <span className="text-muted">{words.label}</span>;
  return (
    <span className={"inline-flex items-center gap-1.5 font-semibold " + (words.level === 3 ? "text-danger" : "text-warning")}>
      {words.level === 3 ? (
        <span aria-hidden="true" className="text-[11px] leading-none">
          ⊘
        </span>
      ) : (
        <span aria-hidden="true" className="level-mark" data-level="2" />
      )}
      {words.label}
    </span>
  );
}

export function TalkTable({
  ruleOf,
  onCycle,
  disabled = false,
  testId = "talk-table",
  compact = false,
}: {
  ruleOf: (from: PoolRole, to: PoolRole) => TalkRule;
  onCycle: (from: PoolRole, to: PoolRole) => void;
  disabled?: boolean;
  testId?: string;
  /** The inspector's narrow column: short column heads. */
  compact?: boolean;
}): JSX.Element {
  return (
    <div className="flex flex-col gap-2">
      <p className="text-meta text-ink-2" data-testid={`${testId}-above`}>
        {TALK_ABOVE}
      </p>
      <div className="overflow-x-auto">
        <table className="w-full border-separate border-spacing-y-0.5 text-meta" data-testid={testId}>
          <thead>
            <tr>
              <th scope="col" className="sr-only">
                Who sends
              </th>
              {TALK_ROLES.map((to) => (
                <th key={to} scope="col" className="section-label px-1.5 pb-1 text-left">
                  {compact ? roleCard(to).label : talkColumnLabel(to)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {TALK_ROLES.map((from) => (
              <tr key={from}>
                <th scope="row" className="whitespace-nowrap py-1 pr-3 text-left font-medium text-ink">
                  {talkRowLabel(from)}
                </th>
                {TALK_ROLES.map((to) => {
                  // One Lead: the Lead doesn't message itself.
                  if (from === to && from === "lead") {
                    return (
                      <td key={to} className="px-1.5 text-faint" aria-label="Not a pair: there is one Lead">
                        —
                      </td>
                    );
                  }
                  const rule = ruleOf(from, to);
                  return (
                    <td key={to} className="p-0">
                      <button
                        type="button"
                        disabled={disabled}
                        onClick={() => onCycle(from, to)}
                        aria-label={`${roleCard(from).label} to ${talkColumnLabel(to)}: ${TALK_WORDS[rule].label}. Press to change.`}
                        className="w-full rounded-md px-1.5 py-1 text-left outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-ink-2/70 disabled:opacity-60"
                        data-testid={`${testId}-cell`}
                        data-pair={`${from}>${to}`}
                        data-rule={rule}
                      >
                        <Rule rule={rule} />
                      </button>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="text-meta text-ink-2" data-testid={`${testId}-below`}>
        {TALK_BELOW} {CLAUDE_CHANNEL_NOTE}
      </p>
    </div>
  );
}
