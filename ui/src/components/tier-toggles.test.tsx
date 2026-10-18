import { describe, expect, it } from "vitest";
import { render, waitFor } from "@testing-library/react";
import { TierToggles } from "./RightInspector";

/**
 * AJ-E508: the per-Account tier toggles — TWO separate switches
 * with the exact labels (Req.4), both off by default, never bundled.
 */

describe("TierToggles (AJ-E508)", () => {
  it("renders both exact labels, off by default, and loads the persisted state", async () => {
    // The bridge's mock path reads this seam (no backend in jsdom).
    (window as unknown as Record<string, unknown>).__AIJ_CLEANUP_TIERS__ = {
      tier1_opt_in: true,
      tier2_opt_in: false,
    };
    const view = render(<TierToggles accountId="acc-1" initial={false} />);
    expect(view.getByTestId("account-cleanup-toggle").textContent).toContain(
      "Automatically tidy left-behind dev servers",
    );
    expect(view.getByTestId("account-tier2-toggle").textContent).toContain(
      "Also tidy after a crash (with notification)",
    );
    // Loads the persisted truth once mounted.
    await waitFor(() =>
      expect(
        (view.getByTestId("account-cleanup-toggle").querySelector("input") as HTMLInputElement)
          .checked,
      ).toBe(true),
    );
    expect(
      (view.getByTestId("account-tier2-toggle").querySelector("input") as HTMLInputElement)
        .checked,
    ).toBe(false);
    view.unmount();
    delete (window as unknown as Record<string, unknown>).__AIJ_CLEANUP_TIERS__;
  });

  it("keeps the explanatory copy honest (exclusions + audit + both-off default)", () => {
    const view = render(<TierToggles accountId="acc-1" initial={false} />);
    const text = view.getByTestId("account-tier-toggles").textContent ?? "";
    expect(text).toContain("Both off by default");
    expect(text).toContain("never touched automatically");
    expect(text).toContain("audit-logged");
    expect(text).toContain("Undo window");
    view.unmount();
  });
});
