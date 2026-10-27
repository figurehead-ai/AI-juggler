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
  carry: { carried: ["The whole conversation"], regenerated: [], dropped: [] },
  redaction: {
    global: "strip",
    override: null,
    effective: "strip",
    source: "global",
    line: "Redaction: Strip",
  },
  redaction_items: [],
  carry_items: [{ label: "The whole conversation", hint: "Every message from the start.", how: "moves" }],
  move_line: "The whole conversation moves — all 12 messages, its subagents and file history. The original stays as it was.",
  secrets: { asks: false, reader: "Claude Code (Anthropic)" },
  target_tool: "claude-code",
};

type Seams = {
  __AIJ_SWITCH_PREFLIGHT__?: PreflightInfoData;
  __AIJ_SWITCH_SETUP__?: typeof EXAMPLE_SETUP;
  __AIJ_SWITCH_EXECUTE_FAIL__?: string;
};

beforeEach(() => {
  (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__ = PREFLIGHT;
  (window as never as Seams).__AIJ_SWITCH_SETUP__ = EXAMPLE_SETUP;
});
afterEach(() => {
  cleanup();
  delete (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__;
  delete (window as never as Seams).__AIJ_SWITCH_SETUP__;
  delete (window as never as Seams).__AIJ_SWITCH_EXECUTE_FAIL__;
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
    // Something the account turns on itself: no box to tick, but the
    // adapter's steps, its page and Check again — and it never blocks.
    expect(view.queryByLabelText(/team-style/)).toBeNull();
    const steps = view.getByTestId("switch-setup-steps");
    expect(steps.textContent).toContain("You can switch now. Personal won't have team-style until you turn it on");
    expect(steps.querySelectorAll("ol li").length).toBe(3);
    expect(view.getByTestId("switch-setup-open-page").textContent).toContain("Open claude.ai settings");
    expect((view.getByTestId("switch-confirm") as HTMLButtonElement).disabled).toBe(false);
    expect(view.getByTestId("switch-summary-needs").textContent).toBe("2 things need you below; none of them stops the switch.");
  });

  it("says one thing in the singular", async () => {
    (window as never as Seams).__AIJ_SWITCH_SETUP__ = {
      ...EXAMPLE_SETUP,
      offers: EXAMPLE_SETUP.offers.filter((offer) => offer.id !== "skill-team-style"),
    };
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-summary-needs")).toBeTruthy());
    expect(view.getByTestId("switch-summary-needs").textContent).toBe("1 thing needs you below; it doesn't stop the switch.");
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

  it("keeps what was set up undoable when the switch then fails", async () => {
    // Review of #132 (M3): the failed view had no Undo, and nothing else did.
    (window as never as Seams).__AIJ_SWITCH_EXECUTE_FAIL__ = "The target account is signed out.";
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-failed")).toBeTruthy());
    expect(view.getByTestId("switch-setup-results").textContent).toContain(
      "Set up the skill release-notes",
    );
    fireEvent.click(view.getByRole("button", { name: "Undo: Set up the skill release-notes" }));
    await waitFor(() =>
      expect(view.getByTestId("switch-setup-results").textContent).toContain(
        "Removed the skill release-notes",
      ),
    );
  });

  it("says so when the target already has everything", () => {
    expect(setupSummary({ ...EXAMPLE_SETUP, offers: [] }, new Set())).toBe(
      "Personal already has everything Work Max has set up.",
    );
  });
});

describe("Check again", () => {
  it("reads the target again, and what was turned on leaves the list", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup-steps")).toBeTruthy());
    // The user turned it on in the account: the next read hasn't it.
    (window as never as Seams).__AIJ_SWITCH_SETUP__ = { ...EXAMPLE_SETUP, offers: EXAMPLE_SETUP.offers.filter((offer) => offer.steps.length === 0) };
    fireEvent.click(view.getByTestId("switch-setup-recheck"));
    await waitFor(() => expect(view.queryByTestId("switch-setup-steps")).toBeNull());
    // What was ticked stays ticked.
    expect((view.getByLabelText(/release-notes/) as HTMLInputElement).checked).toBe(true);
  });

  it("opens the account's page through the app, never as a link the screen holds", async () => {
    const opened: Array<{ kind: string; id: string }> = [];
    (window as never as { __AIJ_OPENED__?: typeof opened }).__AIJ_OPENED__ = opened;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup-open-page")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-setup-open-page"));
    expect(opened[0]?.kind).toBe("account-settings");
  });
});
