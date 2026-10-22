/**
 * The 0.0.12 report, as a pure state machine: Screen Recording asked, back
 * from System Settings without the OS reporting a grant → offer Reopen
 * (never "Allowed"); granted → Allowed. Plus the re-check when the window
 * is back, and the reopen's resume marker.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import type { OsAccessInfo } from "./bridge";
import { accessCard, reopenApp, RESUME_ACCESS_KEY, takeResumeAccess, watchAccess } from "./osAccess";

const NOTE = "Allowed it in System Settings? macOS needs AI Juggler to reopen to see it.";
const entry = (over: Partial<OsAccessInfo>): OsAccessInfo => ({
  id: "preview",
  title: "See a live preview of each session",
  why: "Shows a session's window inside AI Juggler, on this Mac only; nothing is recorded or sent.",
  system_name: "Screen Recording",
  status: "not_asked",
  action: "prompt",
  note: "macOS may ask you to reopen AI Juggler after you allow it.",
  ...over,
});

afterEach(() => {
  localStorage.clear();
  delete (window as unknown as Record<string, unknown>).__AIJ_REOPEN_BLOCK__;
});

describe("the access card's states", () => {
  it("not asked → Allow raises the system prompt", () => {
    const card = accessCard(entry({}));
    expect(card.state).toBe("not_asked");
    expect(card.primary).toEqual({ label: "Allow", action: "request" });
    expect(card.secondary).toBeNull();
  });

  it("asked, back without a grant → offer Reopen, System Settings beside it, never Allowed", () => {
    const card = accessCard(entry({ status: "not_allowed", action: "reopen", note: NOTE }));
    expect(card.state).toBe("reopen");
    expect(card.note).toBe(NOTE);
    expect(card.noteLeads).toBe(true);
    expect(card.primary).toEqual({ label: "Reopen AI Juggler", action: "reopen" });
    expect(card.secondary).toEqual({ label: "Open System Settings", action: "settings" });
  });

  it("granted → Allowed, and only then", () => {
    const card = accessCard(entry({ status: "allowed", action: "none", note: null }));
    expect(card.state).toBe("allowed");
    expect(card.primary).toBeNull();
    // A "reopen" action never turns into Allowed by itself.
    expect(accessCard(entry({ status: "not_allowed", action: "reopen" })).state).not.toBe("allowed");
  });

  it("a refusal the system won't ask about again → its settings pane", () => {
    const card = accessCard(entry({ status: "not_allowed", action: "settings", note: null }));
    expect(card.primary).toEqual({ label: "Open System Settings", action: "settings" });
  });
});

describe("the re-check and the reopen", () => {
  it("checks again when the window is back, not only on the timer", () => {
    vi.useFakeTimers();
    const check = vi.fn();
    const stop = watchAccess(check, 1500);
    window.dispatchEvent(new Event("focus"));
    expect(check).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(1500);
    expect(check).toHaveBeenCalledTimes(2);
    stop();
    window.dispatchEvent(new Event("focus"));
    vi.advanceTimersByTime(3000);
    expect(check).toHaveBeenCalledTimes(2);
    vi.useRealTimers();
  });

  it("marks the resume when it relaunches; clears it when it has to ask first", async () => {
    await expect(reopenApp(false)).resolves.toBeNull();
    expect(localStorage.getItem(RESUME_ACCESS_KEY)).toBe("1");
    expect(takeResumeAccess()).toBe(true);
    expect(takeResumeAccess()).toBe(false);
    (window as unknown as Record<string, unknown>).__AIJ_REOPEN_BLOCK__ = { reason: "2 sessions are running.", can_reopen_anyway: true };
    await expect(reopenApp(false)).resolves.toMatchObject({ can_reopen_anyway: true });
    expect(localStorage.getItem(RESUME_ACCESS_KEY)).toBeNull();
    await expect(reopenApp(true)).resolves.toBeNull();
    expect(localStorage.getItem(RESUME_ACCESS_KEY)).toBe("1");
  });
});
