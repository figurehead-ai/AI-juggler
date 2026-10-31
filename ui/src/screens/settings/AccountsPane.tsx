/**
 * AJ-E2110 — Settings › Accounts: renewal reminders (D-12), and the way
 * to the Accounts page, where each subscription is shown with the
 * profiles that use it (D-35).
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { RenewalSettingsCard } from "../../components/SettingsSections";
import { ToolMark } from "../../components/ToolMark";
import { ToolNewerLine, ToolUpdateChecks, useToolUpdates } from "../../components/ToolUpdateLine";
import { accountsList, providersDiscover, type AccountInfo, type ProviderInstallData } from "../../lib/bridge";
import { installWords } from "../../lib/providers";
import { useTools } from "../../lib/tools";
import { Pane, PaneSection, SettingRow } from "./parts";

/** What this Mac has of each tool, read once (null while reading). */
function useInstalls(): ProviderInstallData[] | null {
  const [rows, setRows] = useState<ProviderInstallData[] | null>(null);
  useEffect(() => {
    void providersDiscover().then(setRows, () => setRows([]));
  }, []);
  return rows;
}

/** 0.1.5: the tools you have accounts on — what's installed and how many
 * accounts each has (the tool page folded in here: the review of 0.1.4
 * found it added nothing Accounts didn't show). */
function ToolsSection({ accounts }: { accounts: AccountInfo[] }): JSX.Element | null {
  const installs = useInstalls();
  const tools = useTools().filter((tool) => accounts.some((account) => account.provider === tool.id));
  const [updates, setChecks] = useToolUpdates(tools.map((tool) => tool.id));
  if (tools.length === 0) return null;
  return (
    <PaneSection id="tools" title="Tools on this computer" footer="Read from each tool's install. AI Juggler updates none of them.">
      {tools.map((tool) => {
        const install = installs?.find((row) => row.provider === tool.id);
        const count = accounts.filter((account) => account.provider === tool.id).length;
        return (
          <SettingRow
            key={tool.id}
            testId="settings-tool"
            icon={<ToolMark tool={tool.id} size={20} className="text-muted" />}
            title={tool.display_name}
            description={`${count === 1 ? "1 account" : `${count} accounts`} · ${installs === null ? "checking what's installed…" : install ? installWords(install) : "Couldn't check what's installed"}`}
          >
            <ToolNewerLine info={updates[tool.id]} name={tool.display_name} />
          </SettingRow>
        );
      })}
      {Object.keys(updates).length > 0 && (
        <li className="px-4 py-3">
          <ToolUpdateChecks rows={updates} onChange={setChecks} />
        </li>
      )}
    </PaneSection>
  );
}

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
      <ToolsSection accounts={accounts} />
      <PaneSection id="renewal" title="Renewal dates" footer="Reminders come only from dates you enter. AI Juggler never reads your billing." card>
        <RenewalSettingsCard accounts={accounts} />
      </PaneSection>
    </Pane>
  );
}
