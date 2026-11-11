/**
 * The rc33 R3 walk: a Grok terminal session (no driven runtime, so its
 * ended header is Relaunch · Resume · ⋯) came back on the same
 * conversation 6 s after its Stop. Nothing in the backend restarts a
 * stopped session (the restarter's verdict is "you stopped it", the
 * pickup runs only at start); the only path is Resume, which takes Stop's
 * very place once the held stop lands, while for its 5 s the button still
 * read "Stop" on a working session. A press meant for Stop never resumes.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { resetToasts, ToastHost } from "../../components/Toast";
import { InspectorStop } from "../../shell/pages";
import { holdStop, RESUME_SETTLE_MS, STOP_HOLD_SECONDS } from "../../lib/stop";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { clearSeams, instance, seamCalls } from "../../test/fixtures";
import { ResumeButton } from "./ResumeButton";


afterEach(() => {
  cleanup();
  resetToasts();
  vi.useRealTimers();
  clearSeams("__AIJ_SESSION_RESUME__", "__AIJ_TERMINAL_KILL_CALLS__");
});

const grok = (status: string): InstanceInfo => instance({ id: "sess-grok", provider: "grok-build", kind: "terminal", status, conversation_id: "conv-g", account_id: "acc-r3", attention: null });

describe("rc33: a user's Stop is never undone", () => {
  it("Stop says it's stopping while held, and the Resume in its place doesn't take the next press", async () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    const stop = (session: InstanceInfo) => holdStop({ id: session.id, label: "r3-tools · Grok · R3" });
    const running = render(<InspectorStop session={grok("running")} onStop={stop} />);
    fireEvent.click(screen.getByTestId("inspector-stop"));
    const held = screen.getByTestId("inspector-stop");
    expect(held.textContent).toBe("Stopping…");
    expect(held.hasAttribute("disabled")).toBe(true);
    await act(async () => {
      vi.advanceTimersByTime(STOP_HOLD_SECONDS * 1000);
    });
    expect(seamCalls("__AIJ_TERMINAL_KILL_CALLS__")).toEqual(["sess-grok"]);
    running.unmount();

    // Ended: Resume takes the place; the press meant for Stop lands on it.
    render(<ResumeButton session={grok("stopped")} openIn={null} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("header-resume"));
    });
    expect(seamCalls("__AIJ_SESSION_RESUME__")).toEqual([]);

    // A deliberate Resume after the settle still resumes.
    await act(async () => {
      vi.advanceTimersByTime(RESUME_SETTLE_MS);
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("header-resume"));
    });
    expect(seamCalls("__AIJ_SESSION_RESUME__")).toEqual(["sess-grok"]);
  });
});
