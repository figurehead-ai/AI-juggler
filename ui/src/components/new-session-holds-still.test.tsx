/**
 * AJ-E2312 item 5 (0.4.7's nightly walk V2: in N5 the New session sheet
 * moved ~50 px when its out-of-room warning left, in P7 ~20 px when
 * Suggested left the App's line). 0.4.7 held the sheet at the top so a
 * line that comes grows it downward; now it holds still when a line goes
 * too, with no dead gap: a warning the person answered by moving off its
 * App stays answered in its place, with Undo, and the card's least height
 * is its tallest so far.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as launchModule from "../lib/ipc/launch";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import { clearSeams, setSeam } from "../test/fixtures";

const RESET = Date.now() + 3 * 3_600_000;
const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-work", "Work Max"), folder("claude-side", "Side")];
const RECIPES: LaunchRecipeInfo[] = [
  { id: "p-1", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: "opus", default_cwd: null, restart_on_crash: null },
  { id: "p-3", name: "Main", account_id: "claude-side", account_label: "Side", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null },
];
// api's own Account is Work's (the only one used there): it opens on Work, warned.
const API: ProjectInfo = { id: "proj-api", root: "/Users/you/lab/api", name: "api", last_activity_ms: Date.now() - 60_000, accounts_used: ["Work Max"], history_sessions: 3, history_last_used_ms: null, repo: null, active_instance: null, instance_count: 0, missing: false };
// Work's week is used up until RESET.
const out: QuotaDisplayData = {
  account: "claude-work",
  weekly: { state: "exhausted", percentage: 100, badge: "measured", stale_note_ms: null, resets_at_ms: RESET, reset_basis: "measured", measured_at_ms: Date.now(), burned_tokens: 0, burned_credits: null, headroom_tokens: null },
  weekly_omitted_reason: null,
  five_hour: null,
  five_hour_omitted_reason: null,
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "learning" },
  unavailable_reason: null,
  credits: null,
};

beforeEach(() => {
  setSeam("__AIJ_RECIPES__", RECIPES);
  setSeam("__AIJ_QUOTA_DISPLAY__", [out]);
  setSeam("__AIJ_RANKING__", []);
  vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  clearSeams("__AIJ_RECIPES__", "__AIJ_QUOTA_DISPLAY__", "__AIJ_RANKING__");
});

const mount = () => render(<NewSessionModal projects={[API]} accounts={ACCOUNTS} recipes={RECIPES} initialDirectory={API.root} onClose={() => {}} onLaunched={() => {}} />);
const sentence = () => screen.getByTestId("new-session-sentence").textContent;

describe("New session holds still when its warning goes (AJ-E2312 item 5)", () => {
  it("the warning the person answered stays answered in its place, with Undo", async () => {
    mount();
    const warning = await screen.findByTestId("launch-out-of-room");
    const place = warning.parentElement!;
    fireEvent.click(screen.getByTestId("launch-use-roomier"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side"));
    expect(screen.queryByTestId("launch-out-of-room")).toBeNull();
    const answered = within(place).getByTestId("launch-app-left");
    // Its Account named as the warning named it.
    expect(answered.textContent).toMatch(/^App changed to Claude Code CLI · Side ?\(was Claude Code CLI · Work Max\): .+ is out of room until .+\. Undo$/);
    // Undo: back to Work, warned again, and the answer goes.
    fireEvent.click(within(answered).getByTestId("launch-app-left-undo"));
    await waitFor(() => expect(sentence()).toMatch(/^Start in api on Claude Code CLI · Work Max/));
    expect(await screen.findByTestId("launch-out-of-room")).toBeTruthy();
    expect(screen.queryByTestId("launch-app-left")).toBeNull();
  });

  it("an App moved to from Change app is answered the same way", async () => {
    mount();
    await screen.findByTestId("launch-out-of-room");
    fireEvent.click(screen.getByTestId("change-app"));
    fireEvent.click(await screen.findByTestId("change-app-claude-side"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side"));
    expect(screen.getByTestId("launch-app-left").textContent).toMatch(/\(was Claude Code CLI · Work Max\): .+ is out of room until /);
  });

  it("the card never gets shorter while the sheet is open: a line that goes leaves its room, so nothing under it moves up", async () => {
    // The card's height, as drawn: its words' length stands in for it here.
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const height = this.dataset.testid === "launch-preselected" ? (this.textContent ?? "").length : 0;
      return { height, width: 0, top: 0, left: 0, bottom: height, right: 0, x: 0, y: 0, toJSON: () => ({}) };
    });
    mount();
    await screen.findByTestId("launch-out-of-room");
    const card = screen.getByTestId("launch-preselected");
    // Held at its tallest so far.
    const held = card.style.minHeight;
    expect(parseFloat(held)).toBeGreaterThanOrEqual((card.textContent ?? "").length);
    // P7's kind: the App shown picked again is the person's own; its why
    // ("— the only account used here") goes, and the card is shorter.
    fireEvent.click(screen.getByTestId("change-app"));
    fireEvent.click(await screen.findByTestId("change-app-claude-work"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max"));
    expect((card.textContent ?? "").length).toBeLessThan(parseFloat(held));
    expect(card.style.minHeight).toBe(held);
  });
});
