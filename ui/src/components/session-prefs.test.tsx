import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { SessionsSettingsCard } from "./SessionsSettings";
import { applyTerminalPrefs } from "./TerminalView";
import { SESSION_PREFS_DEFAULT, SESSION_PREFS_EVENT, sessionPrefsSet, type SessionPrefsInfo } from "../lib/ipc/session-prefs";

/**
 * AJ-E1505 — Settings › Sessions: "Stuck after", the terminal's font size
 * and scrollback, each explained in one line, saved on Enter or leaving
 * the field, with a Reset once it differs; a saved change reaches open
 * terminals at once (the backend side: tests/session_prefs.rs and the
 * heuristics' live-threshold test).
 */

type Seams = { __AIJ_SESSION_PREFS__?: SessionPrefsInfo };
afterEach(() => {
  cleanup();
  delete (window as unknown as Seams).__AIJ_SESSION_PREFS__;
});

const stored = () => (window as unknown as Seams).__AIJ_SESSION_PREFS__;

/** Renders the card inside `act`, so its settings have loaded and each
 * field's first effect (which sets its draft from the saved value) has
 * run before the test types. Found by the input alone, a field could
 * still settle after the typing and put the saved value back: on a busy
 * CI runner the range alert then never came (#154). */
async function renderLoaded(): Promise<void> {
  await act(async () => {
    render(<SessionsSettingsCard />);
  });
}

describe("AJ-E1505 Settings › Sessions", () => {
  it("shows each setting with its one-line why; the terminal ones wait behind Terminal", async () => {
    render(<SessionsSettingsCard />);
    const stuck = await screen.findByLabelText("Stuck after");
    expect((stuck as HTMLInputElement).value).toBe("5");
    expect(stuck.getAttribute("aria-describedby")).toBeTruthy();
    expect(document.getElementById(stuck.getAttribute("aria-describedby")!)!.textContent).toContain("Looks stuck");
    const terminal = screen.getByTestId("settings-terminal") as HTMLDetailsElement;
    expect(terminal.open).toBe(false);
    expect((screen.getByLabelText("Font size") as HTMLInputElement).value).toBe("12.5");
    expect((screen.getByLabelText("Scrollback") as HTMLInputElement).value).toBe("5000");
    // At the defaults, no Reset.
    expect(screen.queryByText(/^Reset to/)).toBeNull();
  });

  it("saves on Enter, never mid-typing, and offers Reset until it's back at the default", async () => {
    await renderLoaded();
    const stuck = screen.getByLabelText("Stuck after") as HTMLInputElement;
    fireEvent.change(stuck, { target: { value: "1" } });
    expect(stored()).toBeUndefined();
    fireEvent.change(stuck, { target: { value: "10" } });
    await act(async () => {
      fireEvent.keyDown(stuck, { key: "Enter" });
    });
    await waitFor(() => expect(stored()?.stuck_after_minutes).toBe(10));
    expect(screen.getByRole("status").textContent).toBe("Sessions are marked Looks stuck after 10 minutes without output.");
    const reset = screen.getByTestId("pref-stuck_after_minutes-reset");
    expect(reset.textContent).toBe("Reset to 5");
    await act(async () => {
      fireEvent.click(reset);
    });
    await waitFor(() => expect(stored()?.stuck_after_minutes).toBe(5));
    expect(screen.queryByTestId("pref-stuck_after_minutes-reset")).toBeNull();
  });

  it("says why a value outside its range isn't saved, and keeps the old one", async () => {
    await renderLoaded();
    const scrollback = screen.getByLabelText("Scrollback") as HTMLInputElement;
    fireEvent.change(scrollback, { target: { value: "100" } });
    await act(async () => {
      fireEvent.blur(scrollback);
    });
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("use 500 to 100000"));
    expect(stored()).toBeUndefined();
  });

  it("a saved change reaches open terminals at once", async () => {
    const seen: SessionPrefsInfo[] = [];
    const follow = (event: Event) => seen.push((event as CustomEvent<SessionPrefsInfo>).detail);
    window.addEventListener(SESSION_PREFS_EVENT, follow);
    await sessionPrefsSet({ ...SESSION_PREFS_DEFAULT, terminal_font_size: 16, terminal_scrollback_lines: 20_000 });
    window.removeEventListener(SESSION_PREFS_EVENT, follow);
    expect(seen).toHaveLength(1);
    const term = { options: { fontSize: 12.5, scrollback: 5_000 } };
    let refits = 0;
    applyTerminalPrefs(term, seen[0]!, () => {
      refits += 1;
    });
    expect(term.options).toEqual({ fontSize: 16, scrollback: 20_000 });
    expect(refits).toBe(1);
  });
});
