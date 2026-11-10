/**
 * D-62 (AJ-E3202 req. 4) — Settings › Accounts › Renewal dates, one row
 * per Account: its renewal is set once there for every App on it. Where
 * its Apps said different dates, the row says so and opens the Account's
 * Settings to choose (asked once, with what it changes).
 */
import { useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { DateField } from "../../components/controls";
import { revertedWords } from "../../lib/optimistic";
import { openAccount } from "../../lib/accounts-page";
import { subscriptionsSetTerm } from "../../lib/ipc/subscriptions";
import { accountName, staleSubscriptions, type SubscriptionInfo } from "../../lib/subscriptions";

export function AccountRenewalRow({ sub, onChanged }: { sub: SubscriptionInfo; onChanged: () => void }): JSX.Element {
  const [date, setDate] = useState(sub.renewal_date ?? "");
  const [error, setError] = useState<string | null>(null);
  const name = accountName(sub);
  const id = `settings-renewal-${sub.key}`;
  const commit = (next: string | null) => {
    setError(null);
    const previous = sub.renewal_date ?? "";
    void subscriptionsSetTerm(sub.account_id!, "renewal", next).then(
      () => {
        staleSubscriptions();
        onChanged();
      },
      (cause) => {
        setDate(previous);
        setError(revertedWords(cause));
      },
    );
  };
  const choose = sub.differs.includes("renewal");
  return (
    <div className="flex flex-wrap items-center gap-2 py-2.5 text-xs" data-testid="settings-renewal" data-account-id={sub.account_id}>
      {choose ? (
        <span className="min-w-44 font-medium text-ink">{name}</span>
      ) : (
        <label htmlFor={id} className="min-w-44 font-medium text-ink">
          {name}
        </label>
      )}
      {choose ? (
        <>
          <span className="text-label text-warning">Its apps say different dates</span>
          <Button size="sm" variant="secondary" onClick={() => openAccount(sub.account_id!, "settings")} data-testid="settings-renewal-choose">
            Choose…{" "}<span className="sr-only">the renewal date for {name}</span>
          </Button>
        </>
      ) : (
        <>
          {/* A8: saved whole, on commit; unset reads "Not set", never today. */}
          <DateField
            id={id}
            value={date || null}
            onCommit={(next) => {
              setDate(next ?? "");
              commit(next);
            }}
            size="sm"
            className="w-32"
          />
          {date ? (
            <Button size="sm" variant="secondary" onClick={() => commit(null)}>
              Clear{" "}<span className="sr-only">the renewal date for {name}</span>
            </Button>
          ) : (
            <span className="text-label text-muted">Not set: no renewal reminders</span>
          )}
          {sub.profiles.length > 1 && <span className="text-label text-muted">for its {sub.profiles.length} apps</span>}
        </>
      )}
      {error && (
        <span className="text-label text-danger" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}
