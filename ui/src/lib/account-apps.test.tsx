/**
 * rc22 (A3, the rc23 walk): an Account's Apps are one list on every
 * screen: its card on Accounts (its rows and "N apps share this
 * account"), its page, Usage's runway chips and Home's runway line. Each
 * is the Apps list (`apps::users_apps`) narrowed to the Account, as
 * `fixtures/session-apps.json` says; the backend runs the same file
 * (`places::tests::every_accounts_apps_are_the_apps_list_on_its_folders`).
 * A Claude app profile's folder is no CLI App of its own until something
 * counts under it ("side").
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { AccountInfo } from "../types/generated/AccountInfo";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";
import type { StoredAppInfo } from "../types/generated/StoredAppInfo";
import type { SubscriptionInfo } from "./subscriptions";
import { appsOf } from "./apps";
import { accountApps } from "./account-items";
import { setToolsForTests } from "./tools";
import { PREVIEW_TOOLS } from "../preview/tools";
import { AccountCard } from "../screens/accounts/AccountCard";
import { appsOn } from "../components/HomeRunway";
import { runwayChips } from "../components/usage/RunwayCards";
import cases from "./fixtures/session-apps.json";

setToolsForTests(PREVIEW_TOOLS);
afterEach(cleanup);

const folders: AccountInfo[] = cases.folders.map((folder) => ({
  id: folder.id, provider: "claude-code", label: folder.label, plan: null, login_state: "logged_in", cleanup_opt_in: false,
  renewal_date: null, preview_opt_in: false, in_place: true,
}));
const places: KnownPlaceInfo[] = cases.places.map((place) => ({
  id: place.id, app_id: "claude-app", app: "Claude", instance: place.instance, profile: place.instance, label: `Claude · ${place.instance}`,
  app_pid: null, lost: false, keep_alive: false, paused: null, account_id: place.folder, seen_ms: place.seen_ms, cannot_start: null,
  recent: place.recent, found: false,
}));
/** The backend's Apps list for the cases: every Account's. */
const stored: StoredAppInfo[] = cases.accounts.flatMap((account) =>
  account.apps.map((id) => ({
    id, kind: id.startsWith("cli:") ? "cli" : "desktop", tool: "claude-code", host_app_id: null, label: null,
    folder_id: id.startsWith("cli:") ? id.slice(4) : (cases.places.find((place) => place.id === id)?.folder ?? null), account_id: null, last_seen_ms: 0,
  })),
);
const apps = appsOf({ stored, places, folders, listed: true });

function subscription(account: (typeof cases.accounts)[number]): SubscriptionInfo {
  return {
    key: `k-${account.name}`, account_id: `acct:${account.name}`, name: null, display_name: account.name, colour_slot: null, tool: "claude-code",
    state: "signed_in", email: null, organisation: null, reason: null,
    profiles: account.folders.map((id) => ({ id, label: id, folder: `~/${id}`, plan: null, renewal_date: null, fee_usd: null })),
    plan: null, renewal_date: null, fee_usd: null, differs: [], choices: [], apps: account.apps,
  };
}

const sorted = (ids: readonly string[]) => [...ids].sort();
const nameOf = (id: string) => apps.find((app) => app.id === id)?.name ?? id;

describe("an Account's Apps, the same on every screen (rc22, A3)", () => {
  it.each(cases.accounts)("$name: its page, Home's and Usage's runway list the Apps list's", (account) => {
    const sub = subscription(account);
    const on = new Set(account.folders);
    const profiles = folders.filter((folder) => on.has(folder.id));
    expect(sorted(accountApps(apps, sub, on, [], new Map()).map((row) => row.app.id))).toEqual(account.apps);
    expect(sorted(appsOn({ key: sub.key, name: account.name, detail: null, profiles, subscription: sub }, apps) ?? [])).toEqual(sorted(account.apps.map(nameOf)));
    expect(sorted((runwayChips(apps, sub, account.folders, new Map()) ?? []).map((chip) => chip.key))).toEqual(account.apps);
  });

  it.each(cases.accounts)("$name: its card's rows and its count", (account) => {
    const sub = subscription(account);
    const on = new Set(account.folders);
    render(
      <AccountCard
        subscription={sub} profiles={folders.filter((folder) => on.has(folder.id))} apps={apps} recipes={[]} displays={[]} sessions={[]} conversations={[]}
        now={0} canUseInPlace={() => false} reportsLimits={() => false} onLaunch={() => null} onDeleteRecipe={async () => undefined}
        onNewRecipe={() => null} onLogin={() => null} onRechecked={async () => undefined}
      />,
    );
    const cli = account.apps.filter((id) => id.startsWith("cli:"));
    expect(sorted(screen.queryAllByTestId("profile-row").map((row) => `cli:${row.dataset.profile ?? ""}`))).toEqual(cli);
    expect(screen.queryAllByTestId("account-app")).toHaveLength(account.apps.length - cli.length);
    expect(screen.getByTestId("account-profiles-count").textContent).toBe(`${account.apps.length} apps share this account`);
  });
});
