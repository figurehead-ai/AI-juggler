/**
 * rc22 (A3), "instant and paged": with the backend live and an Account's
 * Apps not read yet, its card never shows its folders as Apps (the rows
 * the rc23 walk flagged) nor a count made from them: a placeholder row
 * and count until they're read. Read, the rows and the count are the
 * backend's list (`SubscriptionInfo.apps`), before the Apps are.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { AccountInfo } from "../types/generated/AccountInfo";
import type { SubscriptionInfo } from "./subscriptions";
import { setToolsForTests } from "./tools";
import { PREVIEW_TOOLS } from "../preview/tools";
import { AccountCard } from "../screens/accounts/AccountCard";

vi.mock("./bridge", async (original) => ({ ...(await original<typeof import("./bridge")>()), hasLiveBackend: () => true }));

setToolsForTests(PREVIEW_TOOLS);
afterEach(cleanup);

const folder = (id: string, label: string): AccountInfo => ({
  id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: true,
});
const folders = [folder("work", "Work"), folder("work-max", "Work Max")];

function sub(apps?: string[]): SubscriptionInfo {
  return {
    key: "k-work", account_id: "acct:work", name: null, display_name: "Work", colour_slot: null, tool: "claude-code", state: "signed_in",
    email: null, organisation: null, reason: null, plan: null, renewal_date: null, fee_usd: null, differs: [], choices: [],
    profiles: folders.map((f) => ({ id: f.id, label: f.label, folder: `~/${f.id}`, plan: null, renewal_date: null, fee_usd: null })),
    ...(apps ? { apps } : {}),
  };
}

function card(subscription: SubscriptionInfo) {
  return render(
    <AccountCard
      subscription={subscription} profiles={folders} recipes={[]} displays={[]} sessions={[]} conversations={[]} now={0}
      canUseInPlace={() => false} reportsLimits={() => false} onLaunch={() => null} onDeleteRecipe={async () => undefined}
      onNewRecipe={() => null} onLogin={() => null} onRechecked={async () => undefined}
    />,
  );
}

describe("an Account's card while its Apps are read", () => {
  it("shows a placeholder, never its folders as Apps nor their count", () => {
    card(sub());
    expect(screen.queryAllByTestId("profile-row")).toHaveLength(0);
    expect(screen.queryByTestId("account-profiles-count")).toBeNull();
    expect(screen.getByTestId("account-apps-pending")).toBeTruthy();
  });

  it("once the Account's list is read: its rows and its count, before the Apps", () => {
    // work-max is a Claude app profile's folder: no CLI App of its own.
    card(sub(["cli:work", "app:claude-app:/p/Work Max"]));
    expect(screen.getAllByTestId("profile-row").map((row) => row.dataset.profile)).toEqual(["work"]);
    expect(screen.getByTestId("account-profiles-count").textContent).toBe("2 apps share this account");
    // The profile's row waits for the Apps' names.
    expect(screen.getByTestId("account-apps-pending")).toBeTruthy();
  });
});
