/**
 * First run's Computer access step (AJ-E1525, #120's Reopen): its words
 * and the way on, around the shared access rows (AJ-E1527,
 * `OsAccessRows`, also on the welcome and Home).
 */
import type { JSX } from "react";
import { Button } from "./primitives";
import { OsAccessRows } from "./OsAccessRows";
import type { OsAccessInfo } from "../lib/bridge";

/** First run's Computer access step. */
export function AccessStep({
  entries,
  setEntries,
  onDone,
}: {
  entries: OsAccessInfo[];
  setEntries: (next: OsAccessInfo[]) => void;
  onDone: () => void;
}): JSX.Element {
  const allAllowed = entries.every((entry) => entry.status === "allowed");
  return (
    <div className="flex flex-col gap-6" data-testid="first-run-access">
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">
          {allAllowed
            ? "All set."
            : entries.length === 1
              ? "One optional permission makes it smoother."
              : `${entries.length} optional permissions make it smoother.`}
        </h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          Allow opens the system's own prompt, and this page updates as soon as you say yes. Skip any of them: AI Juggler asks again only when you first use what it's for.
        </p>
      </div>
      <OsAccessRows entries={entries} setEntries={setEntries} />
      <div className="flex items-center justify-between gap-4">
        {!allAllowed ? (
          <button type="button" className="text-sm text-muted hover:text-ink hover:underline" onClick={onDone}>
            Skip for now
          </button>
        ) : (
          <span />
        )}
        <Button variant="primary" onClick={onDone} data-testid="first-run-start">
          Start working
        </Button>
      </div>
    </div>
  );
}
