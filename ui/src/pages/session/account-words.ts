/**
 * F3 (SK48-8, the 0.5.0 re-walk): a session page's Runway card was headed
 * a bare "Personal" and its Where it runs row "Personal · p…@…" when a
 * Claude Code and a Codex Account were both called Personal, while the
 * page's own "Apps that can take it over" named them apart. The session's
 * Account is named as every list of Accounts names it (accountNamesApart,
 * over every Account), on both overviews.
 */
import { accountLine, accountNamesApart, type SubscriptionInfo } from "../../lib/subscriptions";

export function sessionAccountWords(
  accounts: readonly SubscriptionInfo[] | null,
  account: SubscriptionInfo | undefined,
): { runway: string | null; where: string | null } {
  if (!account) return { runway: null, where: null };
  const name = accountNamesApart(accounts ?? [account])(account);
  return {
    runway: account.state === "signed_in" ? name : null,
    where: [name, accountLine(account)].filter(Boolean).join(" · "),
  };
}
