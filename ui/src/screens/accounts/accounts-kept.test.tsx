/**
 * 0.3.4, the page-switching report — Accounts showed a full skeleton on
 * every visit and read every account's sign-in again (forced). It now
 * opens on what the app holds and the last accounts read, reads the
 * sign-ins behind it only when they're old, marked old (a sign-in
 * started, an account changed) or don't cover the app's profiles, and
 * shows the skeleton only when nothing is known yet.
 */
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProfilesScreen } from "../Profiles";
import * as bridge from "../../lib/bridge";
import * as subs from "../../lib/ipc/subscriptions";
import { loadSubscriptions, staleSubscriptions, useSubscriptions, type SubscriptionInfo } from "../../lib/subscriptions";
import type { AccountInfo } from "../../lib/bridge";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const profile = (id: string, label: string): AccountInfo => ({
  id,
  provider: "claude-code",
  label,
  plan: null,
  login_state: "logged_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: true,
});

const account = (key: string, ids: string[]): SubscriptionInfo => ({
  key,
  tool: "claude-code",
  state: "signed_in",
  email: "o…@work.com",
  organisation: null,
  reason: null,
  profiles: ids.map((id) => ({ id, label: `Profile ${id}`, folder: `~/p/${id}`, plan: null, renewal_date: null, fee_usd: null })),
  plan: "Max",
  renewal_date: null,
  fee_usd: null,
  differs: [],
});

const WORK = [profile("w1", "Work")];
const open = (known = true) => render(<ProfilesScreen knownAccounts={known ? WORK : null} knownProfiles={known ? [] : null} openTerminal={() => {}} onAddAccounts={() => {}} />);

describe("0.3.4 Accounts opens on what's known", () => {
  let accountsList: ReturnType<typeof vi.spyOn>;
  let subscriptionsList: ReturnType<typeof vi.spyOn>;
  beforeEach(() => {
    accountsList = vi.spyOn(bridge, "accountsList").mockResolvedValue(WORK);
    vi.spyOn(bridge, "launchRecipesList").mockResolvedValue([]);
    vi.spyOn(bridge, "quotaDisplayAll").mockResolvedValue([]);
    vi.spyOn(bridge, "providersCapabilities").mockResolvedValue([]);
    subscriptionsList = vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("work", ["w1"])]);
  });

  it("the first visit with nothing read shows the placeholder, then the accounts", async () => {
    open(false);
    expect(screen.getByRole("status", { name: "Loading accounts" })).toBeTruthy();
    expect(await screen.findAllByTestId("account-card")).toHaveLength(1);
  });

  it("a return paints its accounts in the first frame, no skeleton, and no forced re-read", async () => {
    const first = open();
    await screen.findAllByTestId("account-card");
    await act(async () => undefined);
    first.unmount();
    const listed = accountsList.mock.calls.length;
    open();
    // The first frame: the cards, not the placeholder.
    expect(screen.queryByRole("status", { name: "Loading accounts" })).toBeNull();
    expect(screen.getAllByTestId("account-card")).toHaveLength(1);
    await act(async () => undefined);
    // Within half a minute, the sign-ins aren't read again; the app's own
    // lists aren't read again for it.
    expect(subscriptionsList).toHaveBeenCalledTimes(1);
    expect(accountsList.mock.calls.length).toBe(listed);
  });

  it("reads the sign-ins again behind the cards once they're marked old (a sign-in started)", async () => {
    const first = open();
    await screen.findAllByTestId("account-card");
    first.unmount();
    staleSubscriptions();
    open();
    expect(screen.getAllByTestId("account-card")).toHaveLength(1);
    await waitFor(() => expect(subscriptionsList).toHaveBeenCalledTimes(2));
  });

  it("reads them again when they don't cover the app's profiles (one added elsewhere)", async () => {
    await loadSubscriptions();
    render(<ProfilesScreen knownAccounts={[...WORK, profile("w2", "Work 2")]} knownProfiles={[]} openTerminal={() => {}} onAddAccounts={() => {}} />);
    await waitFor(() => expect(subscriptionsList).toHaveBeenCalledTimes(2));
  });
});

describe("0.3.4 useSubscriptions starts from the last answer", () => {
  it("a view opened again has its accounts in its first render", async () => {
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("work", ["w1"])]);
    const seen: (SubscriptionInfo[] | null)[] = [];
    function View() {
      seen.push(useSubscriptions());
      return null;
    }
    const first = render(<View />);
    await waitFor(() => expect(seen.at(-1)).not.toBeNull());
    first.unmount();
    seen.length = 0;
    render(<View />);
    expect(seen[0]?.map((row) => row.key)).toEqual(["work"]);
  });

  it("a failed read keeps the last answer", async () => {
    const list = vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("work", ["w1"])]);
    await loadSubscriptions();
    list.mockRejectedValue(new Error("gone"));
    await expect(loadSubscriptions(true)).resolves.toHaveLength(1);
  });
});
