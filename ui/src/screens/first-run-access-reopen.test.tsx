/**
 * The 0.0.12 report: Screen Recording allowed in System Settings, the
 * "See a live preview" card never turned to Allowed — macOS answers once
 * per process. The card checks again when the window is back, offers
 * Reopen AI Juggler (live sessions said first), and says Allowed only
 * when the OS does. First run comes back at this step after the reopen.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FirstRun } from "./FirstRun";
import { RESUME_ACCESS_KEY, takeResumeAccess } from "../lib/osAccess";
import type { OsAccessInfo } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;

const REOPEN_NOTE = "Allowed it in System Settings? macOS needs AI Juggler to reopen to see it.";

function preview(over: Partial<OsAccessInfo>): OsAccessInfo {
  return {
    id: "preview",
    title: "See a live preview of each session",
    why: "Shows a session's window inside AI Juggler, on this Mac only; nothing is recorded or sent.",
    system_name: "Screen Recording",
    status: "not_allowed",
    action: "reopen",
    note: REOPEN_NOTE,
    ...over,
  };
}

const calls = () => (w().__AIJ_OS_ACCESS_CALLS__ as string[] | undefined) ?? [];

function renderAccess() {
  return render(<FirstRun mode="first-run" resumeAtAccess onDone={() => null} openLoginTerminal={() => null} />);
}

beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
  w().__AIJ_SCAN_REPORT__ = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false };
  w().__AIJ_OS_ACCESS__ = [preview({})];
  localStorage.clear();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe("Screen Recording after System Settings (0.0.12)", () => {
  it("asked and still not allowed: the card says why, offers the reopen, and never claims Allowed", async () => {
    const view = renderAccess();
    const card = await view.findByTestId("first-run-access-preview");
    expect(card.textContent).toContain(REOPEN_NOTE);
    expect(within(card).getByRole("button", { name: "Reopen AI Juggler" })).toBeTruthy();
    expect(within(card).getByRole("button", { name: /Open System Settings/ })).toBeTruthy();
    expect(card.textContent).not.toContain("Allowed ");
    expect(within(card).queryByRole("status")).toBeNull();
  });

  it("back from System Settings: the window's focus checks again at once", async () => {
    // The 1.5 s poll can never fire (its interval is faked and never
    // advanced), so whatever turns the card to Allowed is the focus, on a
    // machine of any speed. Timeouts and promises stay real.
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    try {
      const view = renderAccess();
      await view.findByTestId("first-run-access-reopen");
      // The OS now says yes (after a reopen, or a change it reports live).
      w().__AIJ_OS_ACCESS__ = [preview({ status: "allowed", action: "none", note: null })];
      // The window comes back until the card turns: the rows' listener is
      // attached after their first paint, which a slow runner can reach
      // after the button shows (main's CI, 2026-10-23). A plain loop on
      // the real setTimeout: waitFor polls with setInterval, faked here,
      // so it would only retry on a DOM change that never comes (#178).
      const allowed = () => view.queryByRole("status")?.textContent?.includes("Allowed") ?? false;
      for (let tries = 0; tries < 60 && !allowed(); tries += 1) {
        act(() => {
          window.dispatchEvent(new Event("focus"));
        });
        await act(() => new Promise((resolve) => setTimeout(resolve, 100)));
      }
      expect(view.getByRole("status").textContent).toContain("Allowed");
      expect(view.queryByTestId("first-run-access-reopen")).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  }, 10_000);

  it("Reopen with nothing live relaunches, and first run will come back at this step", async () => {
    const view = renderAccess();
    fireEvent.click(await view.findByTestId("first-run-access-reopen"));
    await waitFor(() => expect(calls()).toContain("reopen:ask"));
    expect(localStorage.getItem(RESUME_ACCESS_KEY)).toBe("1");
    expect(view.queryByTestId("first-run-access-reopen-block")).toBeNull();
  });

  it("live sessions are said first; Reopen anyway goes ahead, Not now doesn't", async () => {
    w().__AIJ_REOPEN_BLOCK__ = {
      reason: "2 sessions are running. Reopening AI Juggler ends the ones in its terminal; sessions in their own app keep running.",
      can_reopen_anyway: true,
    };
    const view = renderAccess();
    fireEvent.click(await view.findByTestId("first-run-access-reopen"));
    const block = await view.findByTestId("first-run-access-reopen-block");
    expect(block.textContent).toContain("2 sessions are running.");
    expect(localStorage.getItem(RESUME_ACCESS_KEY)).toBeNull();
    fireEvent.click(within(block).getByRole("button", { name: "Not now" }));
    expect(view.queryByTestId("first-run-access-reopen-block")).toBeNull();
    expect(calls()).toEqual(["reopen:ask"]);

    fireEvent.click(view.getByTestId("first-run-access-reopen"));
    const again = await view.findByTestId("first-run-access-reopen-block");
    fireEvent.click(within(again).getByRole("button", { name: "Reopen anyway" }));
    await waitFor(() => expect(calls()).toContain("reopen:anyway"));
    expect(localStorage.getItem(RESUME_ACCESS_KEY)).toBe("1");
  });

  it("a switch under way makes it wait: no Reopen anyway", async () => {
    w().__AIJ_REOPEN_BLOCK__ = {
      reason: "A switch is in progress. Reopen AI Juggler once it finishes.",
      can_reopen_anyway: false,
    };
    const view = renderAccess();
    fireEvent.click(await view.findByTestId("first-run-access-reopen"));
    const block = await view.findByTestId("first-run-access-reopen-block");
    expect(block.textContent).toContain("A switch is in progress.");
    expect(within(block).queryByRole("button", { name: "Reopen anyway" })).toBeNull();
    expect(localStorage.getItem(RESUME_ACCESS_KEY)).toBeNull();
  });

  it("the resume marker is read once", () => {
    localStorage.setItem(RESUME_ACCESS_KEY, "1");
    expect(takeResumeAccess()).toBe(true);
    expect(takeResumeAccess()).toBe(false);
  });
});
