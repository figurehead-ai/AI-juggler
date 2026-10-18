/**
 * AJ-E706 / AJ-E708 vitest — cross-machine switching in the UI:
 * destinations name their machine, badge and readiness in words; the
 * wizard lists preconditions before anything runs, needs a confirmed
 * mapping for another path, offers T3 (never silently) when T2 is
 * blocked, asks about secrets for T3, and sends nothing before the final
 * "Switch to <machine>…" click; switch-back previews its one delta turn,
 * and a divergence opens the conflict sheet with "Review later" focused
 * and three described choices.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { RemoteSwitchTargets } from "./RemoteSwitchTargets";
import { CrossMachineSwitchFlow } from "./CrossMachineSwitchFlow";
import { CrossMachineSwitchBack } from "./CrossMachineSwitchBack";
import type { FleetSwitchBackPlanInfo, RemoteSwitchTargetInfo } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_FLEET_CALLS__ as string[] | undefined) ?? [];

const target = (over: Partial<RemoteSwitchTargetInfo> = {}): RemoteSwitchTargetInfo => ({
  machine_id: "machine-studio",
  machine_label: "Studio Mac",
  recipe_id: "rec-desk",
  recipe_name: "Main",
  account_label: "Desktop Max",
  provider: "claude-code",
  mechanism: "transplant",
  readiness: "t2_ready",
  blockers: [],
  fidelity: "F3",
  badge: "F3 · Unvalidated",
  validated: false,
  carried: ["Conversation", "Rules"],
  regenerated: [],
  dropped: ["MCP servers"],
  same_path_project: "proj-1",
  projects: [{ project_id: "proj-1", name: "storefront", same_path: true, same_name: true }],
  replay_available: true,
  version_here: "2.1.0",
  version_there: "2.1.0",
  ...over,
});

afterEach(cleanup);
beforeEach(() => {
  delete w().__AIJ_FLEET_CALLS__;
  delete w().__AIJ_FLEET_SWITCH_OUTCOME__;
  delete w().__AIJ_FLEET_SWITCH_BACK_OUTCOMES__;
  delete w().__AIJ_FLEET_DIVERGENCES__;
});

describe("AJ-E706 destinations", () => {
  it("names the machine, the badge and the readiness in words; unavailable rows are disabled", () => {
    const { container } = render(
      <RemoteSwitchTargets
        targets={[
          target(),
          target({ recipe_id: "rec-2", readiness: "t2_blocked", mechanism: "replay" }),
          target({ recipe_id: "rec-3", readiness: "unavailable", blockers: ["Studio Mac lets this Mac view only — switching needs View + control (change it on Studio Mac)."] }),
        ]}
        onPick={() => {}}
      />,
    );
    const rows = container.querySelectorAll<HTMLButtonElement>("[data-testid='remote-switch-target']");
    expect(rows).toHaveLength(3);
    expect(container.textContent).toContain("On Studio Mac");
    expect(rows[0].textContent).toContain("F3 · Unvalidated");
    expect(rows[0].textContent).toContain("Ready — same version, same project path");
    expect(rows[1].textContent).toContain("full-story replay offered");
    expect(rows[2].disabled).toBe(true);
    expect(rows[2].textContent).toContain("view only");
  });
});

describe("AJ-E706 wizard", () => {
  it("T2 at the same path: Target → What's carried → Confirm; nothing is sent before the click", async () => {
    const { container } = render(<CrossMachineSwitchFlow sessionId="s1" target={target()} onClose={() => {}} />);
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    expect(q("cross-switch-preconditions")!.textContent).toContain("2.1.0 here, 2.1.0 on Studio Mac");
    expect(q("cross-switch-project")).toBeNull();
    fireEvent.click(q("cross-switch-next")!);
    expect(q("cross-switch-drops")!.textContent).toContain("Will drop: MCP servers");
    fireEvent.click(q("cross-switch-next")!);
    expect(q("cross-switch-go")!.textContent).toBe("Switch to Studio Mac…");
    expect(calls()).toEqual([]);
    await act(async () => {
      fireEvent.click(q("cross-switch-go")!);
    });
    expect(calls()).toEqual(["switch:machine-studio:rec-desk:transplant:proj-1"]);
    await waitFor(() => expect(q("cross-switch-done")).not.toBeNull());
  });

  it("another path needs a chosen project AND a confirmed mapping before Next", async () => {
    const { container } = render(
      <CrossMachineSwitchFlow
        sessionId="s1"
        target={target({
          readiness: "needs_mapping",
          same_path_project: null,
          projects: [
            { project_id: "proj-x", name: "other", same_path: false, same_name: false },
            { project_id: "proj-2", name: "storefront", same_path: false, same_name: true },
          ],
        })}
        onClose={() => {}}
      />,
    );
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    fireEvent.click(q("cross-switch-next")!);
    expect(q("cross-switch-project")).not.toBeNull();
    const next = q("cross-switch-next") as HTMLButtonElement;
    expect(next.disabled).toBe(true);
    const options = container.querySelectorAll<HTMLInputElement>("[data-testid='cross-switch-project-option']");
    // The same-name copy is listed first — a suggestion, never pre-picked.
    expect(options[0].closest("label")!.textContent).toContain("same name (suggested)");
    expect(options[0].checked).toBe(false);
    fireEvent.click(options[0]);
    expect((q("cross-switch-next") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(q("cross-switch-confirm-mapping")!);
    expect((q("cross-switch-next") as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(q("cross-switch-next")!);
    fireEvent.click(q("cross-switch-next")!);
    await act(async () => {
      fireEvent.click(q("cross-switch-go")!);
    });
    expect(calls()).toEqual(["switch:machine-studio:rec-desk:transplant:proj-2"]);
  });

  it("blocked T2 offers T3 openly, and T3 asks how secrets are handled", async () => {
    const { container } = render(
      <CrossMachineSwitchFlow
        sessionId="s1"
        target={target({
          readiness: "t2_blocked",
          mechanism: "replay",
          blockers: ["claude-code 2.1.0 here, 2.0.9 on Studio Mac — T2 needs the same version."],
          version_there: "2.0.9",
        })}
        onClose={() => {}}
      />,
    );
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    expect((q("cross-switch-t2") as HTMLInputElement).disabled).toBe(true);
    expect((q("cross-switch-t3") as HTMLInputElement).checked).toBe(true);
    expect(q("cross-switch-blockers")!.textContent).toContain("T2 needs the same version");
    fireEvent.click(q("cross-switch-next")!);
    expect((q("cross-switch-next") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(q("cross-switch-secrets-strip")!);
    fireEvent.click(q("cross-switch-next")!);
    await act(async () => {
      fireEvent.click(q("cross-switch-go")!);
    });
    expect(calls()).toEqual(["switch:machine-studio:rec-desk:replay:proj-1"]);
  });
});

const plan: FleetSwitchBackPlanInfo = {
  original_machine_id: "machine-laptop",
  original_machine_label: "Laptop",
  original_account_label: "Work Max",
  away_account_label: "Desktop Max",
  new_turns: 2,
  delta_preview: "While you were away on Desktop Max — 2 new turns",
  blockers: [],
};

describe("AJ-E706 / AJ-E708 switch-back and the conflict sheet", () => {
  it("previews the one delta turn, then a divergence opens the sheet focused on Review later", async () => {
    w().__AIJ_FLEET_SWITCH_BACK_OUTCOMES__ = [
      {
        accepted: false,
        reason_code: "diverged",
        message: "The original moved on",
        instance_id: null,
        edge_id: "e",
        divergence: { turns_since: 1, last_activity_ms: 1_800_000_000_000, summary: "1 new turns" },
      },
    ];
    const { container } = render(<CrossMachineSwitchBack sessionId="away" plan={plan} onClose={() => {}} />);
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    expect(q("cross-back-delta")!.textContent).toContain("While you were away");
    await act(async () => {
      fireEvent.click(q("cross-back-go")!);
    });
    await waitFor(() => expect(q("conflict-choices")).not.toBeNull());
    expect(document.activeElement).toBe(q("conflict-later"));
    expect(q("conflict-theirs")!.textContent).toContain("1 new turn in the original");
    expect((q("conflict-apply") as HTMLButtonElement).disabled).toBe(true);
    expect(container.querySelectorAll("[role='radiogroup'] input[type='radio']")).toHaveLength(3);
    fireEvent.click(q("conflict-keep_both")!);
    await act(async () => {
      fireEvent.click(q("conflict-apply")!);
    });
    expect(calls()).toEqual(["switch-back:away:", "switch-back:away:keep_both"]);
    await waitFor(() => expect(q("cross-back-done")).not.toBeNull());
  });

  it("Keep theirs sends nothing and clears the divergence", async () => {
    w().__AIJ_FLEET_DIVERGENCES__ = [
      {
        session_id: "away",
        machine_id: "machine-laptop",
        machine_label: "Laptop",
        turns_since: 1,
        last_activity_ms: null,
        summary: "1 new turns",
        at_ms: 1,
      },
    ];
    const { container } = render(
      <CrossMachineSwitchBack
        sessionId="away"
        plan={plan}
        divergence={{ turns_since: 1, last_activity_ms: null, summary: "1 new turns" }}
        onClose={() => {}}
      />,
    );
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    fireEvent.click(q("conflict-keep_theirs")!);
    await act(async () => {
      fireEvent.click(q("conflict-apply")!);
    });
    expect(calls()).toEqual(["keep-theirs:away"]);
    expect(w().__AIJ_FLEET_DIVERGENCES__).toEqual([]);
    await waitFor(() => expect(q("cross-back-done")!.textContent).toContain("Nothing was sent"));
  });
});
