/**
 * AJ-E1503 Req.4 / AJ-E1552: Home while its first load answers: rows
 * shaped like Home's, then "Still loading…" after 3 s. The shell's first
 * read has its own deadline (shell/use-app-data); when it misses, the read
 * counts as done and the "service stopped answering" banner, with its
 * Retry, takes over, so this skeleton never stays. With no account, the
 * welcome is Home's empty state.
 */
import { RowPlaceholders } from "../components/Placeholder";
import { LoadingState } from "../components/LoadingState";
import { useLoadDeadline } from "../lib/loading";

export function HomeLoading() {
  const { slow } = useLoadDeadline(true, "Home", { timeoutMs: Number.POSITIVE_INFINITY });
  return (
    <LoadingState
      state="loading"
      slow={slow}
      what="Home"
      skeleton={
        <div className="mx-auto flex w-full max-w-content flex-col gap-6 pb-10" data-testid="home-loading">
          <span className="block h-7 w-64 rounded bg-skeleton motion-safe:animate-pulse" aria-hidden="true" />
          <RowPlaceholders shape="home" rows={3} label="Loading Home" />
        </div>
      }
      empty={null}
      // never asked for: the shell's banner has the Retry (see above)
      onRetry={() => undefined}
    >
      {null}
    </LoadingState>
  );
}
