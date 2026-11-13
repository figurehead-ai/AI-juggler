/**
 * D-97 (amends D-31), in the New session sheet: the usual App, guessed
 * from history, on a Low Account gives way to the App with room and the
 * sentence says why in the Low mark's words ("— Claude Code CLI · Work Max is
 * almost out (5% left)", "about" for an estimate); an App the user picked
 * and a project's "Always use" stand, warned, the other offered; so does
 * the repo's own Account, the only one that ever ran there (D-103.7); with
 * no other App that has room, the usual one stays, warned. The pure rule's
 * cases are `lib/new-session-choice.test.ts`; out of room is
 * `launch-sheet.test.tsx` (N5).
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as launchModule from "../lib/ipc/launch";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

const seam = (values: Record<string, unknown>) => Object.assign(window, values);
const RESET = Date.now() + 3 * 3_600_000;

const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-work", "Work Max"), folder("claude-side", "Side")];
const recipe = (id: string, account_id: string, account_label: string): LaunchRecipeInfo => ({ id, name: "Main", account_id, account_label, provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null });
const RECIPES = [recipe("p-1", "claude-work", "Work Max"), recipe("p-3", "claude-side", "Side")];

const project = (name: string, at: number, used: string[]): ProjectInfo => ({
  id: `proj-${name}`,
  root: `/Users/dev/${name}`,
  name,
  last_activity_ms: at,
  accounts_used: used,
  history_sessions: 3,
  history_last_used_ms: null,
  repo: null,
  active_instance: null,
  instance_count: 0,
  missing: false,
});
// Both Accounts ran in api: which one is a guess (D-103.7: no own Account).
const PROJECTS = [project("api", Date.now() - 12 * 60_000, ["Side", "Work Max"]), project("web", Date.now() - 86_400_000, ["Side"])];

/** One Account's reading, as the quota seam hands it over. */
const display = (account: string, left: number, badge = "measured") => ({
  account,
  weekly: { state: left === 0 ? "exhausted" : "ok", percentage: 100 - left, resets_at_ms: RESET, badge, measured_at_ms: Date.now() },
  five_hour: null,
  forecast: { state: "learning", badge: "learning" },
  renewal: { state: "none", label: "", entered_by_user: false },
});

beforeEach(() => {
  seam({ __AIJ_RECIPES__: RECIPES, __AIJ_RANKING__: [] });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  Reflect.deleteProperty(window, "__AIJ_QUOTA_DISPLAY__");
});

const mountSheet = ({ projects = PROJECTS, account = null as string | null } = {}) => {
  const launchSpy = vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
  render(<NewSessionModal projects={projects} accounts={ACCOUNTS} recipes={RECIPES} initialDirectory={null} initialAccount={account} onClose={() => {}} onLaunched={() => {}} openLoginTerminal={() => {}} />);
  return { launchSpy };
};

const sentence = () => screen.getByTestId("new-session-sentence").textContent;

describe("a Low usual App gives way (D-97)", () => {
  const withDefault = (id: string) => PROJECTS.map((row) => (row.name === "api" ? { ...row, default_account_id: id } : row));

  it("the App used here, almost out: room picks Side and the sentence says why, as the Low mark does", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 5), display("claude-side", 80)] });
    const { launchSpy } = mountSheet();
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side — Claude Code CLI · Work Max is almost out (5% left)"));
    expect(screen.queryByTestId("launch-low")).toBeNull();
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-3", "/Users/dev/api", undefined));
  });

  it("an estimated Low says about", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 5, "estimated"), display("claude-side", 80)] });
    mountSheet();
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side — Claude Code CLI · Work Max is almost out (about 5% left)"));
  });

  it("the user's pick on a Low Account stands, warned, the App with room offered", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 5), display("claude-side", 80)] });
    const { launchSpy } = mountSheet({ account: "claude-work" });
    const note = await screen.findByTestId("launch-low");
    expect(note.textContent).toContain("has only 5% left");
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max");
    expect(screen.getByTestId("launch-use-roomier").textContent).toBe("Use Claude Code CLI · Side (80% left)");
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", undefined));
  });

  it("Always use on a Low Account stands, warned", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 5), display("claude-side", 80)] });
    mountSheet({ projects: withDefault("claude-work") });
    expect(await screen.findByTestId("launch-low")).toBeTruthy();
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — your default here");
    expect(screen.getByTestId("launch-use-roomier")).toBeTruthy();
  });

  it("15% left: still marked Low, but the usual one stays, warned, the App with room offered", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 15), display("claude-side", 80)] });
    mountSheet();
    expect(await screen.findByTestId("launch-low")).toBeTruthy();
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — used here before");
    expect(screen.getByTestId("launch-use-roomier").textContent).toBe("Use Claude Code CLI · Side (80% left)");
  });

  it("D-103.7: the repo's own Account, almost out, stands, warned, the App with room offered", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 5), display("claude-side", 80)] });
    const { launchSpy } = mountSheet({ projects: [project("api", Date.now() - 12 * 60_000, ["Work Max"])] });
    expect(await screen.findByTestId("launch-low")).toBeTruthy();
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here");
    expect(screen.getByTestId("launch-use-roomier").textContent).toBe("Use Claude Code CLI · Side (80% left)");
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", undefined));
  });

  it("with no other App that has room, the usual one stays and is warned", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 5), display("claude-side", 3)] });
    mountSheet();
    expect(await screen.findByTestId("launch-low")).toBeTruthy();
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — used here before");
  });

});
