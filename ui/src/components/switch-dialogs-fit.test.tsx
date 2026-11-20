/**
 * AJ-E2311 item 4 (the 0.4.6 walk at 1280×800): Continue in another app…
 * didn't show "see what carries over" or a way out, and didn't scroll
 * there. Both of its dialogs (the limit's decision, and the list of
 * other apps) are never taller than the window: the title and the way
 * out are pinned, and what lies between them scrolls (jsdom has no
 * layout, so the frame's parts are what's checked; the visual gate
 * measures it at 1024×680).
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LimitDialog } from "./LimitDialog";
import { ContinueDialog } from "./ContinueDialog";
import type { SwitchTargetData } from "../lib/ipc/switch-targets";
import { clearSeams, instance, setSeam } from "../test/fixtures";

const NOW = 1_728_800_000_000;
const blocked = instance({ id: "s-1", status: "quota_blocked", account_id: "acct-work", account_label: "Work A", started_at_ms: NOW - 3_600_000, last_activity_ms: NOW - 60_000 });
const personal: SwitchTargetData = {
  recipe_id: "r-acct-personal",
  account_id: "acct-personal",
  account_label: "Personal",
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
  suggested: true,
};

beforeEach(() => {
  setSeam("__AIJ_SWITCH_TARGETS__", [personal]);
});

afterEach(() => {
  cleanup();
  clearSeams("__AIJ_SWITCH_TARGETS__");
});

/** A frame that fits: bounded, with its middle the part that scrolls. */
function expectFits(dialog: HTMLElement, body: HTMLElement, footer: HTMLElement): void {
  expect(dialog.className).toMatch(/\bmax-h-/);
  expect(dialog.className).toContain("overflow-hidden");
  expect(body.className).toContain("overflow-y-auto");
  expect(body.className).toContain("min-h-0");
  expect(footer.className).toContain("shrink-0");
  expect(body.contains(footer)).toBe(false);
}

describe("the switch's dialogs fit the window, the way out always in sight", () => {
  it("the limit's decision: what carries over scrolls; Not now stays", async () => {
    const onClose = vi.fn();
    render(<LimitDialog session={blocked} sessions={[blocked]} now={NOW} onContinue={() => null} onChooseAnother={() => null} onParked={() => null} onClose={onClose} />);
    const dialog = screen.getByTestId("limit-dialog");
    const body = screen.getByTestId("limit-body");
    const footer = screen.getByTestId("limit-footer");
    expectFits(dialog, body, footer);
    fireEvent.click(await within(body).findByRole("button", { name: "see what carries over" }));
    expect(within(body).getByTestId("limit-drops").textContent).toContain("tool approvals");
    expect(within(body).getByTestId("limit-continue")).toBeTruthy();
    fireEvent.click(within(footer).getByTestId("limit-not-now"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("Continue in another app: its list scrolls; Cancel stays under it", () => {
    const onClose = vi.fn();
    render(
      <ContinueDialog title="Continue in another app" onClose={onClose}>
        <p>Many apps</p>
      </ContinueDialog>,
    );
    const dialog = screen.getByTestId("continue-dialog");
    const body = screen.getByTestId("continue-dialog-body");
    const footer = screen.getByTestId("continue-dialog-footer");
    expectFits(dialog, body, footer);
    fireEvent.click(within(footer).getByTestId("continue-dialog-cancel"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
