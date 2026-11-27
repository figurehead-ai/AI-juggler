/**
 * AJ-E1547 (PAT-OSP-3): the sheet before a system prompt. The gate asks
 * (`os-prompt://ask`), the sheet says what the system will show and what
 * to click; Continue answers yes and waits, then the outcome is said;
 * Not now (or Esc) answers no, asks nothing else and says so, with where
 * it's offered again (0.5.1 F28); the same entry asked twice is one sheet.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { notNowOutcome, OsPromptHost, OsPromptSheet, sheetSubtitle, sheetTitle, type SheetPhase } from "./OsPromptSheet";
import type { OsPromptInfo, OsPromptOutcomeInfo } from "../lib/ipc/os-prompts";

const handlers: { ask?: (prompt: OsPromptInfo) => void; told?: (outcome: OsPromptOutcomeInfo) => void } = {};
const answers: Array<[string, boolean]> = [];

vi.mock("../lib/ipc/os-prompts", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc/os-prompts")>();
  return {
    ...original,
    onOsPromptAsk: vi.fn(async (handler: (prompt: OsPromptInfo) => void) => {
      handlers.ask = handler;
      return () => undefined;
    }),
    onOsPromptOutcome: vi.fn(async (handler: (outcome: OsPromptOutcomeInfo) => void) => {
      handlers.told = handler;
      return () => undefined;
    }),
    osPromptAnswer: vi.fn(async (id: string, go: boolean) => {
      answers.push([id, go]);
    }),
  };
});

const accessibility: OsPromptInfo = {
  id: "accessibility",
  title: "Jump to a session's own window",
  why: "Brings the exact window of a session to the front when you open it.",
  system_says: "“AI Juggler” would like to control this computer using accessibility features.",
  click: "Click Open System Settings, then turn on AI Juggler in System Settings › Privacy & Security › Accessibility.",
  if_declined: "Opening a session still brings its app forward; you pick the window yourself.",
  offered_again: "You can turn this on later in Settings › Desktop apps.",
  needed: true,
  status: "not_asked",
  primed: false,
  change_later: "System Settings › Privacy & Security › Accessibility",
  background: "Waits: nothing asks until you've said go.",
  never_shown: false,
  dialog_follows: true,
  dialog: {
    style: "alert",
    title: "“AI Juggler” would like to control this computer using accessibility features.",
    body: "Grant access to this application in Privacy & Security settings, located in System Settings.",
    buttons: ["Deny", "Open System Settings"],
    click: "Open System Settings",
  },
};

beforeEach(() => {
  answers.length = 0;
  handlers.ask = undefined;
  handlers.told = undefined;
});
afterEach(cleanup);

async function asked(prompt: OsPromptInfo = accessibility) {
  render(<OsPromptHost />);
  await act(async () => {
    await Promise.resolve();
  });
  act(() => handlers.ask?.(prompt));
}

describe("the sheet before a system prompt", () => {
  it("says the system will ask, shows its dialog and marks the button to click", async () => {
    await asked();
    expect(screen.getByRole("heading").textContent).toBe("Jump to a session's own window");
    expect(screen.getByTestId("os-prompt-subtitle").textContent).toContain("macOS will ask next");
    const picture = screen.getByRole("img");
    expect(picture.getAttribute("aria-label")).toContain("Click Open System Settings.");
    expect(picture.querySelector('[data-marked="1"]')?.textContent).toBe("Open System Settings");
    expect(screen.getByTestId("os-prompt-click").textContent).toContain("Open System Settings");
    expect(answers).toEqual([]);
  });

  it("answers yes on Continue, waits, then says the outcome", async () => {
    await asked();
    fireEvent.click(screen.getByTestId("os-prompt-continue"));
    expect(answers).toEqual([["accessibility", true]]);
    expect(screen.getByTestId("os-prompt-sheet").dataset.phase).toBe("waiting");
    act(() =>
      handlers.told?.({
        id: "accessibility",
        result: "called",
        status: "not_allowed",
        words: "Opening a session still brings its app forward; you pick the window yourself. You can change it later in System Settings › Privacy & Security › Accessibility.",
      }),
    );
    const outcome = screen.getByTestId("os-prompt-outcome");
    expect(outcome.dataset.status).toBe("not_allowed");
    expect(outcome.textContent).toContain("You can change it later");
    fireEvent.click(screen.getByTestId("os-prompt-done"));
    expect(screen.queryByTestId("os-prompt-sheet")).toBeNull();
  });

  it("answers no on Not now and on Esc, and asks nothing else", async () => {
    await asked();
    fireEvent.click(screen.getByTestId("os-prompt-not-now"));
    expect(answers).toEqual([["accessibility", false]]);
    fireEvent.click(screen.getByTestId("os-prompt-done"));
    expect(screen.queryByTestId("os-prompt-sheet")).toBeNull();
    act(() => handlers.ask?.(accessibility));
    fireEvent.keyDown(window, { key: "Escape" });
    expect(answers).toEqual([
      ["accessibility", false],
      ["accessibility", false],
    ]);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("os-prompt-sheet")).toBeNull();
    expect(answers).toHaveLength(2);
  });

  it("after Not now says the outcome and where it's offered again (0.5.1 F28)", async () => {
    await asked();
    fireEvent.click(screen.getByTestId("os-prompt-not-now"));
    expect(screen.getByRole("heading").textContent).toBe("Jump to a session's own window");
    const outcome = screen.getByTestId("os-prompt-outcome");
    expect(outcome.dataset.result).toBe("not_now");
    expect(outcome.textContent).toBe(
      "Nothing was asked. Opening a session still brings its app forward; you pick the window yourself. You can turn this on later in Settings › Desktop apps.",
    );
    // The calm way out has focus; nothing else is offered.
    expect(document.activeElement).toBe(screen.getByTestId("os-prompt-done"));
    expect(screen.queryByTestId("os-prompt-continue")).toBeNull();
    expect(answers).toEqual([["accessibility", false]]);
  });

  it("words Not now for an entry offered again on its next use", () => {
    const terminal = { ...accessibility, id: "automation-terminal", if_declined: "Terminal comes forward; you pick the tab yourself.", offered_again: "It asks again the next time you open a session that runs in Terminal." };
    expect(notNowOutcome(terminal).words).toBe(
      "Nothing was asked. Terminal comes forward; you pick the tab yourself. It asks again the next time you open a session that runs in Terminal.",
    );
  });

  /** 0.5.2 G36 (RW1 V4): Bot's helper's first start brings no dialog
   * from macOS, so its sheet promises none, draws none and has no What to
   * click; while it starts it says so, never "Answer macOS's dialog". */
  it("says a dialog is coming only when one will", async () => {
    const helper: OsPromptInfo = {
      ...accessibility,
      id: "helper",
      title: "Start Bot's AI helper",
      why: "Bot's answers are written by AI on this computer, through a small helper inside AI Juggler.",
      system_says: "",
      click: "",
      dialog: null,
      dialog_follows: false,
    };
    await asked(helper);
    expect(screen.getByTestId("os-prompt-subtitle").textContent).toBe("macOS won’t ask anything. Here’s what it does.");
    expect(screen.queryByTestId("os-prompt-picture")).toBeNull();
    expect(screen.queryByTestId("os-prompt-click")).toBeNull();
    fireEvent.click(screen.getByTestId("os-prompt-continue"));
    expect(screen.queryByTestId("os-prompt-subtitle")).toBeNull();
    expect(screen.getByTestId("os-prompt-sheet").textContent).toContain("Starting it now…");
    expect(screen.getByTestId("os-prompt-sheet").textContent).not.toContain("dialog");
    act(() => handlers.told?.({ id: "helper", result: "called", status: "allowed", words: "Bot's AI helper started: Bot answers with the AI you chose." }));
    expect(screen.getByTestId("os-prompt-outcome").textContent).toBe("Bot's AI helper started: Bot answers with the AI you chose.");
    // An entry whose call makes macOS ask still says so.
    expect(sheetSubtitle({ kind: "ask" }, accessibility)).toContain("macOS will ask next");
  });

  it("puts Not now first in focus, the calm choice", async () => {
    await asked();
    expect(document.activeElement).toBe(screen.getByTestId("os-prompt-not-now"));
  });

  it("draws a banner for a notice that asks nothing", () => {
    const login: OsPromptInfo = {
      ...accessibility,
      id: "login-item",
      dialog: { style: "banner", title: "Login Item Added", body: "“AI Juggler” is an item that can run in the background.", buttons: [], click: "" },
    };
    render(<OsPromptSheet prompt={login} phase={{ kind: "ask" }} onContinue={() => undefined} onNotNow={() => undefined} onDone={() => undefined} />);
    expect(screen.getByTestId("os-prompt-picture").dataset.style).toBe("banner");
    expect(screen.getByRole("img").getAttribute("aria-label")).not.toContain("Click");
  });

  /** AJ-E1555 R2: the heading is the registry's title; the line under
   * it never repeats it; the body is never a placeholder or a state word
   * (0.5.1's sheet: its title twice and "Asked." as its body). */
  it("never repeats its title or shows a state word as its body", async () => {
    const helperChanged: OsPromptInfo = {
      ...accessibility,
      id: "helper-changed",
      title: "Start Bot's AI helper after this update",
      why: "Bot's AI helper changed on disk since it last ran here, so the system may ask once whether to open it.",
      dialog: null,
    };
    const BANNED = ["", "Asked.", "Asked", "Allowed", "Allowed.", "Not allowed."];
    const outcome: OsPromptOutcomeInfo = { id: "helper-changed", result: "called", status: "allowed", words: "Bot's AI helper started again after the update: Bot answers as before." };
    const phases: SheetPhase[] = [{ kind: "ask" }, { kind: "waiting" }, { kind: "done", outcome }, { kind: "done", outcome: notNowOutcome(helperChanged) }];
    for (const phase of phases) {
      cleanup();
      render(<OsPromptSheet prompt={helperChanged} phase={phase} onContinue={() => undefined} onNotNow={() => undefined} onDone={() => undefined} />);
      const heading = screen.getByRole("heading").textContent ?? "";
      expect(heading).toBe(sheetTitle(helperChanged));
      const subtitle = screen.queryByTestId("os-prompt-subtitle")?.textContent ?? null;
      expect(subtitle).toBe(sheetSubtitle(phase, helperChanged));
      expect(subtitle).not.toBe(heading);
      const body = phase.kind === "done" ? screen.getByTestId("os-prompt-outcome").textContent ?? "" : document.getElementById("os-prompt-why")?.textContent ?? "";
      expect(BANNED).not.toContain(body.trim());
      expect(body.trim()).not.toBe(heading);
      expect(body.split(" ").length).toBeGreaterThan(3);
    }
  });
});
