/**
 * D-62 (AJ-E3205) — an Account's Settings: its name (yours, or the
 * default), and its renewal and fee, set once for the Account: they're
 * written to each App on it, so one plan never shows two dates. Its plan
 * is read from the sign-in.
 */
import { useId, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { RenameSheet } from "../../components/ManageWork";
import { PaneSection, RowNote, SettingRow } from "../../screens/settings/parts";
import { accountSetFee, accountSetRenewal, errorText } from "../../lib/bridge";
import { subscriptionsRename } from "../../lib/ipc/subscriptions";
import { accountName, planWords, type SubscriptionInfo } from "../../lib/subscriptions";

export function AccountSettings({ sub, all, onChanged }: { sub: SubscriptionInfo; all: readonly SubscriptionInfo[]; onChanged: () => void }): JSX.Element {
  const id = useId();
  const [renaming, setRenaming] = useState(false);
  const [fee, setFee] = useState(sub.fee_usd != null ? String(sub.fee_usd) : "");
  const [error, setError] = useState<string | null>(null);
  const name = accountName(sub, all);
  const folders = sub.profiles.map((profile) => profile.id);
  const everyApp = async (write: (folder: string) => Promise<unknown>) => {
    setError(null);
    try {
      await Promise.all(folders.map(write));
      onChanged();
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  const saveFee = () => {
    const parsed = fee.trim() === "" ? null : Number.parseFloat(fee);
    if (parsed !== null && (Number.isNaN(parsed) || parsed < 0)) {
      setError("The fee is a number of dollars a month, like 200.");
      return;
    }
    void everyApp((folder) => accountSetFee(folder, parsed));
  };
  const differs = (what: string) => sub.differs.includes(what);
  return (
    <div className="flex w-full max-w-content flex-col gap-grid" data-testid="account-settings-tab">
      <PaneSection id="account-name" title="Name">
        <SettingRow
          title={name}
          description={sub.name ? "Your name for this account." : "The name AI Juggler gives it: its team, else its plan, else its email."}
          testId="account-name-row"
          control={
            <>
              {sub.name && sub.account_id && (
                <Button size="sm" variant="ghost" onClick={() => void subscriptionsRename(sub.account_id!, null).then(onChanged)} data-testid="account-name-reset">
                  Use the default
                </Button>
              )}
              <Button size="sm" variant="secondary" onClick={() => setRenaming(true)} disabled={!sub.account_id} data-testid="account-rename">
                Rename
              </Button>
            </>
          }
        />
      </PaneSection>
      <PaneSection id="account-terms" title="Plan, renewal and fee" footer="Entered by you, for the whole account: AI Juggler doesn't read your billing.">
        <SettingRow title="Plan" description={sub.plan ? `${planWords(sub.plan)}, as its sign-in says.` : "Its sign-in doesn't say."} testId="account-plan-row" />
        <SettingRow
          title="Renewal date"
          titleId={`${id}-renewal`}
          description={differs("renewal") ? "Its apps had different dates; a date here is set for all of them." : "When the plan renews: its limits reset then."}
          testId="account-renewal-row"
          control={
            <input
              type="date"
              aria-labelledby={`${id}-renewal`}
              value={sub.renewal_date ?? ""}
              onChange={(event) => void everyApp((folder) => accountSetRenewal(folder, event.target.value === "" ? null : event.target.value))}
              className="rounded-lg border border-hairline bg-surface px-2 py-1 text-meta text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
              data-testid="account-renewal-input"
            />
          }
        />
        <SettingRow
          title="Monthly fee"
          titleId={`${id}-fee`}
          description={differs("fee") ? "Its apps had different fees; a fee here is set for all of them." : "What you pay a month, in dollars: cost per session uses it."}
          testId="account-fee-row"
          control={
            <span className="flex items-center gap-1.5">
              <input
                aria-labelledby={`${id}-fee`}
                value={fee}
                onChange={(event) => setFee(event.target.value)}
                placeholder="e.g. 200"
                inputMode="decimal"
                className="w-20 rounded-lg border border-hairline bg-surface px-2 py-1 text-meta text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
                data-testid="account-fee-input"
              />
              <Button size="sm" variant="secondary" onClick={saveFee} data-testid="account-fee-save">
                Save
              </Button>
            </span>
          }
        >
          {error && <RowNote tone="warning">{error}</RowNote>}
        </SettingRow>
      </PaneSection>
      {renaming && sub.account_id && (
        <RenameSheet
          title={`Rename ${name}`}
          initial={sub.name ?? ""}
          placeholder={name}
          hint="Shown wherever this account is named. Empty goes back to the default."
          testId="account-rename-sheet"
          onSave={async (next) => {
            await subscriptionsRename(sub.account_id!, next.trim() || null);
            setRenaming(false);
            onChanged();
          }}
          onClose={() => setRenaming(false)}
        />
      )}
    </div>
  );
}
