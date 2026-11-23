/**
 * rc22 (A3), "instant and paged": with the backend live and an Account's
 * Apps not read yet, its card never shows its folders as Apps (the rows
 * the rc23 walk flagged) nor a count made from them: a placeholder row
 * and count until they're read. Read, the rows and the count are the
 * backend's list (`SubscriptionInfo.apps`), before the Apps are.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { AccountInfo } from "../types/generated/AccountInfo";
import type { SubscriptionInfo } from "./subscriptions";
import { setToolsForTests } from "./tools";
import { PREVIEW_TOOLS } from "../preview/tools";
import { AccountCard } from "../screens/accounts/AccountCard";
import type { AppModel } from "./apps";
import { ADD_APP } from "./found-tools";

vi.mock("./bridge", async (original) => ({ ...(await original<typeof import("./bridge")>()), hasLiveBackend: () => true }));

setToolsForTests(PREVIEW_TOOLS);
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

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

function card(subscription: SubscriptionInfo, apps?: readonly AppModel[]) {
  return render(
    <AccountCard
      apps={apps}
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

// AJ-E1552: the 0.5.0 Nightly's card stuck as a skeleton for over a
// minute. Its Account named a desktop App (a Claude app seen running and
// linked to its folder, not added) that the Apps list never brings a
// model for, and the card waited on that model for ever.
describe("an Account's Apps row: loading, empty, error (AJ-E1552)", () => {
  const notAdded = "app:claude-app:/p/Seen running";

  it("once the Apps are read, an App with no model isn't waited on: the row is empty, with its next step", () => {
    card(sub([notAdded]), []);
    expect(screen.queryByTestId("account-apps-pending")).toBeNull();
    expect(screen.queryByTestId("account-profiles-count-pending")).toBeNull();
    expect(screen.getByTestId("account-apps-empty").textContent).toBe("No app uses this account yet · Add its app");
  });

  it("an Account with no App: the empty state, and Add its app opens the tool's Add flow", () => {
    card(sub([]), []);
    const asked: string[] = [];
    const listen = (event: Event) => asked.push((event as CustomEvent<{ tool: string }>).detail.tool);
    window.addEventListener(ADD_APP, listen);
    fireEvent.click(screen.getByTestId("account-apps-add"));
    window.removeEventListener(ADD_APP, listen);
    expect(asked).toEqual(["claude-code"]);
    expect(screen.queryByTestId("account-apps-pending")).toBeNull();
  });

  it("Apps that never come: Still loading after 3 s, then an error with Retry after the deadline", () => {
    vi.useFakeTimers();
    vi.spyOn(console, "warn").mockImplementation(() => {});
    card(sub(["cli:work", notAdded]));
    expect(screen.getByTestId("account-apps-pending")).toBeTruthy();
    expect(screen.queryByTestId("loading-slow")).toBeNull();
    act(() => void vi.advanceTimersByTime(3_000));
    expect(screen.getByTestId("loading-slow").textContent).toBe("Still loading…");
    act(() => void vi.advanceTimersByTime(9_000));
    expect(screen.queryByTestId("account-apps-pending")).toBeNull();
    expect(screen.getByRole("alert").textContent).toContain("Couldn't load this account's apps.");
    expect(console.warn).toHaveBeenCalled();
    // Retry reads again: loading, its clock from zero.
    fireEvent.click(screen.getByTestId("loading-retry"));
    expect(screen.getByTestId("account-apps-pending")).toBeTruthy();
    expect(screen.queryByTestId("loading-slow")).toBeNull();
  });
});
