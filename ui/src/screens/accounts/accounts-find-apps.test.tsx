/**
 * D-97 (3): an empty Accounts page offers "Find my apps", first run's own
 * scan and found step (never a second scan), said in Account words; Add
 * an app (the tool list) stays beside it.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProfilesScreen } from "../Profiles";
import * as subscriptions from "../../lib/subscriptions";
import { account } from "../../test/fixtures";

afterEach(cleanup);

describe("D-97 the empty Accounts page", () => {
  it("offers Find my apps, which opens first run's scan, and Add an app beside it", async () => {
    const onFindApps = vi.fn();
    const onAddAccounts = vi.fn();
    render(<ProfilesScreen knownAccounts={[]} knownProfiles={[]} openTerminal={() => {}} onAddAccounts={onAddAccounts} onFindApps={onFindApps} />);
    const empty = await screen.findByTestId("accounts-empty");
    expect(empty.textContent).toContain("No accounts yet");
    expect(empty.textContent).not.toContain("No apps yet");
    fireEvent.click(screen.getByTestId("accounts-find-apps"));
    expect(onFindApps).toHaveBeenCalledOnce();
    expect(onAddAccounts).not.toHaveBeenCalled();
    expect(screen.getByTestId("accounts-find-apps").textContent).toBe("Find my apps");
    fireEvent.click(screen.getByTestId("accounts-empty-add"));
    expect(onAddAccounts).toHaveBeenCalledOnce();
  });

  // rc30: right after "Bring in 2 apps" both cards read "Account unknown …
  // just added: its sign-in is read next time", with skeleton bars, until
  // the page was left and opened again.
  it("reads the sign-ins of the apps brought in at once, on the page as it is", async () => {
    const reads = vi.spyOn(subscriptions, "loadSubscriptions");
    const props = { knownProfiles: [], openTerminal: () => {}, onAddAccounts: () => {} };
    const { rerender } = render(<ProfilesScreen knownAccounts={[]} {...props} />);
    await screen.findByTestId("accounts-empty");
    await waitFor(() => expect(reads).toHaveBeenCalled());
    const before = reads.mock.calls.length;
    const brought = [account({ id: "lab-1", label: "Lab" })];
    rerender(<ProfilesScreen knownAccounts={brought} {...props} />);
    await waitFor(() => expect(reads.mock.calls.length).toBeGreaterThan(before));
    expect(reads.mock.calls.at(-1)).toEqual([true]);
    reads.mockRestore();
  });
});
