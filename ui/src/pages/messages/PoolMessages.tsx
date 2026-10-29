/**
 * E18 — a pool's Messages tab: its threads and the conversation, then the
 * documents its agents shared. Loaded only when the tab opens.
 */
import type { JSX } from "react";
import type { InstanceInfo } from "../../lib/bridge";
import { usePoolPlace } from "../../lib/pools";
import type { Member } from "./Compose";
import { MessagesView } from "./MessagesView";
import { SharedDocs } from "./SharedDocs";

export function PoolMessages({ poolId, members, now }: { poolId: string; members: Member[]; now: number }): JSX.Element {
  return (
    <div className="flex w-full min-w-0 max-w-[1440px] flex-col gap-6" data-testid="pool-messages">
      <MessagesView poolId={poolId} members={members} now={now} />
      <SharedDocs poolId={poolId} now={now} />
    </div>
  );
}

/** A session's Messages tab: its threads, in its pool, written to it. */
export function SessionMessages({ session, now }: { session: InstanceInfo; now: number }): JSX.Element | null {
  const poolId = usePoolPlace(session.id)?.pool_id;
  if (!poolId) return null;
  const name = session.label ?? session.title ?? session.account_label;
  return <MessagesView poolId={poolId} sessionId={session.id} members={[{ session_id: session.id, name }]} composeTo={session.id} now={now} />;
}
