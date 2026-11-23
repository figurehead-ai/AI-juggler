/**
 * AJ-E1552 — what a read shows in each of its states (lib/loading): its
 * skeleton only while it loads, with "Still loading…" once it is slow;
 * an error in plain words with Retry; its own empty state; else what it
 * read. The one place a skeleton is drawn for a read, so none is drawn
 * because a list is empty or a value is null.
 */
import type { JSX, ReactNode } from "react";
import type { LoadState } from "../lib/loading";
import { Button } from "./primitives";

export function LoadingState({
  state,
  slow = false,
  what,
  skeleton,
  empty,
  onRetry,
  failed,
  children,
}: {
  state: LoadState;
  /** Still loading after 3 s. */
  slow?: boolean;
  /** What is read, in words ("this account's apps"). */
  what: string;
  /** Drawn only while it loads. */
  skeleton: ReactNode;
  /** Drawn when the answer has nothing in it. */
  empty: ReactNode;
  /** Reads again (the error's Retry). */
  onRetry: () => void;
  /** The error's own sentence, when the read knows why it failed
   * (default "Couldn't load <what>."). */
  failed?: string;
  children: ReactNode;
}): JSX.Element {
  if (state === "loading") {
    return (
      <div className="flex flex-col gap-1" data-testid="loading-state" data-state="loading">
        {skeleton}
        {slow && (
          <output className="block px-2 text-label text-muted" data-testid="loading-slow">
            Still loading…
          </output>
        )}
      </div>
    );
  }
  if (state === "error") {
    return (
      <div className="flex flex-wrap items-center gap-2 px-2 py-1" role="alert" data-testid="loading-state" data-state="error">
        <p className="text-meta text-ink-2">{failed ?? `Couldn't load ${what}.`}</p>
        <Button size="sm" variant="secondary" onClick={onRetry} data-testid="loading-retry">
          Retry
        </Button>
      </div>
    );
  }
  if (state === "empty") return <>{empty}</>;
  return <>{children}</>;
}
