import type { JSX } from "react";
import { useLoadDeadline } from "../lib/loading";
import { LoadingState } from "./LoadingState";
import { RowPlaceholders } from "./Placeholder";

/** AJ-E1552: the rail's rows while the sessions are first read: Still
 * loading after 3 s; the shell's first read has the deadline, and its
 * "stopped answering" notice and Retry take over when it misses. */
export function RailLoading(): JSX.Element {
  const { slow } = useLoadDeadline(true, "the sessions", { timeoutMs: Number.POSITIVE_INFINITY });
  return (
    <LoadingState state="loading" slow={slow} what="the sessions" skeleton={<RowPlaceholders shape="rail" rows={4} label="Loading sessions" />} empty={null} onRetry={() => undefined}>
      {null}
    </LoadingState>
  );
}
