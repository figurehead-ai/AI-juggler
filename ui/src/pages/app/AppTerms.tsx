/**
 * D-62 (AJ-E3202 req. 4) — an App's plan, renewal and fee are its
 * Account's: set once there for every App on it. Its Settings show them
 * read-only, with "Set on its account" opening the Account's Settings.
 * An App whose sign-in isn't read yet has no Account: its folder keeps
 * its own (account/Settings.tsx).
 */
import type { JSX } from "react";
import { Button } from "../../components/primitives";
import { PaneSection, SettingRow } from "../../screens/settings/parts";
import { openAccount } from "../../lib/accounts-page";
import { accountName, planWords, type SubscriptionInfo } from "../../lib/subscriptions";
import { termNoun, termValueWords } from "../../lib/account-terms";
import type { Term } from "../../lib/ipc/subscriptions";
import { IconProfiles } from "../../components/icons";
import { IconStar } from "../../components/icons-actions";
import { IconCalendar } from "../../components/icons-controls";
import { IconCoin } from "../../components/icons-settings";

/** AJ-E1558 (D-126): each term's icon, as on the account's own Settings. */
const TERM_ICONS: Record<Term, typeof IconStar> = { plan: IconStar, renewal: IconCalendar, fee: IconCoin };

/** The stored Account an App is on: the one its row names, else its
 * folder's; `null` when it has none. 0.5.1 F36: its own first, so a
 * desktop App signed in apart from its folder's terminal sign-in shows
 * its own Account (AJ-E3210), whichever comes first in the list. */
export function accountForApp(subs: readonly SubscriptionInfo[], folderId: string | null, accountId: string | null): SubscriptionInfo | null {
  const own = accountId === null ? undefined : subs.find((sub) => sub.account_id === accountId);
  return own ?? subs.find((sub) => sub.account_id && folderId !== null && sub.profiles.some((profile) => profile.id === folderId)) ?? null;
}

export function AppTerms({ sub, now }: { sub: SubscriptionInfo; now: number }): JSX.Element {
  const name = accountName(sub);
  const value = (term: Term, raw: string | null): string => {
    if (sub.differs.includes(term)) return "To choose on its account";
    if (raw === null) return "Not set";
    return term === "plan" ? planWords(raw) : termValueWords(term, raw, now);
  };
  const rows: [Term, string | null][] = [
    // Optional in the type: a term left out reads as not set, never blank.
    ["plan", sub.plan ?? null],
    ["renewal", sub.renewal_date ?? null],
    ["fee", sub.fee_usd != null ? String(sub.fee_usd) : null],
  ];
  return (
    <PaneSection id="app-terms" title="Plan, renewal and fee">
      <SettingRow icon={IconProfiles}
        title={`On ${name}`}
        description="They're its account's, set once for every app on it."
        testId="app-terms-account"
        control={
          <Button size="sm" variant="secondary" onClick={() => openAccount(sub.account_id!, "settings")} data-testid="app-terms-open">
            Set on its account
          </Button>
        }
      />
      {rows.map(([term, raw]) => (
        <SettingRow icon={TERM_ICONS[term]}
          key={term}
          title={termNoun(term).replace(/^./, (first) => first.toUpperCase())}
          testId={`app-terms-${term}`}
          control={<span className={`text-meta ${raw === null && !sub.differs.includes(term) ? "text-muted" : "text-ink"}`}>{value(term, raw)}</span>}
        />
      ))}
    </PaneSection>
  );
}
