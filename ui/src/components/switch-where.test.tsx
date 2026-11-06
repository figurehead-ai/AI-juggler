import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import type { PreflightInfoData } from "../lib/ipc/switch-preflight";
import type { DestinationsInfo } from "../types/generated/DestinationsInfo";
import { EXAMPLE_DESTINATIONS, EXAMPLE_DESTINATIONS_APP_SIGN_IN, EXAMPLE_DESTINATIONS_DESKTOP, EXAMPLE_DESTINATIONS_NO_DESKTOP } from "../lib/ipc/switchDestinationsExamples";
import { parseSourceKey, switchSourceOf } from "../lib/ipc/conversations";

/**
 * 0.3.5 — where the switched conversation continues (the maintainer,
 * 2026-11-02: "always going to give two options … Just call it a CLI and
 * Desktop"; "you should have info there and all the details, but it should
 * not be shown by default"): two cards on the last screen, where the
 * original ran chosen at first, the button naming the choice, Desktop never
 * hidden, the choice sent to the switch, and every fold closed at first.
 */

const ONECLICK = {
  mechanism: "transplant",
  fidelity: "F4",
  fidelity_words: "Continues exactly",
  score: 1,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: true,
  source_keeps_running: false,
  experimental_copy: "Beta, not proven yet: the whole session is carried over. The source stays resumable.",
  door: "one-click",
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: { carried: [], regenerated: [], dropped: [] },
  redaction: { global: "strip", override: null, effective: "strip", source: "global", line: "" },
  redaction_items: [],
  carry_items: [
    { label: "The whole conversation", hint: "Every message from the start.", how: "moves", kind: "chat" },
    { label: "Project rules", hint: "The rule files.", how: "moves", kind: "rules" },
    { label: "The task list", hint: "From its latest version.", how: "recreated", kind: "tasks" },
  ],
  move_line: "The whole conversation is carried over — all 42 messages.",
  secrets: { asks: false, reader: "Claude Code (Anthropic)" },
  target_tool: "claude-code",
} as unknown as PreflightInfoData;

const WIZARD = { ...ONECLICK, door: "wizard", wizard_steps: ["Target", "What's carried", "Confirm"], wizard_reasons: ["This switch hasn't been proven on a real setup yet."] } as PreflightInfoData;

/** A session another app (the Claude app) runs. */
const OUTSIDE = { ...ONECLICK, source_keeps_running: true, source_outside: true, source_app: "Claude" } as PreflightInfoData;

type Seams = {
  __AIJ_SWITCH_PREFLIGHT__?: unknown;
  __AIJ_SWITCH_EXECUTE_DESTINATION__?: string[];
  __AIJ_SWITCH_EXECUTE_HANG__?: boolean;
};
const seams = () => window as never as Seams;

beforeEach(() => {
  delete seams().__AIJ_SWITCH_EXECUTE_DESTINATION__;
  delete seams().__AIJ_SWITCH_EXECUTE_HANG__;
});
afterEach(() => {
  cleanup();
  delete seams().__AIJ_SWITCH_PREFLIGHT__;
  localStorage.clear();
});

function mount(preflight: PreflightInfoData, destinations: DestinationsInfo, onSwitched = vi.fn()) {
  seams().__AIJ_SWITCH_PREFLIGHT__ = { ...preflight, destinations };
  return render(<SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" onClose={vi.fn()} onSwitched={onSwitched} />);
}

const radio = (view: ReturnType<typeof render>, name: RegExp) => view.getByRole("radio", { name }) as HTMLInputElement;
/** A card's words (its test id is on the radio inside it). */
const card = (view: ReturnType<typeof render>, which: "desktop" | "cli") => view.getByTestId(`switch-where-${which}`).closest("label")!.textContent;

async function toConfirm(view: ReturnType<typeof render>) {
  await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
  fireEvent.click(view.getByTestId("switch-continue"), { detail: 1 });
  fireEvent.click(view.getByTestId("switch-continue"), { detail: 1 });
  await waitFor(() => expect(view.getByTestId("switch-step-confirm")).toBeTruthy());
}

describe("0.3.5 where it continues: Desktop or CLI", () => {
  it("chooses Desktop at first when the original ran in the Claude app, and the button says so", async () => {
    const view = mount(OUTSIDE, EXAMPLE_DESTINATIONS_DESKTOP);
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    expect(radio(view, /^Desktop/).checked).toBe(true);
    expect(radio(view, /^CLI/).checked).toBe(false);
    expect(card(view, "desktop")).toContain("Same as the original");
    expect(view.getByTestId("switch-where").textContent).toContain("Claude app · Personal");
    expect(view.getByTestId("switch-where").textContent).toContain("Claude Code in a terminal");
    expect(view.getByTestId("switch-confirm").textContent).toBe("Switch to Desktop");
    // It isn't open on Personal: one quiet line says AI Juggler opens it.
    expect(view.getByTestId("switch-where-starts").textContent).toBe("AI Juggler starts it as it switches.");
    expect(view.getByTestId("switch-outside").textContent).toBe("The original stays open in Claude, untouched. It continues in Claude app · Personal.");
  });

  it("chooses CLI at first when the original ran in a terminal; picking Desktop changes the button", async () => {
    const view = mount(ONECLICK, EXAMPLE_DESTINATIONS);
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    expect(radio(view, /^CLI/).checked).toBe(true);
    expect(card(view, "cli")).toContain("Same as the original");
    expect(view.getByTestId("switch-confirm").textContent).toBe("Switch to CLI");
    fireEvent.click(radio(view, /^Desktop/));
    expect(view.getByTestId("switch-confirm").textContent).toBe("Switch to Desktop");
    // Running already: no line about starting it.
    expect(view.queryByTestId("switch-where-starts")).toBeNull();
  });

  it("shows Desktop greyed with its reason when the target has no Claude app profile, never hides it", async () => {
    // The original was on Desktop, but the target can't take it there.
    const view = mount(OUTSIDE, { ...EXAMPLE_DESTINATIONS_NO_DESKTOP, suggested: "desktop" });
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    const desktop = radio(view, /^Desktop/);
    expect(desktop.disabled).toBe(true);
    expect(card(view, "desktop")).toContain("No desktop app profile on Personal yet. Open a session on it in the app once, and Desktop is offered here.");
    expect(radio(view, /^CLI/).checked).toBe(true);
    expect(view.getByTestId("switch-confirm").textContent).toBe("Switch to CLI");
  });

  it("0.4: a Claude app profile signed in through the app picks Desktop, and CLI says why not", async () => {
    // The original ran in a terminal; the target signs in through its app.
    const view = mount(ONECLICK, { ...EXAMPLE_DESTINATIONS_APP_SIGN_IN, suggested: "cli" });
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    expect(radio(view, /^Desktop/).checked).toBe(true);
    const cli = radio(view, /^CLI/);
    expect(cli.disabled).toBe(true);
    expect(card(view, "cli")).toContain("signs in through Claude app · Personal");
    // Still where the original ran: CLI keeps its "Same as the original".
    expect(card(view, "cli")).toContain("Same as the original");
    expect(view.getByTestId("switch-confirm").textContent).toBe("Switch to Desktop");
  });

  it("offers both on the wizard's last step too, and the Target step says it's chosen there", async () => {
    const view = mount(WIZARD, EXAMPLE_DESTINATIONS);
    await waitFor(() => expect(view.getByTestId("switch-step-target")).toBeTruthy());
    expect(view.getByTestId("switch-target-next").textContent).toContain("pick Desktop or CLI");
    await toConfirm(view);
    expect(radio(view, /^Desktop/)).toBeTruthy();
    expect(radio(view, /^CLI/)).toBeTruthy();
  });

  it("sends the choice to the switch", async () => {
    const view = mount(ONECLICK, EXAMPLE_DESTINATIONS);
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    fireEvent.click(radio(view, /^Desktop/));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(seams().__AIJ_SWITCH_EXECUTE_DESTINATION__).toEqual(["desktop"]));
  });

  it("ends on Desktop with where it opened and a button to its page (the Claude app's conversation)", async () => {
    const onSwitched = vi.fn();
    const view = mount(ONECLICK, EXAMPLE_DESTINATIONS, onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    fireEvent.click(radio(view, /^Desktop/));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-done")).toBeTruthy());
    expect(view.getByTestId("switch-done-where").textContent).toBe("Opened in Claude app · Personal");
    // No undo window for a conversation the Claude app now holds.
    expect(onSwitched).not.toHaveBeenCalled();
    fireEvent.click(view.getByRole("button", { name: "Open its page" }));
    const [key, undo] = onSwitched.mock.calls[0]!;
    expect(key).toMatch(/^conversation:p-1:preview-/);
    expect(undo).toBeNull();
    expect(parseSourceKey(key)).toEqual({ accountId: "p-1", id: key.split(":")[2] });
  });

  it("ends on the CLI as before: the new session, with the undo window", async () => {
    const onSwitched = vi.fn();
    const view = mount(ONECLICK, EXAMPLE_DESTINATIONS, onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(onSwitched).toHaveBeenCalledTimes(1));
    expect(onSwitched.mock.calls[0]![0]).toMatch(/^sess-preview-/);
    expect(onSwitched.mock.calls[0]![1]?.edgeId).toMatch(/^edge-preview-/);
  });

  it("names the desktop app while it opens there", async () => {
    seams().__AIJ_SWITCH_EXECUTE_HANG__ = true;
    const view = mount(ONECLICK, EXAMPLE_DESTINATIONS);
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    fireEvent.click(radio(view, /^Desktop/));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-executing").textContent).toContain("Opening it in the Claude app on Personal"));
  });

  it("a backend that predates the choice offers CLI (Desktop shown, greyed)", async () => {
    seams().__AIJ_SWITCH_PREFLIGHT__ = { ...ONECLICK, destinations: undefined };
    const view = render(<SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" onClose={vi.fn()} onSwitched={vi.fn()} />);
    await waitFor(() => expect(view.getByTestId("switch-where")).toBeTruthy());
    expect(radio(view, /^Desktop/).disabled).toBe(true);
    expect(radio(view, /^CLI/).checked).toBe(true);
    expect(view.getByTestId("switch-confirm").textContent).toBe("Switch to CLI");
  });
});

describe("0.3.5 simple by default, details on demand", () => {
  it("keeps Details and every 'What does this mean?' closed at first, even after Details was opened before", async () => {
    localStorage.setItem("aij.switch.details", "open");
    const view = mount(OUTSIDE, EXAMPLE_DESTINATIONS_DESKTOP);
    await waitFor(() => expect(view.getByTestId("switch-details")).toBeTruthy());
    expect(view.getByTestId("switch-details").hasAttribute("open")).toBe(false);
    for (const fold of view.getByTestId("switch-flow").querySelectorAll("details")) expect(fold.hasAttribute("open")).toBe(false);
  });

  it("says what goes along in one line that opens to the list, chips tinted and with an icon each", async () => {
    const view = mount(ONECLICK, EXAMPLE_DESTINATIONS);
    await waitFor(() => expect(view.getByTestId("switch-carry-summary")).toBeTruthy());
    const summary = view.getByTestId("switch-carry-summary");
    expect(summary.hasAttribute("open")).toBe(false);
    expect(within(summary).getByTestId("switch-carry-words").textContent).toBe("Everything goes along · 3 items");
    const moves = within(summary).getByTestId("switch-carry-moves");
    expect(moves.querySelectorAll("svg").length).toBe(2);
    expect(moves.querySelector("span")!.className).toContain("bg-success-soft");
    expect(within(summary).getByTestId("switch-carry-recreated").querySelector("span")!.className).toContain("bg-accent-soft");
  });

  it("says what stays behind in the warning tint", async () => {
    const view = mount({ ...ONECLICK, carry_items: [...ONECLICK.carry_items.slice(0, 2), { label: "The task list", hint: "It doesn't go along.", how: "stays", kind: "tasks" }] } as PreflightInfoData, EXAMPLE_DESTINATIONS);
    await waitFor(() => expect(view.getByTestId("switch-carry-summary")).toBeTruthy());
    expect(view.getByTestId("switch-carry-words").textContent).toBe("2 go along · 1 stays behind");
    expect(view.getByTestId("switch-carry-stays").querySelector("span")!.className).toContain("bg-warning-soft");
  });

  it("colours the step marks: done ticked in the success tint, the current one in the accent", async () => {
    const view = mount(WIZARD, EXAMPLE_DESTINATIONS);
    await toConfirm(view);
    const marks = view.getByTestId("switch-steps").querySelectorAll("li");
    expect(marks[0]!.dataset.done).toBe("true");
    expect(marks[0]!.className).toContain("bg-success-soft");
    expect(marks[0]!.querySelector("svg")).toBeTruthy();
    expect(marks[2]!.getAttribute("aria-current")).toBe("step");
    expect(marks[2]!.className).toContain("bg-accent-soft");
  });
});

describe("0.3.5 never 'here in AI Juggler'", () => {
  // A word of its own ("Where in AI Juggler's code", a crash report's field, is fine).
  const PHRASE = /\bhere in AI Juggler/i;

  it("on no switch screen, from the first step to done, for a session another app runs and one AI Juggler runs", async () => {
    for (const preflight of [{ ...WIZARD, source_outside: true, source_app: "Claude", source_keeps_running: true }, WIZARD] as PreflightInfoData[]) {
      const view = mount(preflight, EXAMPLE_DESTINATIONS_DESKTOP);
      await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
      for (const _ of preflight.wizard_steps) {
        expect(view.getByTestId("switch-flow").textContent).not.toMatch(PHRASE);
        const next = view.queryByTestId("switch-continue");
        if (next) fireEvent.click(next, { detail: 1 });
      }
      for (const fold of view.getByTestId("switch-flow").querySelectorAll("details")) fold.setAttribute("open", "");
      expect(view.getByTestId("switch-flow").textContent).not.toMatch(PHRASE);
      fireEvent.click(view.getByTestId("switch-confirm"), { detail: 1 });
      await waitFor(() => expect(view.getByTestId("switch-done")).toBeTruthy());
      expect(view.getByTestId("switch-flow").textContent).not.toMatch(PHRASE);
      cleanup();
    }
  });

  it("in no source file the app ships", () => {
    const sources = import.meta.glob("../**/*.{ts,tsx}", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
    const hits = Object.entries(sources)
      .filter(([path]) => !/\.test\.tsx?$/.test(path))
      .filter(([, text]) => PHRASE.test(text))
      .map(([path]) => path);
    expect(hits).toEqual([]);
  });

  it("reads a conversation's key back as switchSourceOf writes it", () => {
    const key = switchSourceOf({ session_id: null, account_id: "claude-luka", id: "abc:def" });
    expect(parseSourceKey(key)).toEqual({ accountId: "claude-luka", id: "abc:def" });
    expect(parseSourceKey("sess-1")).toBeNull();
  });
});
