import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import { SwitchSetupList, setupSummary } from "./SwitchSetupList";
import {
  EXAMPLE_SETUP,
  EXAMPLE_SETUP_UNUSED,
  SWITCHING_GUIDE_URL,
  switchSetupTurnOns,
  type OfferInfo,
  type SetupCheckInfo,
} from "../lib/ipc/switchSetup";
import type { PreflightInfoData } from "../lib/bridge";

/**
 * AJ-E1542/AJ-E1543: the switch shows what the target lacks and sets up
 * only what the user leaves ticked. Safe items start ticked; one that
 * needs the user starts unticked with what to do; one that comes with the
 * other account has no box, and says how to turn it on there. What this
 * conversation used leads; the rest waits under one collapsed line.
 * Nothing is set up until the switch runs.
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
  __AIJ_SWITCH_EXECUTE_FAIL__?: string;
  __AIJ_OPENED__?: string[];
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
  delete (window as never as Seams).__AIJ_OPENED__;
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
      "Personal doesn't have 3 things this conversation used. 1 will be set up, 1 you turn on yourself.",
    );
    const skill = view.getByRole("checkbox", { name: /release-notes/ }) as HTMLInputElement;
    expect(skill.checked).toBe(true);
    const server = view.getByRole("checkbox", { name: /github/ }) as HTMLInputElement;
    expect(server.checked).toBe(false);
    expect(view.getByText(/You can switch now, then finish it: it needs GITHUB_TOKEN/)).toBeTruthy();
    // Comes with the account: no box, says it doesn't hold up the switch.
    expect(view.queryByRole("checkbox", { name: /computer-use/ })).toBeNull();
    expect(
      view.getByText("You can switch now. Personal won't have computer-use until you turn it on."),
    ).toBeTruthy();
    const computer = view.getByText("computer-use").closest("li") as HTMLElement;
    expect(within(computer).getByText("From Anthropic, with Work Max's account")).toBeTruthy();
    expect(within(computer).getByText(/Needs a Pro or Max plan/)).toBeTruthy();
    expect(view.queryByText(/Can't be set up: it comes with the claude.ai account/)).toBeNull();
  });

  it("puts what wasn't used here under one closed line", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    const unused = view.getByTestId("switch-setup-unused") as HTMLDetailsElement;
    expect(unused.open).toBe(false);
    expect(unused.querySelector("summary")?.textContent).toBe(
      "Also on Work Max, not on Personal (2, not used in this conversation)",
    );
    expect(unused.textContent).toContain("deep-research");
    expect(unused.textContent).not.toContain("computer-use");
  });

  it("says how to turn it on, opens its page and checks again", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    expect(view.queryByTestId("turn-on-steps")).toBeNull();
    fireEvent.click(view.getByRole("button", { name: "Show me how to turn on computer-use" }));
    const steps = view.getByTestId("turn-on-steps");
    expect(steps.querySelectorAll("li")).toHaveLength(4);
    expect(steps.textContent).toContain("Sign in to claude.ai as Personal.");
    expect(steps.textContent).toContain("Start a session on Personal so it syncs.");
    const row = view.getByText("computer-use").closest("li") as HTMLElement;
    fireEvent.click(within(row).getByRole("button", { name: /^Open claude\.ai settings/ }));
    await waitFor(() =>
      expect((window as never as Seams).__AIJ_OPENED__).toEqual(["https://claude.ai/customize/skills"]),
    );
    fireEvent.click(view.getByRole("button", { name: "Check again for computer-use" }));
    await waitFor(() => expect(view.getByRole("status").textContent).toContain("Personal doesn't have it yet"));
  });

  it("leads with nothing when the conversation used nothing missing", async () => {
    (window as never as Seams).__AIJ_SWITCH_SETUP__ = EXAMPLE_SETUP_UNUSED;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    expect(view.getByTestId("switch-setup-summary").textContent).toBe(
      "Personal has everything this conversation used.",
    );
    expect(view.getAllByTestId("switch-setup-row")).toHaveLength(5);
    expect((view.getByTestId("switch-setup-unused") as HTMLDetailsElement).open).toBe(false);
  });

  it("sets up what's ticked when the switch runs, and says what it did", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    fireEvent.click(view.getByLabelText(/github/));
    expect(view.getByTestId("switch-setup-summary").textContent).toContain("2 will be set up,");
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-setup-results")).toBeTruthy());
    const results = view.getByTestId("switch-setup-results").textContent ?? "";
    expect(results).toContain("Set up on Personal");
    expect(results).toContain("Set up the skill release-notes");
    expect(results).toContain("Set up the MCP server github");
    expect(results).not.toContain("computer-use");
    // The switch kept a reminder for what Personal turns on itself.
    const reminders = await switchSetupTurnOns();
    expect(reminders.map((reminder) => reminder.name)).toContain("computer-use");
    expect(reminders.map((reminder) => reminder.name)).not.toContain("deep-research");
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

describe("each row's words", () => {
  const base: OfferInfo = {
    id: "o",
    kind: "skill",
    name: "x",
    scope: "user",
    verdict: "copy",
    note: null,
    about: null,
    suggested: false,
    used: null,
    origin: "own",
    provided_by: null,
    get_it: null,
    can_copy: false,
  };
  const check = (offers: OfferInfo[]): SetupCheckInfo => ({ from: "Work Max", to: "Personal", already_there: 0, offers });
  const cases: [Partial<OfferInfo>, string][] = [
    [{ verdict: "copy" }, "Set up as it is when you switch. You can undo it after."],
    [{ verdict: "translate", note: "as a Claude command" }, "Set up as a Claude command when you switch."],
    [{ verdict: "needs-you", note: "sign it in afterwards" }, "You can switch now, then finish it: sign it in afterwards"],
    [{ verdict: "turn-on", origin: "unknown", get_it: { steps: ["a"], open_url: null, needs: [], then: null } }, "You can switch now. Personal won't have x until you turn it on."],
    [{ verdict: "impossible", note: "it has no place here" }, "You can switch now. Personal won't have it: it has no place here"],
  ];
  for (const [offer, words] of cases) {
    it(`says what happens for ${offer.verdict}, with what it means and the guide`, () => {
      const view = render(<SwitchSetupList check={check([{ ...base, ...offer }])} picked={new Set()} onToggle={() => {}} />);
      expect(view.getByTestId("switch-setup-consequence").textContent).toBe(words);
      const meaning = view.getByTestId("setup-meaning");
      expect(meaning.querySelector("summary")?.textContent).toBe("What does this mean?");
      expect(meaning.textContent).toContain("For example:");
      fireEvent.click(view.getByRole("button", { name: /switching guide/ }));
      expect((window as never as Seams).__AIJ_OPENED__).toContain(SWITCHING_GUIDE_URL);
      const all = view.container.textContent ?? "";
      expect(all).not.toMatch(/[{}]|provider|target|\bpack\b/i);
    });
  }

  it("lets the account's own content be copied instead, never a provided one", () => {
    const toggle = vi.fn();
    const turnOn: Partial<OfferInfo> = { verdict: "turn-on", get_it: { steps: ["a"], open_url: null, needs: [], then: null } };
    const view = render(
      <SwitchSetupList
        check={check([
          { ...base, ...turnOn, id: "mine", name: "team-notes", origin: "account", can_copy: true },
          { ...base, ...turnOn, id: "theirs", name: "pdf", origin: "provided", provided_by: "Anthropic" },
        ])}
        picked={new Set()}
        onToggle={toggle}
      />,
    );
    expect(view.getByText("From Work Max's account: yours, your organization's or shared with you")).toBeTruthy();
    fireEvent.click(view.getByRole("checkbox", { name: "Copy team-notes to Personal now instead (only if you made it)" }));
    expect(toggle).toHaveBeenCalledWith("mine");
    expect(view.queryByRole("checkbox", { name: /pdf/ })).toBeNull();
  });

  it("never reads an unknown use as not used", () => {
    const view = render(
      <SwitchSetupList check={check([{ ...base, used: null }])} picked={new Set()} onToggle={() => {}} />,
    );
    expect(view.queryByTestId("switch-setup-unused")).toBeNull();
    expect(view.getByTestId("switch-setup-summary").textContent).toBe(
      "Personal doesn't have 1 thing Work Max has. 0 will be set up.",
    );
  });
});
