/**
 * 0.1.5 — the Usage tab of a session AI Juggler started: the same tab as
 * any session's (`UsagePanel`: the charts once its tool has named its
 * conversation, its account's window beside them), never a page of its
 * own design.
 */
import { useMemo, type JSX } from "react";
import type { InstanceInfo } from "../../lib/bridge";
import { useConversations } from "../../lib/conversations";
import { accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import { fromInstance } from "./model";
import { UsagePanel } from "./UsagePanel";

export function OwnSessionUsage({ session, visible }: { session: InstanceInfo; visible: boolean }): JSX.Element {
  const { all } = useConversations();
  const accounts = useSubscriptions();
  const seen =
    all.find((c) => c.session_id === session.id) ??
    all.find((c) => c.account_id === session.account_id && c.id === session.conversation_id);
  const json = JSON.stringify(fromInstance(session, seen));
  const model = useMemo(() => JSON.parse(json) as ReturnType<typeof fromInstance>, [json]);
  const account = accounts ? accountOfProfile(accounts, model.profileId) : undefined;
  const tokens = session.tokens ? { input: session.tokens.input, output: session.tokens.output, cache_read: session.tokens.cache_read, cache_write: session.tokens.cache_write } : null;
  return <UsagePanel model={model} visible={visible} tokens={tokens} account={account} />;
}
