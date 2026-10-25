/**
 * AJ-E2108 — a session page's Usage tab, for any session. With the
 * tool's id for its conversation it is the charts (0.1.5,
 * `SessionUsage`: turn by turn, by kind, by model, its share of the
 * limits) and its account's window beside them; before the tool has
 * written one, its counts and model as `SessionUsageTab` reads them.
 */
import type { JSX } from "react";
import { accountDisplay, accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import { SessionUsage } from "../session-usage/SessionUsage";
import { useCached } from "../../lib/usage/cached";
import { usageSession } from "../../lib/ipc/usageHistory";
import { useRunways } from "./data";
import type { SessionModel } from "./model";
import { Runway } from "./Runway";
import { SessionUsageTab, useTranscriptModels, type TokenKinds } from "./UsageTab";

export function UsagePanel({ model, visible, tokens, account }: { model: SessionModel; visible: boolean; tokens: TokenKinds | null; account: SubscriptionInfo | undefined }): JSX.Element {
  const displays = useRunways();
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === model.profileId);
  // The same read SessionUsage makes (one key, one answer): a session the
  // usage history has no turns for shows its own counts instead of a page
  // of empty charts.
  const history = useCached(model.conversationId !== null && visible ? `session-usage:${model.profileId}:${model.conversationId}` : null, () => usageSession(model.profileId, model.conversationId!), 30_000);
  const empty = history.data !== null && history.current && history.data.totals.messages === 0;
  const known = model.conversationId !== null && !empty;
  const { models, why } = useTranscriptModels(model.profileId, model.conversationId, visible && !known, model.model);
  const accountTitle = account && account.state === "signed_in" ? accountName(account) : null;
  if (known) {
    return (
      <div className="mx-auto flex w-full max-w-[1440px] flex-col gap-grid" data-testid="session-usage">
        <SessionUsage profile={model.profileId} conversation={model.conversationId!} profileLabel={model.profileLabel} />
        <div className="grid grid-cols-1 gap-grid lg:grid-cols-2">
          <Runway display={display} title={accountTitle} heading="Its account's window" testId="session-usage-runway">
            <p className="border-t border-hairline pt-3 text-label leading-[1.5] text-muted">The window counts every session on the account; this session's share is above.</p>
          </Runway>
        </div>
      </div>
    );
  }
  return (
    <SessionUsageTab
      tokens={tokens}
      models={model.conversationId === null && !model.model ? [] : models}
      modelsWhy={why}
      display={display}
      accountTitle={accountTitle}
    />
  );
}
