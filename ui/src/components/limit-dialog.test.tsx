/**
 * D-25 vitest — the account-limit decision: the reset in words, the
 * ranker's pick on another account (never the same one, never a filtered
 * one), Continue hands that target on, Park parks and says when it comes
 * back, and nothing happens until a choice is made.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LimitDialog, pickTarget } from "./LimitDialog";
import type { InstanceInfo, SwitchTargetData } from "../lib/bridge";

const NOW = 1_728_800_000_000;
const w = () => window as unknown as Record<string, unknown>;

const blocked = {
  id: "s-1",
  status: "quota_blocked",
  account_id: "acct-work",
  account_label: "Work A",
  quota: { used_percentage: 100, resets_at_ms: NOW + 130 * 60_000, confidence: "measured" },
  started_at_ms: NOW - 3_600_000,
  last_activity_ms: NOW - 60_000,
} as InstanceInfo;

const target = (account: string, label: string, extra: Partial<SwitchTargetData> = {}): SwitchTargetData => ({
  recipe_id: `r-${account}`,
  account_id: account,
  account_label: label,
  badge: "Full story, fresh start · Beta",
  validated: false,
  cross_provider: false,
  drops: ["tool approvals"],
  top_factors: [],
  factors: [],
  confidence: "computed",
  score: null,
  filtered_reason: null,
  tte_min: null,
  suggested: false,
  ...extra,
});

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
});

describe("D-25 limit decision — the pick", () => {
  it("prefers the ranker's suggestion on another account, skipping filtered ones", () => {
    const rows = [
      target("acct-work", "Work A", { suggested: true }),
      target("acct-lab", "Lab", { filtered_reason: "exhausted" }),
      target("acct-personal", "Personal", { suggested: true }),
    ];
    expect(pickTarget(rows, "acct-work")?.account_label).toBe("Personal");
    expect(pickTarget([target("acct-work", "Work A")], "acct-work")).toBeNull();
  });
});

describe("D-25 limit decision — the dialog", () => {
  const renderIt = (props: Partial<Parameters<typeof LimitDialog>[0]> = {}) =>
    render(
      <LimitDialog
        session={blocked}
        sessions={[blocked]}
        now={NOW}
        onContinue={props.onContinue ?? (() => null)}
        onChooseAnother={props.onChooseAnother ?? (() => null)}
        onParked={props.onParked ?? (() => null)}
        onClose={props.onClose ?? (() => null)}
      />,
    );

  it("says when it resets and that nothing is lost; offers the pick", async () => {
    w().__AIJ_SWITCH_TARGETS__ = [target("acct-personal", "Personal", { suggested: true })];
    const onContinue = vi.fn();
    const { getByTestId, findByRole } = renderIt({ onContinue });
    const dialog = getByTestId("limit-dialog");
    expect(dialog.getAttribute("role")).toBe("dialog");
    expect(dialog.textContent).toContain("Work A hit its limit");
    expect(dialog.textContent).toContain("in 2 h 10 min");
    expect(dialog.textContent).toContain("nothing is lost");
    fireEvent.click(await findByRole("button", { name: "Continue on Personal" }));
    expect(onContinue).toHaveBeenCalledWith(expect.objectContaining({ recipe_id: "r-acct-personal" }));
    expect(w().__AIJ_PARK_CALLS__).toBeUndefined();
  });

  it("says the switch is ready when it was prepared for the pick (AJ-E1513)", async () => {
    w().__AIJ_SWITCH_TARGETS__ = [target("acct-personal", "Personal", { suggested: true })];
    w().__AIJ_STAGED__ = {
      session_id: "s-1",
      target_recipe_id: "r-acct-personal",
      target_account_id: "acct-personal",
      target_account_label: "Personal",
      prepared_at_ms: NOW - 3 * 60_000,
      fidelity_words: "Full story",
    };
    const { findByTestId } = renderIt();
    expect((await findByTestId("limit-ready")).textContent).toBe("Ready to continue on Personal · prepared 3 min ago");
  });

  it("says nothing about a prepared switch for another account", async () => {
    w().__AIJ_SWITCH_TARGETS__ = [target("acct-personal", "Personal", { suggested: true })];
    w().__AIJ_STAGED__ = {
      session_id: "s-1",
      target_recipe_id: "r-acct-lab",
      target_account_id: "acct-lab",
      target_account_label: "Lab",
      prepared_at_ms: NOW - 30_000,
      fidelity_words: "Full story",
    };
    const { findByRole, queryByTestId } = renderIt();
    await findByRole("button", { name: "Continue on Personal" });
    expect(queryByTestId("limit-ready")).toBeNull();
  });

  it("a click on the dialog's text keeps focus inside it", async () => {
    const { default: userEvent } = await import("@testing-library/user-event");
    const user = userEvent.setup();
    const { getByTestId, getByText } = renderIt();
    const dialog = getByTestId("limit-dialog");
    await user.click(getByText(/nothing is lost/));
    expect(dialog.contains(document.activeElement)).toBe(true);
    expect(document.activeElement).not.toBe(document.body);
  });

  it("shows what doesn't carry over, on request", async () => {
    w().__AIJ_SWITCH_TARGETS__ = [target("acct-personal", "Personal")];
    const { findByRole, getByTestId } = renderIt();
    fireEvent.click(await findByRole("button", { name: "see what carries over" }));
    expect(getByTestId("limit-drops").textContent).toBe("Doesn't carry: tool approvals.");
  });

  it("Park parks the session and says it comes back at the reset", async () => {
    w().__AIJ_SWITCH_TARGETS__ = [];
    const onParked = vi.fn();
    const { findByTestId, getByTestId } = renderIt({ onParked });
    expect((await findByTestId("limit-no-target")).textContent).toContain("No other account can take this session");
    await act(async () => {
      fireEvent.click(within(getByTestId("limit-wait")).getByRole("button", { name: /^Park until/ }));
    });
    await waitFor(() => expect(getByTestId("limit-parked")).toBeTruthy());
    expect(w().__AIJ_PARK_CALLS__).toEqual(["park:s-1"]);
    expect(onParked).toHaveBeenCalledOnce();
  });

  it("Escape closes without doing anything", async () => {
    const onClose = vi.fn();
    const { getByTestId } = renderIt({ onClose });
    fireEvent.keyDown(getByTestId("limit-dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
    expect(w().__AIJ_PARK_CALLS__).toBeUndefined();
  });
});
