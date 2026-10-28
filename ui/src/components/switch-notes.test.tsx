import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import { setupSummary } from "./SwitchSetupList";
import { CarryList } from "./switch/CarryList";
import { notesQuestion } from "./switch/NotesRow";
import { EXAMPLE_SETUP, EXAMPLE_SETUP_NOTES } from "../lib/ipc/switchSetup";
import type { PreflightInfoData } from "../lib/bridge";

/**
 * D-51: the switch offers this project's notes (the memory a tool keeps
 * for itself) as one row, a question, never ticked for the user; copies
 * them only when ticked; says what is kept as it is; and, for a tool that
 * keeps no notes, says in What's carried that they stay behind.
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
  redaction: { global: "strip", override: null, effective: "strip", source: "global", line: "Redaction: Strip" },
  redaction_items: [],
  carry_items: [{ label: "The whole conversation", hint: "Every message from the start.", how: "moves" }],
  move_line: "The whole conversation is carried over — all 12 messages. The original stays as it was.",
  secrets: { asks: false, reader: "Claude Code (Anthropic)" },
  target_tool: "claude-code",
};

type Seams = {
  __AIJ_SWITCH_PREFLIGHT__?: PreflightInfoData;
  __AIJ_SWITCH_SETUP__?: typeof EXAMPLE_SETUP;
  __AIJ_OPENED__?: Array<{ kind: string; id: string }>;
};

beforeEach(() => {
  (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__ = PREFLIGHT;
  (window as never as Seams).__AIJ_SWITCH_SETUP__ = EXAMPLE_SETUP_NOTES;
});
afterEach(() => {
  cleanup();
  delete (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__;
  delete (window as never as Seams).__AIJ_SWITCH_SETUP__;
  delete (window as never as Seams).__AIJ_OPENED__;
});

function mount() {
  return render(
    <SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" onClose={() => {}} onSwitched={vi.fn()} />,
  );
}

describe("this project's notes", () => {
  it("asks once, unticked, and says what is kept as it is", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    const box = view.getByRole("checkbox", {
      name: "Work Max's notes for this project (3) aren't on Personal: copy them?",
    }) as HTMLInputElement;
    expect(box.checked).toBe(false);
    const row = box.closest("li") as HTMLElement;
    expect(within(row).getByTestId("switch-notes-kept").textContent).toBe(
      "Personal has its own project-codename.md with other words: it stays as it is, and Work Max's isn't copied.",
    );
    expect(within(row).getByTestId("switch-setup-consequence").textContent).toBe(
      "Copied as new files when you switch; nothing Personal has is replaced. You can undo it after.",
    );
    // What does this mean: the consequence, an example, and its guide section.
    const meaning = within(row).getByTestId("setup-meaning");
    expect(meaning.textContent).toContain("A note Personal has with other words is kept as it is.");
    expect(meaning.textContent).toContain("For example: a note that the project's codename is Bluebird is copied");
    fireEvent.click(within(meaning).getByRole("button", { name: /switching guide/ }));
    expect((window as never as Seams).__AIJ_OPENED__).toEqual([{ kind: "guide", id: "switching#this-projects-notes" }]);
  });

  it("isn't counted as something this conversation used", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    expect(view.getByTestId("switch-setup-summary").textContent).toBe(
      "Personal doesn't have 1 thing this conversation used. 1 will be set up.",
    );
    const notesOnly = { ...EXAMPLE_SETUP_NOTES, offers: EXAMPLE_SETUP_NOTES.offers.slice(1) };
    expect(setupSummary(notesOnly, new Set())).toBe("Personal has everything else Work Max has set up.");
  });

  it("says one note in the singular", () => {
    expect(notesQuestion(1, "Work Max", "Personal")).toBe("Work Max's note for this project (1) isn't on Personal: copy it?");
  });

  it("copies them only when ticked, and can undo it", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    fireEvent.click(view.getByRole("checkbox", { name: /notes for this project/ }));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-setup-results")).toBeTruthy());
    const results = view.getByTestId("switch-setup-results");
    expect(results.textContent).toContain("Copied the 3 notes for this project from Work Max");
    fireEvent.click(view.getByRole("button", { name: "Undo: Copied the 3 notes for this project from Work Max" }));
    await waitFor(() => expect(results.textContent).toContain("Removed the 3 notes for this project from Work Max"));
  });

  it("isn't copied when left unticked", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-setup")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-setup-results")).toBeTruthy());
    expect(view.getByTestId("switch-setup-results").textContent).not.toContain("for this project");
  });
});

describe("What's carried", () => {
  it("lists the notes the other account lacks as not there yet", () => {
    const view = render(<CarryList items={PREFLIGHT.carry_items} target="Personal" tool="Claude Code" setup={EXAMPLE_SETUP_NOTES} />);
    const missing = view.getByTestId("switch-carry-not-there");
    expect(missing.textContent).toContain("This project's notes");
    expect(missing.textContent).toContain("3 of Work Max's notes for this project aren't on Personal yet");
  });

  it("says plainly that the notes stay when the other tool keeps none", () => {
    const view = render(
      <CarryList items={PREFLIGHT.carry_items} target="Codex" tool="Codex" setup={{ ...EXAMPLE_SETUP, offers: [], notes_stay: 2 }} />,
    );
    const stays = view.getByTestId("switch-carry-stays");
    expect(stays.textContent).toContain("This project's notes");
    expect(stays.textContent).toContain("Work Max's 2 notes for this project stay with Work Max: Codex doesn't keep notes like these.");
  });
});
