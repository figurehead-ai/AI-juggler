/**
 * AJ-E2108 — a session page's Usage tab, for any session: its counts,
 * the model(s) (AI Juggler's own record, else the end of its transcript,
 * read once while the tab is shown) and its account's window from the
 * shared runway read. The tab's content is `SessionUsageTab`.
 */
import type { JSX } from "react";
import { accountDisplay, accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import { useRunways } from "./data";
import type { SessionModel } from "./model";
import { SessionUsageTab, useTranscriptModels, type TokenKinds } from "./UsageTab";

export function UsagePanel({ model, visible, tokens, account }: { model: SessionModel; visible: boolean; tokens: TokenKinds | null; account: SubscriptionInfo | undefined }): JSX.Element {
  const displays = useRunways();
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === model.profileId);
  const { models, why } = useTranscriptModels(model.profileId, model.conversationId, visible, model.model);
  return (
    <SessionUsageTab
      tokens={tokens}
      models={model.conversationId === null && !model.model ? [] : models}
      modelsWhy={why}
      display={display}
      accountTitle={account && account.state === "signed_in" ? accountName(account) : null}
    />
  );
}
