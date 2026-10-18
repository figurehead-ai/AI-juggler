/**
 * AJ-E1206 vitest — Your Macs on Figurehead: hidden unless Fleet
 * discovery is on; listed Macs show their pairing state in words; pairing
 * shows both fingerprints and needs "Both fingerprints match" before it
 * pins (View only); a Mac without a key says to use a code.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { AccountMacsCard } from "./AccountMacsCard";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_ACCOUNT_MACS_CALLS__ as string[] | undefined) ?? [];
const linked = (on: boolean) => ({
  state: "linked", email: "you@example.com", plan: "Free", device_name: "Studio",
  features: [{ key: "fleetDiscovery", access: "included", tier: "free", on }],
  link: null, refreshed_at_ms: 1, offline: false, notice: null, consent_version: 1, consent_current: 1,
});
const MACS = {
  this_fingerprint: "AAAA-BBBB-CCCC-DDDD",
  read_at_ms: 1,
  macs: [
    { device_id: "dev_mini", name: "Mac mini", platform: "macos", fingerprint: "EEEE-FFFF-GGGG-HHHH", state: "not_paired", last_seen_at: null },
    { device_id: "dev_old", name: "Old laptop", platform: "macos", fingerprint: null, state: "no_key", last_seen_at: null },
  ],
};

afterEach(cleanup);
beforeEach(() => {
  delete w().__AIJ_ACCOUNT_MACS__;
  delete w().__AIJ_ACCOUNT_MACS_CALLS__;
});

describe("AJ-E1206 account discovery", () => {
  it("is hidden and silent while discovery is off", async () => {
    w().__AIJ_FIGUREHEAD__ = linked(false);
    w().__AIJ_ACCOUNT_MACS__ = MACS;
    const { container } = render(<AccountMacsCard />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.querySelector("[data-testid='account-macs']")).toBeNull();
    expect(calls()).toEqual([]);
  });

  it("pins only after both fingerprints are confirmed", async () => {
    w().__AIJ_FIGUREHEAD__ = linked(true);
    w().__AIJ_ACCOUNT_MACS__ = MACS;
    const { container } = render(<AccountMacsCard />);
    await waitFor(() => expect(container.querySelectorAll("[data-testid='account-mac']")).toHaveLength(2));
    expect(container.textContent).toContain("Can't pair this way — use a pairing code");
    fireEvent.click(container.querySelector("[data-testid='account-mac-compare']")!);
    expect(container.querySelector("[data-testid='compare-this']")!.textContent).toBe("AAAA-BBBB-CCCC-DDDD");
    expect(container.querySelector("[data-testid='compare-other']")!.textContent).toBe("EEEE-FFFF-GGGG-HHHH");
    const pair = () => container.querySelector<HTMLButtonElement>("[data-testid='compare-pair']")!;
    expect(pair().disabled).toBe(true);
    fireEvent.click(container.querySelector("[data-testid='compare-match']")!);
    await act(async () => {
      fireEvent.click(pair());
    });
    expect(calls()).toContain("pin:dev_mini:EEEE-FFFF-GGGG-HHHH");
    await waitFor(() => expect(container.textContent).toContain("waiting for it to confirm this Mac"));
  });
});
