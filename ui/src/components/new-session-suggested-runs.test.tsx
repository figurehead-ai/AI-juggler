/**
 * SK-6 (the 0.4.6 soak, N5): New session kept "Suggested" on an App whose
 * Account was out of room, while its own note said a session there
 * "would stop at its limit at once". It never suggests an App that can't
 * run now: the repo's own Account (D-103.7) stays the pick, warned, with
 * no mark, and the App it offers with room is the one marked Suggested,
 * in the note and in Change app.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as launchModule from "../lib/ipc/launch";
import { appsByRoom, chooseApp, suggestedMarkOn } from "../lib/new-session-choice";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

const seam = (values: Record<string, unknown>) => Object.assign(window, values);
const RESET = Date.now() + 3 * 3_600_000;

const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-work", "Work Max"), folder("claude-side", "Side")];
const recipe = (id: string, account_id: string, account_label: string): LaunchRecipeInfo => ({ id, name: "Main", account_id, account_label, provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null });
const RECIPES = [recipe("p-1", "claude-work", "Work Max"), recipe("p-3", "claude-side", "Side")];
// Only Work Max ever ran in web: its own Account (D-103.7).
const WEB: ProjectInfo = { id: "proj-web", root: "/Users/dev/web", name: "web", last_activity_ms: Date.now() - 60_000, accounts_used: ["Work Max"], history_sessions: 3, history_last_used_ms: null, repo: null, active_instance: null, instance_count: 0, missing: false };

const display = (account: string, left: number) => ({
  account,
  weekly: { state: left === 0 ? "exhausted" : "ok", percentage: 100 - left, resets_at_ms: RESET, badge: "measured", measured_at_ms: Date.now() },
  five_hour: null,
  forecast: { state: "learning", badge: "learning" },
  renewal: { state: "none", label: "", entered_by_user: false },
});

beforeEach(() => {
  seam({ __AIJ_RECIPES__: RECIPES, __AIJ_RANKING__: [] });
  vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  Reflect.deleteProperty(window, "__AIJ_QUOTA_DISPLAY__");
});

const mountSheet = () => render(<NewSessionModal projects={[WEB]} accounts={ACCOUNTS} recipes={RECIPES} initialDirectory={null} onClose={() => {}} onLaunched={() => {}} openLoginTerminal={() => {}} />);

describe("SK-6 Suggested is never an App that can't run now", () => {
  it("the repo's own App, out of room, stays the pick with no mark; the App with room it offers is Suggested", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 0), display("claude-side", 80)] });
    mountSheet();
    const note = await screen.findByTestId("launch-out-of-room");
    expect(note.textContent).toContain("would stop at its limit at once");
    expect(screen.getByTestId("launch-app").textContent).toContain("Work Max");
    expect(within(screen.getByTestId("launch-app-line")).queryByTestId("suggested-mark")).toBeNull();
    expect(within(note).getByTestId("suggested-mark")).toBeTruthy();
    // Change app's hint says so on the one with room, never on Work Max.
    fireEvent.click(screen.getByTestId("change-app"));
    expect((await screen.findByTestId("change-app-claude-side")).textContent).toContain("Suggested");
    expect(screen.getByTestId("change-app-claude-work").textContent).not.toContain("Suggested");
  });

  it("the rule: its own choice when it can run; the offered one with room when it can't; none for one unmeasured", () => {
    const folders = ACCOUNTS;
    const roomOf = (left: Record<string, number | null>) => (id: string) => ({ left: left[id] ?? null, resetsAtMs: null, credits: false });
    const own = { recipes: RECIPES, project: WEB, folders, ranHere: ["claude-work"] };
    const out = { ...own, roomOf: roomOf({ "claude-work": 0, "claude-side": 80 }) };
    const outChoice = chooseApp(out);
    expect(outChoice).toMatchObject({ why: "own-account", kind: "out" });
    expect(suggestedMarkOn(outChoice, appsByRoom(out))).toBe("claude-side");
    const roomy = { ...own, roomOf: roomOf({ "claude-work": 60, "claude-side": 80 }) };
    expect(suggestedMarkOn(chooseApp(roomy), appsByRoom(roomy))).toBe("claude-work");
    // The App offered isn't measured: it's offered, not Suggested.
    const unmeasured = { ...own, roomOf: roomOf({ "claude-work": 0, "claude-side": null }) };
    expect(suggestedMarkOn(chooseApp(unmeasured), appsByRoom(unmeasured))).toBeNull();
  });
});
