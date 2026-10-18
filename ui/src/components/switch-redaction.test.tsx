import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import type { PreflightInfoData } from "../lib/bridge";

/**
 * AJ-E208 guardrails: the policy is visible and the user's will is
 * carried — the first-run chooser has NO pre-selected default and
 * blocks execution until chosen; the confirm line is verbatim
 * (global vs override); the per-switch Ask answer and per-item
 * un-redact indices ride the execute call and nothing else does.
 */

const BASE: PreflightInfoData = {
  mechanism: "transplant",
  fidelity: "F1",
  score: 0.85,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: true,
  experimental_copy: "Experimental copy.",
  door: "one-click",
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: {
    carried: ["chat transcript"],
    regenerated: [],
    dropped: [],
  },
  redaction: {
    global: "strip",
    override: null,
    effective: "strip",
    source: "global",
    line: "Redaction for claude-code: Strip — replace keys with ••• (your global choice)",
  },
  redaction_items: [],
};

type Seams = {
  __AIJ_SWITCH_PREFLIGHT__?: PreflightInfoData;
  __AIJ_SWITCH_EXECUTE_HANG__?: boolean;
  __AIJ_SWITCH_EXECUTE_REDACTION__?: {
    redactionChoice: string | null | undefined;
    redactionUnredact: number[];
  }[];
  __AIJ_REDACTION_SET_GLOBAL__?: (string | null)[];
};

beforeEach(() => {
  delete (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__;
  delete (window as never as Seams).__AIJ_SWITCH_EXECUTE_HANG__;
  delete (window as never as Seams).__AIJ_SWITCH_EXECUTE_REDACTION__;
  delete (window as never as Seams).__AIJ_REDACTION_SET_GLOBAL__;
});
afterEach(cleanup);

function mount(preflight: PreflightInfoData) {
  (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__ = preflight;
  return render(
    <SwitchFlow
      sessionId="sess-1"
      targetProfileId="p-1"
      targetLabel="Personal"
      onClose={() => {}}
      onSwitched={() => {}}
    />,
  );
}

describe("AJ-E208 redaction policy", () => {
  it("shows the policy line verbatim (global choice)", async () => {
    const view = mount(BASE);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    expect(view.getByTestId("switch-redaction-line").textContent).toBe(
      "Redaction for claude-code: Strip — replace keys with ••• (your global choice)",
    );
    expect(view.getByTestId("switch-redaction-none")).toBeTruthy();
  });

  it("names the override variant", async () => {
    const view = mount({
      ...BASE,
      redaction: {
        global: "keep",
        override: "strip",
        effective: "strip",
        source: "override",
        line: "Redaction for claude-code: Strip — replace keys with ••• (override)",
      },
    });
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    expect(view.getByTestId("switch-redaction-line").textContent).toContain("(override)");
  });

  it("first run: no default selected, blocks execution until the user chooses", async () => {
    const view = mount({
      ...BASE,
      redaction: { global: null, override: null, effective: null, source: "unset", line: "" },
    });
    await waitFor(() => expect(view.getByTestId("switch-redaction-chooser")).toBeTruthy());
    const save = view.getByTestId("switch-redaction-save") as HTMLButtonElement;
    expect(save.disabled).toBe(true); // no pick yet — nothing is guessed
    const confirm = view.getByTestId("switch-confirm") as HTMLButtonElement;
    expect(confirm.disabled).toBe(true); // the first pack is blocked
    // Every radio starts UNCHECKED (no dark pattern).
    const radios = view
      .getAllByRole("radio")
      .filter((radio) => (radio as HTMLInputElement).name === "redaction-global");
    expect(radiansUnchecked(radios)).toBe(true);

    fireEvent.click(view.getByLabelText("Strip — replace keys with •••"));
    await waitFor(() => expect(save.disabled).toBe(false));
    fireEvent.click(save);
    await waitFor(() =>
      expect((window as never as Seams).__AIJ_REDACTION_SET_GLOBAL__).toEqual(["strip"]),
    );
  });

  it("Ask: requires the per-switch answer; the answer + un-redact indices ride the click", async () => {
    const view = mount({
      ...BASE,
      door: "wizard",
      wizard_steps: ["Target", "What's carried", "Confirm"],
      redaction: {
        global: "ask",
        override: null,
        effective: "ask",
        source: "global",
        line: "Redaction for claude-code: Ask each time (your global choice)",
      },
      redaction_items: [
        { kind: "sk-key", preview: "s•••5678", segment: 0 },
        { kind: "bearer", preview: "c•••aaaa", segment: 1 },
      ],
    });
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    // The per-item un-redact list lives on the preview step.
    fireEvent.click(view.getByTestId("switch-continue"));
    await waitFor(() => expect(view.getByTestId("switch-redaction-items")).toBeTruthy());
    fireEvent.click(view.getByText("sk-key"));
    // Ask answer lives on the Confirm step — and blocks until chosen.
    fireEvent.click(view.getByTestId("switch-continue"));
    await waitFor(() => expect(view.getByTestId("switch-step-confirm")).toBeTruthy());
    const confirm = view.getByTestId("switch-confirm") as HTMLButtonElement;
    expect(confirm.disabled).toBe(true); // Ask each time requires the answer
    fireEvent.click(view.getByLabelText("Strip secrets this time"));
    await waitFor(() => expect(confirm.disabled).toBe(false));
    fireEvent.click(confirm);
    await waitFor(() =>
      expect((window as never as Seams).__AIJ_SWITCH_EXECUTE_REDACTION__).toHaveLength(1),
    );
    const payload = (window as never as Seams).__AIJ_SWITCH_EXECUTE_REDACTION__![0];
    expect(payload.redactionChoice).toBe("strip");
    expect(payload.redactionUnredact).toEqual([0]); // the un-redacted item rides the click
  });
});

function radiansUnchecked(radios: HTMLElement[]): boolean {
  return radios.every((radio) => !(radio as HTMLInputElement).checked);
}
