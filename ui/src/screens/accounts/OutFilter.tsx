/**
 * 0.4.2 — Accounts' "Out" filter, where the top bar's "4 accounts out"
 * lands: its chip ("Out · 4", cleared with ×), and on each out Account's
 * card the line that says when it's back, in the Out mark's own words
 * ("Out · resets Thu 9:00 AM", `lib/room-mark`), and its best action:
 * New session on the Account with the most room (`accountAdvice`, the
 * one order by room the switch and New session use).
 */
import type { JSX } from "react";
import { RoomChip } from "../../components/RoomChip";
import { Button } from "../../components/primitives";
import type { RoomMark } from "../../lib/room-mark";

/** Where an out Account's work can go now: the Account with room. */
export interface OutTarget {
  /** Its name ("Personal"). */
  name: string;
  /** The folder New session starts on (its first signed-in one). */
  folderId: string;
  /** Its % left. */
  left: number;
}

/** Where an out Account's work can go: the Account with the most room
 * (`accountAdvice`'s best), by its first folder; `null` when none has. */
export function outTarget(best: { row: { name: string; group: { profiles: readonly { id: string }[] } }; left: number } | null): OutTarget | null {
  const folder = best?.row.group.profiles[0];
  return best && folder ? { name: best.row.name, folderId: folder.id, left: best.left } : null;
}

/** With a filter on and nothing it names: says so, in its words. */
export function FilteredNone({ word }: { word: string }): JSX.Element {
  return (
    <p className="text-meta text-ink-2" data-testid="accounts-filter-none">
      {word === "Out" ? "No account is out of room now." : "No account is low on room now."}
    </p>
  );
}

/** The lit filter chip: "Out · 4" ("Low · 2"), and × to show every
 * Account again. */
export function OutFilterChip({ word, count, onClear }: { word: string; count: number; onClear: () => void }): JSX.Element {
  return (
    <span className="inline-flex h-8 shrink-0 items-center gap-1 rounded-full border border-accent/45 bg-accent-soft pl-3 pr-1 text-meta font-medium text-ink" data-testid="accounts-filter" data-filter={word}>
      <span>
        {word} · <span className="tabular-nums">{count}</span>
      </span>
      <button
        type="button"
        onClick={onClear}
        aria-label={`Clear the ${word} filter: show every account`}
        title="Show every account"
        data-testid="accounts-filter-clear"
        className="flex size-6 items-center justify-center rounded-full text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60"
      >
        <span aria-hidden="true">×</span>
      </button>
    </span>
  );
}

/** An out Account's line, under its card's header: when it's back, and
 * where its work can go now. */
export function OutLine({ mark, target, self }: { mark: RoomMark | null; target: OutTarget | null; self: string }): JSX.Element {
  const elsewhere = target && target.name !== self ? target : null;
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-hairline bg-danger-soft/40 px-5 py-2.5" data-testid="account-out">
      {mark ? <RoomChip mark={mark} size="md" testId="account-out-mark" /> : <span className="text-meta font-semibold text-danger">Out</span>}
      <span className="min-w-0 flex-1 text-meta text-ink-2" data-testid="account-out-best">
        {elsewhere ? `${elsewhere.name} has ${elsewhere.left}% left.` : "No other account has room now."}
      </span>
      {elsewhere && (
        <Button
          size="sm"
          variant="secondary"
          onClick={() => window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: elsewhere.folderId } }))}
          data-testid="account-out-action"
        >
          New session on {elsewhere.name}
        </Button>
      )}
    </div>
  );
}
