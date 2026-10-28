import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import type { PreflightInfoData } from "../lib/bridge";

/**
 * Keys and passwords in the conversation (D-14; the maintainer's walk of
 * 0.1.8, 2026-10-27): asked only when the preflight says the words go
 * somewhere new (a replay to another tool), never for a same-tool switch
 * on this Mac; what was found and who reads it said plainly; nothing
 * preselected; "Remember my choice" saves it, unticked it's this switch
 * only; nothing found never blocks.
 */

const TRANSPLANT: PreflightInfoData = {
  mechanism: "transplant",
  fidelity: "F3",
  fidelity_words: "Same conversation, new account",
  score: 0.85,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: false,
  source_keeps_running: false,
  experimental_copy: "Beta, not proven yet: the whole session moves.",
  door: "one-click",
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: false,
  carry: { carried: ["The whole conversation"], regenerated: [], dropped: [] },
  redaction: { global: null, override: null, effective: null, source: "unset", line: "" },
  redaction_items: [{ kind: "sk-key", preview: "s•••a91f", segment: 3 }],
  carry_items: [{ label: "The whole conversation", hint: "Every message from the start.", how: "moves" }],
  move_line: "The whole conversation moves — all 12 messages, its subagents and file history. The original stays as it was.",
  secrets: { asks: false, reader: "Claude Code (Anthropic)" },
  target_tool: "claude-code",
};

const REPLAY: PreflightInfoData = {
  ...TRANSPLANT,
  mechanism: "replay",
  fidelity: "F1",
  fidelity_words: "Full story, fresh start",
  door: "wizard",
  wizard_steps: ["Target", "What's carried", "Confirm"],
  redaction_items: [
    { kind: "sk-key", preview: "s•••a91f", segment: 3 },
    { kind: "bearer", preview: "B•••x2k9", segment: 7 },
  ],
  secrets: { asks: true, reader: "Codex (OpenAI)" },
  target_tool: "codex",
};

type Seams = {
  __AIJ_SWITCH_PREFLIGHT__?: PreflightInfoData;
  __AIJ_SWITCH_EXECUTE_REDACTION__?: { redactionChoice: string | null | undefined; redactionUnredact: number[] }[];
  __AIJ_REDACTION_SET_GLOBAL__?: (string | null)[];
};
const seams = () => window as never as Seams;

beforeEach(() => {
  delete seams().__AIJ_SWITCH_PREFLIGHT__;
  delete seams().__AIJ_SWITCH_EXECUTE_REDACTION__;
  delete seams().__AIJ_REDACTION_SET_GLOBAL__;
});
afterEach(cleanup);

function mount(preflight: PreflightInfoData) {
  seams().__AIJ_SWITCH_PREFLIGHT__ = preflight;
  return render(<SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" onClose={() => {}} onSwitched={() => {}} />);
}

async function toCarried(view: ReturnType<typeof mount>) {
  await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
  fireEvent.click(view.getByTestId("switch-continue"));
  await waitFor(() => expect(view.getByTestId("switch-step-carried")).toBeTruthy());
}

describe("keys and passwords", () => {
  it("never asks for a same-tool switch on this Mac, and never blocks it", async () => {
    const view = mount(TRANSPLANT);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    expect(view.queryByTestId("switch-keys")).toBeNull();
    expect((view.getByTestId("switch-confirm") as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(seams().__AIJ_SWITCH_EXECUTE_REDACTION__?.length).toBe(1));
    expect(seams().__AIJ_SWITCH_EXECUTE_REDACTION__![0]!.redactionChoice).toBeNull();
  });

  it("asks for a replay: says what was found and who reads it, nothing preselected", async () => {
    const view = mount(REPLAY);
    await toCarried(view);
    const question = view.getByTestId("switch-keys");
    expect(view.getByTestId("switch-keys-found").textContent).toBe(
      "We found 2: an API key (s•••a91f) and a sign-in token (B•••x2k9). Codex (OpenAI) will read this conversation to pick up the work.",
    );
    expect(question.textContent).toContain("Codex sees OPENAI_API_KEY=•••");
    expect(question.textContent).toContain("Codex and OpenAI see the keys exactly as written");
    expect(question.textContent).not.toMatch(/\bpacks?\b|redact/i);
    expect((view.getByTestId("switch-keys-hide") as HTMLInputElement).checked).toBe(false);
    expect((view.getByTestId("switch-keys-leave") as HTMLInputElement).checked).toBe(false);
    // Nothing moves on until it's answered.
    expect((view.getByTestId("switch-continue") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(view.getByTestId("switch-keys-hide"));
    expect((view.getByTestId("switch-continue") as HTMLButtonElement).disabled).toBe(false);
  });

  it("this switch only: the answer and the ones shown anyway ride the click, nothing is saved", async () => {
    const view = mount(REPLAY);
    await toCarried(view);
    fireEvent.click(view.getByTestId("switch-keys-hide"));
    fireEvent.click(view.getByLabelText(/a sign-in token/));
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(seams().__AIJ_SWITCH_EXECUTE_REDACTION__?.length).toBe(1));
    expect(seams().__AIJ_SWITCH_EXECUTE_REDACTION__![0]).toEqual({ redactionChoice: "strip", redactionUnredact: [1] });
    expect(seams().__AIJ_REDACTION_SET_GLOBAL__).toBeUndefined();
  });

  it("Remember my choice saves it as the policy before the switch", async () => {
    const view = mount(REPLAY);
    await toCarried(view);
    fireEvent.click(view.getByTestId("switch-keys-leave"));
    fireEvent.click(view.getByTestId("switch-keys-remember"));
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(seams().__AIJ_REDACTION_SET_GLOBAL__).toEqual(["keep"]));
    await waitFor(() => expect(seams().__AIJ_SWITCH_EXECUTE_REDACTION__![0]!.redactionChoice).toBe("keep"));
  });

  it("nothing found: says so and doesn't block", async () => {
    const view = mount({ ...REPLAY, redaction_items: [] });
    await toCarried(view);
    expect(view.getByTestId("switch-keys-none").textContent).toContain("We found no keys or passwords");
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(seams().__AIJ_SWITCH_EXECUTE_REDACTION__![0]!.redactionChoice).toBe("strip"));
  });

  it("a remembered choice is one line, and can be changed for this switch", async () => {
    const view = mount({ ...REPLAY, redaction: { global: "strip", override: null, effective: "strip", source: "global", line: "" } });
    await toCarried(view);
    expect(view.getByTestId("switch-keys-remembered").textContent).toContain("hidden from Codex (OpenAI)");
    expect((view.getByTestId("switch-continue") as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(view.getByTestId("switch-keys-change"));
    expect(view.getByTestId("switch-keys")).toBeTruthy();
    expect((view.getByTestId("switch-continue") as HTMLButtonElement).disabled).toBe(true);
  });
});
