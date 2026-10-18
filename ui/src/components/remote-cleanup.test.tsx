/**
 * AJ-E707 vitest — paired machines in the Cleanup Centre: verification
 * shows before any action; zero Clean buttons on rows the machine did
 * not verify; Clean needs View + control and a confirmation; the
 * outcome is the owning machine's words.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { RemoteCleanup } from "./RemoteCleanup";

const w = () => window as unknown as Record<string, unknown>;
const NOW = Date.now();
const proc = (pid: number, verified: boolean) => ({
  instance_id: "inst-1", provider: "claude-code", account_label: "Work Max", pid, role: "helper",
  state: verified ? "orphaned" : "unverifiable",
  state_label: verified ? "Still running after its Instance closed" : "Unverifiable — will not be touched",
  verified, name: "node", age_ms: 60_000, rss_bytes: null, cpu_percent: null, ports: verified ? "3000" : "", measured_at_ms: NOW,
});
const machine = (permission: string) => ({
  machine_id: "machine-mini", label: "Mac mini", connection: "connected", freshness: "live",
  last_heard_ms: NOW, permission, resources: null, instances: [], alerts: [],
  processes: [proc(4242, true), proc(4243, false)],
});

afterEach(cleanup);
beforeEach(() => {
  delete w().__AIJ_REMOTE_CLEAN__;
  delete w().__AIJ_FLEET_CALLS__;
});

describe("RemoteCleanup", () => {
  it("never offers Clean on a row its machine didn't verify", async () => {
    w().__AIJ_FLEET_REMOTE__ = [machine("control")];
    const view = render(<RemoteCleanup />);
    await view.findByTestId("remote-cleanup");
    const rows = view.getAllByTestId("remote-cleanup-row");
    expect(rows).toHaveLength(2);
    const unverified = rows.find((row) => row.dataset.verified === "false")!;
    expect(unverified.textContent).toContain("Not owned — never touched");
    expect(unverified.querySelector('[data-testid="remote-cleanup-clean"]')).toBeNull();
    expect(view.getAllByTestId("remote-cleanup-clean")).toHaveLength(1);
  });

  it("Clean confirms first and shows the machine's outcome", async () => {
    w().__AIJ_FLEET_REMOTE__ = [machine("control")];
    w().__AIJ_REMOTE_CLEAN__ = { accepted: true, reason_code: null, message: "Needed SIGKILL (pid 4242)" };
    const view = render(<RemoteCleanup />);
    fireEvent.click(await view.findByTestId("remote-cleanup-clean"));
    const sheet = view.getByTestId("remote-cleanup-confirm");
    expect(sheet.textContent).toContain("checks again that pid 4242");
    expect(document.activeElement?.textContent).toBe("Cancel");
    await act(async () => {
      fireEvent.click(view.getByTestId("remote-cleanup-yes"));
    });
    await waitFor(() =>
      expect(view.getByTestId("remote-cleanup-outcome").textContent).toBe("Mac mini: Needed SIGKILL (pid 4242)"),
    );
    expect(w().__AIJ_FLEET_CALLS__).toEqual(["remote-clean:machine-mini:inst-1:4242"]);
  });

  it("View only: verified rows show, Clean is disabled with the reason", async () => {
    w().__AIJ_FLEET_REMOTE__ = [machine("view")];
    const view = render(<RemoteCleanup />);
    expect((await view.findByTestId("remote-cleanup-readonly")).textContent).toContain(
      "cleaning needs View + control on Mac mini",
    );
    expect((view.getByTestId("remote-cleanup-clean") as HTMLButtonElement).disabled).toBe(true);
  });
});
