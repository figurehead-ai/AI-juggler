/**
 * 0.5.1 F36 (PL1 D2, F2): a Claude app profile removed and added again
 * holds its Claude Code folder (the folder is the profile's, rc22 A3),
 * and is on its own sign-in's Account (AJ-E3210), which may differ from
 * the folder's terminal sign-in. Then:
 * - the folder's Account card says why no app is on it, and opens the
 *   profile's page, instead of "No app uses this account yet · Add its
 *   app";
 * - the profile's own Account, known only from its sign-in (no folder of
 *   its own), has its card on Accounts;
 * - the App's page names its own Account, not its folder's.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { setToolsForTests } from "../../lib/tools";
import { PREVIEW_TOOLS } from "../../preview/tools";
import { AccountCard } from "./AccountCard";
import type { AppModel } from "../../lib/apps";
import { accountForApp } from "../../pages/app/AppTerms";
import { ProfilesScreen } from "../Profiles";
import * as bridgeLaunch from "../../lib/ipc/launch";
import * as bridgeAccounts from "../../lib/ipc/accounts";
import * as bridgeProviders from "../../lib/ipc/providers";
import * as bridgeQuota from "../../lib/ipc/quota";
import * as subs from "../../lib/ipc/subscriptions";

setToolsForTests(PREVIEW_TOOLS);
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const WORK: AccountInfo = { id: "work", provider: "claude-code", label: "work", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: true };
const PROFILE = "app:claude-app:/Users/someone/Library/Application Support/ClaudeProfiles/work";

const account = (id: string, folders: AccountInfo[], apps: string[], extra: Partial<SubscriptionInfo> = {}): SubscriptionInfo => ({
  key: `k-${id}`, account_id: id, name: null, display_name: id, colour_slot: null, tool: "claude-code", state: "signed_in",
  email: null, organisation: null, reason: null, plan: null, renewal_date: null, fee_usd: null, differs: [], choices: [],
  profiles: folders.map((f) => ({ id: f.id, label: f.label, folder: `~/${f.id}`, plan: null, renewal_date: null, fee_usd: null })),
  apps,
  ...extra,
});
// The folder's terminal sign-in, and the profile's own.
const FOLDERS = account("acct:folder", [WORK], [], { email: "w…@example.invalid" });
const OWN = account("acct:own", [], [PROFILE]);

const profileApp: AppModel = { id: PROFILE, kind: "desktop", tool: "claude-code", appId: "claude-app", app: "Claude app", profile: "work", name: "Claude app · work", defaultName: "Claude app · work", named: false, railKey: PROFILE, folder: WORK, accountId: "acct:own", place: null, slot: 0 };

describe("F36: a folder a desktop App holds", () => {
  it("its Account card says the folder is that App's and opens it, not Add its app", () => {
    render(
      <AccountCard
        apps={[profileApp]}
        subscription={FOLDERS} profiles={[WORK]} recipes={[]} displays={[]} sessions={[]} conversations={[]} now={0}
        canUseInPlace={() => false} reportsLimits={() => false} onLaunch={() => null} onDeleteRecipe={async () => undefined}
        onNewRecipe={() => null} onLogin={() => null} onRechecked={async () => undefined}
      />,
    );
    const empty = screen.getByTestId("account-apps-empty");
    expect(empty.textContent).toBe("Its folder is used by Claude app · work, which signs in on its own · Open it");
    expect(screen.queryByTestId("account-apps-add")).toBeNull();
    expect(screen.getByTestId("account-apps-holder")).toBeTruthy();
  });

  it("the App's page names its own Account, whichever comes first", () => {
    expect(accountForApp([FOLDERS, OWN], WORK.id, "acct:own")?.account_id).toBe("acct:own");
    // A CLI App on the folder (its row names the folder's Account) keeps it.
    expect(accountForApp([FOLDERS, OWN], WORK.id, null)?.account_id).toBe("acct:folder");
  });

  it("Accounts shows the App's own Account though no folder is its", async () => {
    vi.spyOn(bridgeLaunch, "launchRecipesList").mockResolvedValue([]);
    vi.spyOn(bridgeQuota, "quotaDisplayAll").mockResolvedValue([]);
    vi.spyOn(bridgeProviders, "providersCapabilities").mockResolvedValue([]);
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([WORK]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([FOLDERS, OWN]);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} />);
    const cards = await screen.findAllByTestId("account-card");
    expect(cards).toHaveLength(2);
  });
});
