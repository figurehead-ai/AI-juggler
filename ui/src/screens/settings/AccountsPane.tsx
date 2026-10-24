/**
 * AJ-E2110 — Settings › Accounts: renewal reminders (D-12), and the way
 * to the Accounts page, where each subscription is shown with the
 * profiles that use it (D-35).
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { RenewalSettingsCard } from "../../components/SettingsSections";
import { accountsList, type AccountInfo } from "../../lib/bridge";
import { Pane, PaneSection, SettingRow } from "./parts";

/** Every account (each profile's sign-in), read once for a pane. */
export function useAccounts(): AccountInfo[] {
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  useEffect(() => {
    void accountsList().then(setAccounts, () => setAccounts([]));
  }, []);
  return accounts;
}

export function AccountsPane({ anchor }: { anchor?: string }): JSX.Element {
  const accounts = useAccounts();
  return (
    <Pane id="accounts" title="Accounts" anchor={anchor}>
      <PaneSection
        id="accounts-page"
        title="Accounts and profiles"
        footer="An account is a subscription. A profile is the setup an app runs on; several can share one account."
      >
        <SettingRow
          title="Your accounts"
          description="Each subscription with its limits, and the profiles signed in to it."
          control={
            <Button size="sm" variant="secondary" onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "accounts" } }))} data-testid="settings-open-accounts">
              Open Accounts
            </Button>
          }
        />
      </PaneSection>
      <PaneSection id="renewal" title="Renewal dates" footer="Reminders come only from dates you enter. AI Juggler never reads your billing." card>
        <RenewalSettingsCard accounts={accounts} />
      </PaneSection>
    </Pane>
  );
}
