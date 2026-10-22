import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import { setupSummary } from "./SwitchSetupList";
import { EXAMPLE_SETUP } from "../lib/ipc/switchSetup";
import type { PreflightInfoData } from "../lib/bridge";

/**
 * AJ-E1542/AJ-E1543: the switch shows what the target lacks and sets up
 * only what the user leaves ticked. Safe items start ticked; one that
 * needs the user starts unticked with what to do; one that can't be set
 * up has no box at all. Nothing is set up until the switch runs.
 */

const PREFLIGHT: PreflightInfoData = {
  mechanism: "transplant",
  fidelity: "F4",
  fidelity_words: "Continues exactly",
  score: 1.0,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: false,
  source_keeps_running: false,
  experimental_copy: "The source stays resumable.",
  door: "one-click",
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: { carried: ["chat transcript"], regenerated: [], dropped: [] },
  redaction: {
    global: "strip",
    override: null,
    effective: "strip",
    source: "global",
    line: "Redaction: Strip",
  },
  redaction_items: [],
};

type Seams = {
  __AIJ_SWITCH_PREFLIGHT__?: PreflightInfoData;
  __AIJ_SWITCH_SETUP__?: typeof EXAMPLE_SETUP;
};

beforeEach(() => {
  (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__ = PREFLIGHT;
  (window as never as Seams).__AIJ_SWITCH_SETUP__ = EXAMPLE_SETUP;
});
afterEach(() => {
  cleanup();
  delete (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__;
  delete (window as never as Seams).__AIJ_SWITCH_SETUP__;
});

function mount(onSwitched = vi.fn()) {
  return render(
    <SwitchFlow
      sessionId="sess-1"
      targetProfileId="p-1"
      targetLabel="Personal"
      onClose={() => {}}
      onSwitched={onSwitched}
    />,
  );
}

describe("what the target lacks", () => {
  it("lists each item with what will happen, safe ones ticked", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    expect(view.getByTestId("switch-setup-summary").textContent).toBe(
      "Personal doesn't have 3 things Work Max has. 1 will be set up.",
    );
    const skill = view.getByLabelText(/release-notes/) as HTMLInputElement;
    expect(skill.checked).toBe(true);
    const server = view.getByLabelText(/github/) as HTMLInputElement;
    expect(server.checked).toBe(false);
    expect(view.getByText(/Needs you: it needs GITHUB_TOKEN/)).toBeTruthy();
    // Can't be set up: shown, with why, and no box to tick.
    expect(view.getByText(/Can't be set up: it comes with the claude.ai account/)).toBeTruthy();
    expect(view.queryByLabelText(/team-style/)).toBeNull();
  });

  it("sets up what's ticked when the switch runs, and says what it did", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    fireEvent.click(view.getByLabelText(/github/));
    expect(view.getByTestId("switch-setup-summary").textContent).toContain("2 will be set up.");
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-setup-results")).toBeTruthy());
    const results = view.getByTestId("switch-setup-results").textContent ?? "";
    expect(results).toContain("Set up on Personal");
    expect(results).toContain("Set up the skill release-notes");
    expect(results).toContain("Set up the MCP server github");
    expect(results).not.toContain("team-style");
  });

  it("can undo each set-up afterwards", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-setup-results")).toBeTruthy());
    fireEvent.click(view.getByRole("button", { name: "Undo: Set up the skill release-notes" }));
    await waitFor(() =>
      expect(view.getByTestId("switch-setup-results").textContent).toContain(
        "Removed the skill release-notes",
      ),
    );
    expect(view.queryByRole("button", { name: /^Undo/ })).toBeNull();
  });

  it("says so when the target already has everything", () => {
    expect(setupSummary({ ...EXAMPLE_SETUP, offers: [] }, new Set())).toBe(
      "Personal already has everything Work Max has set up.",
    );
  });
});
