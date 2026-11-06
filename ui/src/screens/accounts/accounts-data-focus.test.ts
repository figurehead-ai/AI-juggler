/**
 * Walk A10: a sign-in changed in a terminal shows when you come back to
 * the window: the Accounts screen reads the sign-ins again on focus (when
 * older than half a minute, as on opening).
 */
import { act, renderHook } from "@testing-library/react";
import { expect, it, vi } from "vitest";

const loads: boolean[] = [];
vi.mock("../../lib/subscriptions", async (actual) => ({
  ...(await actual<typeof import("../../lib/subscriptions")>()),
  loadSubscriptions: (force = false) => {
    loads.push(force);
    return Promise.resolve([]);
  },
}));

it("reads the sign-ins again when the window is back", async () => {
  const { useAccountsData } = await import("./useAccountsData");
  const props = { knownAccounts: [], knownProfiles: [], onError: () => undefined };
  renderHook(() => useAccountsData(props));
  const opened = loads.length;
  expect(opened).toBeGreaterThan(0);
  await act(async () => {
    window.dispatchEvent(new Event("focus"));
  });
  expect(loads.length).toBe(opened + 1);
  // Not forced: the half-minute rule still holds.
  expect(loads[loads.length - 1]).toBe(false);
});
