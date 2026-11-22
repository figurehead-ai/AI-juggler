import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, waitFor } from "@testing-library/react";
import { KillSheet, type KillBridge } from "./KillSheet";
import { resetToasts, toastEntries } from "./Toast";

/**
 * R3 (the 0.5.0 re-walk, SK48-14): a stop of a leftover process went
 * through, then a modal said "Won't stop it: It has already ended. Nothing
 * was stopped." The Processes page hands the sheet a new bridge object on
 * every render (its live list and the recount re-render it), and the
 * sheet's check re-ran on each one, so after the stop it checked a process
 * that was gone. One press is one stop: one check, one stop, a success
 * toast, and no second dialog however often the page re-renders.
 */
const count = { verify: 0, execute: 0 };

function observed(verified: () => boolean): KillBridge {
  return {
    verify: (_session: string, pid: number) => {
      count.verify += 1;
      return Promise.resolve({
        pid,
        session: "",
        verified: verified(),
        refusal: verified() ? null : "It has already ended.",
        exe_path: "/bin/sleep",
        cmdline: "sleep",
        cwd: "/",
        ports: [],
        pgid: null,
        escaped_group: false,
        members: [],
        tree_pids: [],
        platform: "TERM → grace → KILL",
        grace_secs: 5,
        provider: "",
        account_id: null,
        project_id: null,
      });
    },
    execute: (_s: string, pid: number) => {
      count.execute += 1;
      return Promise.resolve({ refusal: null, members: [{ pid, outcome: "exited on TERM" }], cancelled: false, all_succeeded: true, still_holding: [], reoffer_pid: null });
    },
    cancel: vi.fn(() => Promise.resolve()),
    liveness: vi.fn(() => Promise.resolve([false])),
  };
}

describe("R3 one press is one stop", () => {
  it("a parent's re-render with a new bridge object neither re-checks nor says it won't stop", async () => {
    resetToasts();
    let alive = true;
    count.verify = 0;
    count.execute = 0;
    // Each render builds its bridge afresh, as ProcessesView did.
    const fresh = () => observed(() => alive);
    const onClose = vi.fn();
    const sheet = () => <KillSheet session="" pid={4242} bridge={fresh()} observed onClose={onClose} onlyThisProcess="Only this process." />;
    const view = render(sheet());
    await waitFor(() => expect(view.getByTestId("kill-confirm-4242")).toBeTruthy());
    view.rerender(sheet());
    view.rerender(sheet());
    fireEvent.click(view.getByTestId("kill-confirm-4242"));
    alive = false;
    // The live list and the recount re-render the page while it stops.
    view.rerender(sheet());
    await waitFor(() => expect(onClose).toHaveBeenCalledWith(true));
    view.rerender(sheet());
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(count.verify).toBe(1);
    expect(count.execute).toBe(1);
    expect(view.queryByTestId("kill-refused-4242")).toBeNull();
    expect(document.body.textContent ?? "").not.toContain("Won't stop it");
    const shown = toastEntries().map((entry) => entry.options);
    expect(shown).toHaveLength(1);
    expect(shown[0]).toMatchObject({ tone: "success", message: "Stopped sleep (PID 4242)." });
    view.unmount();
  });
});
