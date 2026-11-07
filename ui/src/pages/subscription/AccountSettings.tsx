/**
 * D-62 (AJ-E3205) — an Account's Settings: its name (yours, or the
 * default), then its plan, renewal and fee, kept on the Account once for
 * every App on it (AJ-E3202 req. 4). Where its Apps said different things
 * before, the choice comes first, asked once (TermChoice); its Apps'
 * Settings point here.
 */
import { useId, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { RenameSheet } from "../../components/ManageWork";
import { PaneSection, RowNote, SettingRow } from "../../screens/settings/parts";
import { errorText } from "../../lib/bridge";
import { subscriptionsRename, subscriptionsSetTerm, type Term } from "../../lib/ipc/subscriptions";
import { accountName, planWords, type SubscriptionInfo } from "../../lib/subscriptions";
import { termNoun } from "../../lib/account-terms";
import { TermChoice } from "./TermChoice";

const FIELD = "rounded-lg border border-hairline bg-surface px-2 py-1 text-meta text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60";

export function AccountSettings({ sub, now, onChanged }: { sub: SubscriptionInfo; now: number; onChanged: () => void }): JSX.Element {
  const [renaming, setRenaming] = useState(false);
  const name = accountName(sub);
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
      {/* Keyed by the values, so a choice made or a value set elsewhere shows at once. */}
      <AccountTerms key={`${sub.plan}|${sub.renewal_date}|${sub.fee_usd}|${sub.differs.join()}`} sub={sub} now={now} onChanged={onChanged} />
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

/** Its plan, renewal date and monthly fee: the choices first, then a row
 * each. */
function AccountTerms({ sub, now, onChanged }: { sub: SubscriptionInfo; now: number; onChanged: () => void }): JSX.Element {
  const id = useId();
  // A plan as it's said ("Max 20x", not "max-20x"); the same plan either way.
  const said = sub.plan ? planWords(sub.plan) : "";
  const [plan, setPlan] = useState(said);
  const [fee, setFee] = useState(sub.fee_usd != null ? String(sub.fee_usd) : "");
  const [error, setError] = useState<{ term: Term; text: string } | null>(null);
  const set = async (term: Term, value: string | null) => {
    if (!sub.account_id) return;
    setError(null);
    try {
      await subscriptionsSetTerm(sub.account_id, term, value);
      onChanged();
    } catch (cause) {
      setError({ term, text: errorText(cause) });
    }
  };
  const saveFee = () => {
    const parsed = fee.trim() === "" ? null : Number.parseFloat(fee);
    if (parsed !== null && (Number.isNaN(parsed) || parsed < 0)) {
      setError({ term: "fee", text: "The fee is a number of dollars a month, like 200." });
      return;
    }
    void set("fee", parsed === null ? null : String(parsed));
  };
  const open = (term: Term) => sub.differs.includes(term);
  const note = (term: Term) => error?.term === term && <RowNote tone="warning">{error.text}</RowNote>;
  const pending = (term: Term, otherwise: string) => (open(term) ? `Its apps say different ${termNoun(term, true)}: choose above, or set one here.` : otherwise);
  return (
    <PaneSection id="account-terms" title="Plan, renewal and fee" footer="Entered by you, once for the whole account and every app on it: AI Juggler doesn't read your billing.">
      {sub.choices.map((choice) => (
        <TermChoice key={choice.term} sub={sub} choice={choice} now={now} onChosen={onChanged} />
      ))}
      <SettingRow
        title="Plan"
        titleId={`${id}-plan`}
        description={pending("plan", "Like Max or Pro: in its name until you give it one.")}
        testId="account-plan-row"
        control={
          <span className="flex items-center gap-1.5">
            <input autoCorrect="off" autoCapitalize="off" spellCheck={false} aria-labelledby={`${id}-plan`} value={plan} onChange={(event) => setPlan(event.target.value)} placeholder="e.g. Max" maxLength={40} className={`w-28 ${FIELD}`} data-testid="account-plan-input" />
            <Button size="sm" variant="secondary" onClick={() => void set("plan", plan.trim() || null)} disabled={plan.trim() === said} data-testid="account-plan-save">
              Save
            </Button>
          </span>
        }
      >
        {note("plan")}
      </SettingRow>
      <SettingRow
        title="Renewal date"
        titleId={`${id}-renewal`}
        description={pending("renewal", "When the plan renews: reminders come before it.")}
        testId="account-renewal-row"
        control={
          <input
            type="date"
            aria-labelledby={`${id}-renewal`}
            value={sub.renewal_date ?? ""}
            onChange={(event) => void set("renewal", event.target.value === "" ? null : event.target.value)}
            className={FIELD}
            data-testid="account-renewal-input"
          />
        }
      >
        {note("renewal")}
      </SettingRow>
      <SettingRow
        title="Monthly fee"
        titleId={`${id}-fee`}
        description={pending("fee", "What you pay a month, in dollars: cost per session uses it.")}
        testId="account-fee-row"
        control={
          <span className="flex items-center gap-1.5">
            <input autoCorrect="off" autoCapitalize="off" spellCheck={false} aria-labelledby={`${id}-fee`} value={fee} onChange={(event) => setFee(event.target.value)} placeholder="e.g. 200" inputMode="decimal" className={`w-20 ${FIELD}`} data-testid="account-fee-input" />
            <Button size="sm" variant="secondary" onClick={saveFee} data-testid="account-fee-save">
              Save
            </Button>
          </span>
        }
      >
        {note("fee")}
      </SettingRow>
    </PaneSection>
  );
}
