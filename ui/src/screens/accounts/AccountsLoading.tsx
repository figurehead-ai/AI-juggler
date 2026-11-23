/**
 * AJ-E1503 Req.4, AJ-E1552 — the Accounts screen until its first read
 * answers: card-shaped placeholders while it loads ("Still loading…"
 * after 3 s), then, when the read fails or runs past its deadline, an
 * error in plain words with Retry. The screen's own empty state (no
 * account yet) is drawn by the screen once the read answers.
 */
import { useState, type JSX } from "react";
import { LoadingState } from "../../components/LoadingState";
import { RowPlaceholders } from "../../components/Placeholder";
import { useReadState } from "../../lib/loading";

export function AccountsLoading({ failed, onRetry }: { failed: string | undefined; onRetry: () => void }): JSX.Element {
  // `round` restarts the clock (a Retry).
  const [round, setRound] = useState(0);
  const read = useReadState(null, "your accounts", { error: failed, round });
  return (
    <div className="mx-auto flex w-full max-w-content flex-col gap-5">
      <LoadingState
        state={read.state}
        slow={read.slow}
        what="your accounts"
        skeleton={
          <div className="flex flex-col gap-5" data-testid="profiles-loading">
            <span className="block h-6 w-32 rounded bg-skeleton motion-safe:animate-pulse" aria-hidden="true" />
            <RowPlaceholders shape="card" rows={3} label="Loading accounts" />
          </div>
        }
        empty={null}
        onRetry={() => {
          setRound((n) => n + 1);
          onRetry();
        }}
      >
        {null}
      </LoadingState>
    </div>
  );
}
