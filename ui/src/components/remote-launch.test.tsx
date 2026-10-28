/**
 * AJ-E1515 vitest — New session on a paired Mac: the picker only with
 * paired Macs; that Mac's projects and launch settings; a Mac that can't
 * start one says why and Start is off; Start sends ids only and closes,
 * or shows that Mac's words when it refuses. Usage by machine: totals in
 * words, "Hasn't reported yet" before a first report.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MachinePicker, RemoteSessionSheet, useWhere } from "./RemoteLaunch";
import { machineUsageWords } from "./MachineUsage";
import type { LaunchMachineInfo } from "../lib/ipc/fleet-launch";

const w = () => window as unknown as Record<string, unknown>;
afterEach(cleanup);
beforeEach(() => {
  for (const key of ["__AIJ_LAUNCH_MACHINES__", "__AIJ_REMOTE_STARTED__", "__AIJ_REMOTE_OUTCOME__"]) delete w()[key];
});

const STUDIO: LaunchMachineInfo = {
  machine_id: "m-studio",
  label: "Studio Mac",
  why_not: null,
  recipes: [{ id: "r-1", name: "Main", tool: "Claude Code", account: "Desktop Max" }],
  projects: [{ id: "p-1", name: "storefront" }],
};
const AIR: LaunchMachineInfo = { machine_id: "m-air", label: "MacBook Air", why_not: "MacBook Air isn't connected right now", recipes: [], projects: [] };

function Sheet({ onClose }: { onClose: () => void }) {
  const where = useWhere();
  return where.machine ? <RemoteSessionSheet where={where} onClose={onClose} /> : <MachinePicker where={where} />;
}

describe("AJ-E1515 — New session on a paired Mac", () => {
  it("offers no picker without paired Macs", async () => {
    const { container } = render(<Sheet onClose={() => {}} />);
    await act(async () => {});
    expect(container.innerHTML).toBe("");
  });

  it("shows that Mac's projects and launch settings, and starts there with ids only", async () => {
    w().__AIJ_LAUNCH_MACHINES__ = [STUDIO, AIR];
    const onClose = vi.fn();
    const { findByRole, getByTestId } = render(<Sheet onClose={onClose} />);
    fireEvent.click(await findByRole("radio", { name: "Studio Mac" }));
    expect(getByTestId("remote-projects").textContent).toContain("storefront");
    expect(getByTestId("remote-recipes").textContent).toContain("Desktop Max");
    expect(getByTestId("remote-recipes").textContent).toContain("Claude Code · Launch settings: Main");
    await act(async () => {
      fireEvent.click(getByTestId("remote-start"));
    });
    expect(w().__AIJ_REMOTE_STARTED__).toEqual({ machineId: "m-studio", recipeId: "r-1", projectId: "p-1" });
    expect(onClose).toHaveBeenCalled();
  });

  it("says why a Mac can't start one, with Start off", async () => {
    w().__AIJ_LAUNCH_MACHINES__ = [STUDIO, AIR];
    const { findByRole, getByTestId } = render(<Sheet onClose={() => {}} />);
    fireEvent.click(await findByRole("radio", { name: "MacBook Air" }));
    expect(getByTestId("remote-why-not").textContent).toBe("MacBook Air isn't connected right now.");
    expect((getByTestId("remote-start") as HTMLButtonElement).disabled).toBe(true);
  });

  it("shows that Mac's own words when it refuses", async () => {
    w().__AIJ_LAUNCH_MACHINES__ = [STUDIO];
    w().__AIJ_REMOTE_OUTCOME__ = { accepted: false, reason_code: "not_confirmed", message: "Not allowed on that computer — nothing ran." };
    const onClose = vi.fn();
    const { findByRole, getByTestId } = render(<Sheet onClose={onClose} />);
    fireEvent.click(await findByRole("radio", { name: "Studio Mac" }));
    await act(async () => {
      fireEvent.click(getByTestId("remote-start"));
    });
    await waitFor(() => expect(getByTestId("remote-error").textContent).toBe("Not allowed on that computer — nothing ran."));
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe("AJ-E1515 — usage by machine", () => {
  it("says each machine's last day in words", () => {
    const row = { machine_id: "m", label: "Studio Mac", used_units: 1_234_567, usd_est: 4.1, sessions: 3, as_of_ms: 1, reported: true };
    expect(machineUsageWords(row)).toBe("1.2M tokens · ≈ $4.10 · 3 sessions");
    expect(machineUsageWords({ ...row, usd_est: null, sessions: 1, used_units: 900 })).toBe("900 tokens · 1 session");
    expect(machineUsageWords({ ...row, used_units: 0 })).toBe("Nothing used");
    expect(machineUsageWords({ ...row, reported: false })).toBe("Hasn't reported yet");
  });
});
