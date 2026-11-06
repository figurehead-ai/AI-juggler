/**
 * AJ-E3208 — the Accounts screen's switch between its Accounts and its
 * Apps, each counted; the one last chosen comes back (a per-viewer
 * convenience, kept in this browser only).
 */
import { useState, type JSX } from "react";
import { Segmented } from "../../components/usage/controls";

export type AccountsView = "accounts" | "apps";

const KEY = "aij.accounts.view";

/** The view last chosen, and how to choose another. */
export function useAccountsView(): [AccountsView, (next: AccountsView) => void] {
  const [view, setView] = useState<AccountsView>(() => {
    try {
      return localStorage.getItem(KEY) === "apps" ? "apps" : "accounts";
    } catch {
      return "accounts";
    }
  });
  const choose = (next: AccountsView) => {
    setView(next);
    try {
      localStorage.setItem(KEY, next);
    } catch {
      /* the choice just isn't remembered */
    }
  };
  return [view, choose];
}

export function AccountsViewSwitch({ view, onChange, accounts, apps }: { view: AccountsView; onChange: (next: AccountsView) => void; accounts: number; apps: number | null }): JSX.Element {
  return (
    <Segmented
      label="Show"
      options={[
        { id: "accounts", label: `Accounts ${accounts}` },
        { id: "apps", label: apps === null ? "Apps" : `Apps ${apps}` },
      ]}
      value={view}
      onChange={onChange}
      testId="accounts-view"
    />
  );
}
