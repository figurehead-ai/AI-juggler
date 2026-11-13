/**
 * The New session sheet (AJ-E306 flow (e), reshaped by AJ-E3207): it
 * opens with the usual folder and App chosen and said in one sentence
 * with a short why, from what the app holds (no wait); ⌘↩ or the one
 * primary button starts it (N1). Change folder and Change app are one
 * step away and the sentence follows (N2). The rare options are folded,
 * their values in the fold's label (N3). With no history the guess says
 * why (N4); a usual App out of room or low gives way to the App with
 * room and says why, the user's own stands, warned, with the other
 * offered (N5, D-31 as amended by D-97). The admission verdict is words, never a
 * fabricated %; Sign in first keeps the sheet open (Req.8); "Profile"
 * never names a thing here (D-02/D-10).
 */
import { moreOptionsWords } from "../screens/new-session/MoreOptions";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as bridgeModule from "../lib/ipc/terminal-io";
import * as bridgeModuleLaunch from "../lib/ipc/launch";
import * as bridgeModuleAccounts from "../lib/ipc/accounts";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { QuotaDisplayData } from "../lib/ipc/quota";

const w = () => window as unknown as Record<string, unknown>;
const RESET = Date.now() + 3 * 3_600_000;

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_out", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-side", provider: "claude-code", label: "Side", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
] as AccountInfo[];

const RECIPES = [
  { id: "p-1", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: "opus", default_cwd: null, restart_on_crash: null },
  { id: "p-2", name: "Main", account_id: "claude-personal", account_label: "Personal", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null },
  { id: "p-3", name: "Main", account_id: "claude-side", account_label: "Side", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null },
] as LaunchRecipeInfo[];

const project = (name: string, at: number, used: string[]): ProjectInfo =>
  ({ id: `proj-${name}`, root: `/Users/dev/${name}`, name, last_activity_ms: at, accounts_used: used, history_sessions: 3, history_last_used_ms: null, repo: null, active_instance: null, instance_count: 0, missing: false }) as ProjectInfo;

const PROJECTS = [project("api", Date.now() - 12 * 60_000, ["Work Max"]), project("web", Date.now() - 86_400_000, ["Side"])];

const display = (account: string, left: number): QuotaDisplayData =>
  ({
    account,
    weekly: { state: left === 0 ? "exhausted" : "ok", percentage: 100 - left, resets_at_ms: RESET, badge: "measured", measured_at_ms: Date.now() },
    five_hour: null,
    forecast: { state: "learning", badge: "learning" },
    renewal: { state: "none", label: "", entered_by_user: false },
  }) as unknown as QuotaDisplayData;

beforeEach(() => {
  w().__AIJ_RECIPES__ = RECIPES;
  w().__AIJ_QUOTA_DISPLAY__ = [];
  w().__AIJ_RANKING__ = [{ account: "claude-work", score: 0.9, top_factors: [], factors: [], headroom_unknown: false, confidence: "Measured", suggested: true, admission: "tight" }];
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete w().__AIJ_FIXTURE__;
});

const mountSheet = ({ projects = PROJECTS, dir = null as string | null, account = null as string | null, kept = RECIPES as LaunchRecipeInfo[] | undefined, onProjectsChanged = undefined as (() => void) | undefined } = {}) => {
  const launchSpy = vi.spyOn(bridgeModuleLaunch, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 } as never);
  const signinSpy = vi.spyOn(bridgeModule, "terminalSpawnLogin").mockResolvedValue({ id: "t1" } as never);
  const launched = vi.fn();
  const mounted = render(
    <NewSessionModal projects={projects} accounts={ACCOUNTS} recipes={kept} initialDirectory={dir} initialAccount={account} onClose={() => {}} onLaunched={launched} openLoginTerminal={() => {}} onProjectsChanged={onProjectsChanged} />,
  );
  return { ...mounted, launchSpy, signinSpy, launched };
};

const sentence = () => screen.getByTestId("new-session-sentence").textContent;

describe("N1 the usual, made and said", () => {
  it("opens with the folder and App chosen in its first frame, said in one sentence with why", () => {
    mountSheet();
    // No wait: the launch settings the app holds make the choice at once.
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here");
    expect(screen.getByTestId("launch-preselected").textContent).toContain("~/api");
    expect(screen.getByTestId("launch-preselected").textContent).toContain("worked here 12 min ago");
    expect(screen.getByTestId("launch-now").textContent).toContain("Start in api");
  });

  it("⌘↩ starts it in that folder on that App, with no click", async () => {
    const { launchSpy, launched } = mountSheet();
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", undefined));
    await waitFor(() => expect(launched).toHaveBeenCalledWith("s1", expect.any(String)));
  });

  it("has exactly one primary button, and it starts the same", async () => {
    const { container, launchSpy } = mountSheet();
    expect(container.querySelectorAll("[data-testid='launch-now']")).toHaveLength(1);
    fireEvent.click(screen.getByTestId("launch-now"));
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", undefined));
  });

  it("the door that opened it chose the App; the sentence says no why for it", () => {
    mountSheet({ account: "claude-side" });
    expect(sentence()).toBe("Start in api on Claude Code CLI · Side");
  });

  it("without the app's launch settings it reads them, then says the choice", async () => {
    mountSheet({ kept: undefined });
    await waitFor(() => expect(sentence()).toContain("on Claude Code CLI · Work Max"));
  });
});

describe("N2 change it, one step away", () => {
  it("Change folder lists Recent then All; a pick updates the sentence and the App's why", async () => {
    mountSheet();
    fireEvent.click(screen.getByTestId("change-folder"));
    fireEvent.click(await screen.findByTestId("change-folder-/Users/dev/web"));
    await waitFor(() => expect(sentence()).toBe("Start in web on Claude Code CLI · Side — the only account used here"));
    expect(screen.getByTestId("launch-now").textContent).toContain("Start in web");
  });

  it("Change app lists the Apps with their room; one not signed in is last and says so", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 60), display("claude-side", 80)];
    mountSheet();
    fireEvent.click(screen.getByTestId("change-app"));
    const side = await screen.findByTestId("change-app-claude-side");
    expect(side.textContent).toContain("Claude Code CLI · Side");
    await waitFor(() => expect(screen.getByTestId("change-app-claude-side").textContent).toContain("80% left"));
    const items = screen.getAllByRole("menuitemradio").map((item) => item.getAttribute("data-testid"));
    expect(items.at(-1)).toBe("change-app-claude-personal");
    expect(screen.getByTestId("change-app-claude-personal").textContent).toContain("Not signed in");
    // rc12 walk: no account named beside it, only the App and that.
    expect(screen.getByTestId("change-app-claude-personal").textContent).toBe("Claude Code CLI · PersonalNot signed in");
    fireEvent.click(screen.getByTestId("change-app-claude-side"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side"));
  });

  it("an App not signed in: Sign in first detours WITHOUT closing the sheet, and Start waits", async () => {
    // rc7 walk: its account's room isn't said beside an App that can't start.
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-personal", 78)];
    const { signinSpy } = mountSheet({ account: "claude-personal" });
    expect(screen.getByTestId("launch-signed-out").textContent).toContain("Claude Code CLI · Personal isn't signed in");
    expect((screen.getByTestId("launch-now") as HTMLButtonElement).disabled).toBe(true);
    await act(async () => new Promise((done) => setTimeout(done, 20)));
    expect(screen.queryByTestId("launch-runway")).toBeNull();
    // rc8 walk: no account named for it either (its label, or an email).
    expect(screen.getByTestId("launch-account").textContent).toBe("not signed in");
    // rc14 walk: nor its colour (a neutral ring, as for no account).
    const ring = within(screen.getByTestId("launch-preselected")).getAllByTestId("account-avatar").find((avatar) => avatar.title === "Not signed in");
    expect(ring?.dataset.tone).toBe("neutral");
    fireEvent.click(screen.getByTestId("sign-in-first"));
    await waitFor(() => expect(signinSpy).toHaveBeenCalledWith("claude-personal"));
    expect(screen.getByTestId("new-session-sheet")).toBeTruthy();
  });
});

// PRINCIPLES' worked example: "The app picker offers 'Always use for
// jive-code-graph'. It sets the project default", which New session
// picks first. Set and cleared in Change app; out of room still warns.
describe("Always use for a project", () => {
  const withDefault = (id: string) => PROJECTS.map((row) => (row.name === "api" ? { ...row, default_account_id: id } : row));
  const openApps = () => fireEvent.click(screen.getByTestId("change-app"));

  it("Always use for api sets the project's default; New session opens on it, said as your default", async () => {
    const setSpy = vi.spyOn(bridgeModuleAccounts, "projectSetDefaultAccount").mockResolvedValue();
    const reload = vi.fn();
    const first = mountSheet({ onProjectsChanged: reload });
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here");
    openApps();
    fireEvent.click(await screen.findByTestId("change-app-claude-side"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side"));
    openApps();
    const always = await screen.findByTestId("change-app-always:set");
    expect(always.textContent).toBe("Always use for apiClaude Code CLI · Side");
    fireEvent.click(always);
    await waitFor(() => expect(setSpy).toHaveBeenCalledWith("proj-api", "claude-side"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side — your default here"));
    expect(reload).toHaveBeenCalled();
    first.unmount();
    // Opened again, on the projects as reloaded: picked first.
    mountSheet({ projects: withDefault("claude-side") });
    expect(sentence()).toBe("Start in api on Claude Code CLI · Side — your default here");
  });

  it("No default for api clears it, from the same picker; the usual choice comes back", async () => {
    const setSpy = vi.spyOn(bridgeModuleAccounts, "projectSetDefaultAccount").mockResolvedValue();
    mountSheet({ projects: withDefault("claude-side") });
    openApps();
    // Already the default: only the way back is offered.
    expect(screen.queryByTestId("change-app-always:set")).toBeNull();
    const clear = await screen.findByTestId("change-app-always:clear");
    expect(clear.textContent).toBe("No default for apiBack to the usual choice");
    fireEvent.click(clear);
    await waitFor(() => expect(setSpy).toHaveBeenCalledWith("proj-api", null));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here"));
  });

  it("the default out of room: kept and said, the App with room offered, never switched (D-31)", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-side", 0), display("claude-work", 80)];
    const { launchSpy } = mountSheet({ projects: withDefault("claude-side") });
    expect(await screen.findByTestId("launch-out-of-room")).toBeTruthy();
    expect(sentence()).toBe("Start in api on Claude Code CLI · Side — your default here");
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-3", "/Users/dev/api", undefined));
  });

  it("a default whose App was removed: the usual choice, and nothing to clear", async () => {
    mountSheet({ projects: withDefault("claude-gone") });
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here");
    openApps();
    expect(await screen.findByTestId("change-app-always:set")).toBeTruthy();
    expect(screen.queryByTestId("change-app-always:clear")).toBeNull();
  });
});

describe("N3 the rare options, folded", () => {
  it("the fold is closed and its label says their values; inside: how it runs and the rest", () => {
    mountSheet();
    const fold = screen.getByTestId("new-session-more") as HTMLDetailsElement;
    expect(fold.open).toBe(false);
    // The 0.4 design review, 51 and 71: the model as people say it, and How
    // it runs only once it differs from the launch settings (inside, it
    // says "As its launch settings say").
    expect(screen.getByTestId("new-session-more-values").textContent).toBe("· Opus");
    // Walk N3: with nothing set, the label says so.
    expect(moreOptionsWords({ recipe: { ...RECIPES[0]!, model: null }, settingsCount: 1, runtime: undefined, machine: null })).toBe("the defaults");
    expect(fold.textContent).toContain("How it runs");
    expect(fold.textContent).toContain("Start from a saved handover…");
    expect(fold.textContent).toContain("Advanced…");
  });

  // N3 step 2 (D-40, AJ-E3207 req. 3): a fold option is this start's own.
  // The next New session opens on the defaults, and nothing offers to keep it.
  it("a changed option is for this start only: the next New session opens on the defaults again", async () => {
    const first = mountSheet();
    screen.getByTestId("new-session-more").setAttribute("open", "");
    fireEvent.click(within(screen.getByTestId("runtime-pick")).getByRole("button"));
    fireEvent.click(await screen.findByText("Start in terminal"));
    await waitFor(() => expect(screen.getByTestId("new-session-more-values").textContent).toBe("· Opus · runs in its terminal"));
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(first.launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", "terminal"));
    first.unmount();
    mountSheet();
    expect(screen.getByTestId("new-session-more-values").textContent).toBe("· Opus");
    expect(screen.getByTestId("new-session-more").textContent).not.toMatch(/remember|always/i);
  });

  it("an App with several launch settings lets you pick one there, and its name joins the label", async () => {
    const quick = { ...RECIPES[0]!, id: "p-4", name: "Quick fixes", model: "sonnet" };
    w().__AIJ_RECIPES__ = [...RECIPES, quick];
    const { launchSpy } = mountSheet({ kept: [...RECIPES, quick] });
    await waitFor(() => expect(screen.getByTestId("new-session-more-values").textContent).toBe("· Main · Opus"));
    (screen.getByTestId("new-session-more") as HTMLDetailsElement).open = true;
    fireEvent.click(screen.getByTestId("new-session-settings").querySelector("button")!);
    fireEvent.click(await screen.findByText("Quick fixes"));
    await waitFor(() => expect(screen.getByTestId("new-session-more-values").textContent).toBe("· Quick fixes · Sonnet"));
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-4", "/Users/dev/api", undefined));
  });
});

describe("N4 first time", () => {
  it("with no history, the guess is the most room, and it says so", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 30), display("claude-side", 80)];
    mountSheet({ projects: [] });
    await waitFor(() => expect(sentence()).toBe("Start a session on Claude Code CLI · Side — the most room left"));
    expect(screen.getByTestId("launch-preselected").textContent).toContain("None yet: it starts in a temporary folder");
    expect(screen.getByTestId("launch-now").textContent).toContain("Start on Claude Code CLI · Side");
  });

  // Walk N4 (rc26): the lab's one project, "web", had no history and no
  // time, and the sheet opened on a temporary folder.
  it("with one project and no history, opens in that project, said as yours, with no time made up", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 30), display("claude-side", 80)];
    const web = { ...project("web", 0, []), last_activity_ms: null, history_sessions: 0 } as ProjectInfo;
    const { launchSpy } = mountSheet({ projects: [web] });
    await waitFor(() => expect(sentence()).toBe("Start in web on Claude Code CLI · Side — the most room left"));
    const card = screen.getByTestId("launch-preselected").textContent;
    expect(card).toContain("~/web · your project");
    expect(card).not.toContain("worked here");
    expect(card).not.toContain("temporary folder");
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-3", "/Users/dev/web", undefined));
  });
});

describe("N5 the usual is out of room", () => {
  // Both Accounts ran in api: its App is a guess (D-103.7: no own Account).
  const shared = PROJECTS.map((row) => (row.name === "api" ? { ...row, accounts_used: ["Side", "Work Max"] } : row));

  it("D-97: gives way to the App with room before you start, and says why", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 0), display("claude-side", 80)];
    const { launchSpy } = mountSheet({ projects: shared });
    await waitFor(() => expect(sentence()).toMatch(/^Start in api on Claude Code CLI · Side — Claude Code CLI · Work Max is out of room until \S/));
    // The choice has room: nothing to warn about below.
    expect(screen.queryByTestId("launch-out-of-room")).toBeNull();
    fireEvent.click(screen.getByTestId("launch-now"));
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-3", "/Users/dev/api", undefined));
  });

  it("D-103.7: the repo's own Account out of room stays, warned, the App with room offered; never switched", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 0), display("claude-side", 80)];
    const { launchSpy } = mountSheet();
    expect(await screen.findByTestId("launch-out-of-room")).toBeTruthy();
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here");
    expect(screen.getByTestId("launch-use-roomier").textContent).toBe("Use Claude Code CLI · Side (80% left)");
    fireEvent.click(screen.getByTestId("launch-now"));
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", undefined));
  });

  it("with no other App that has room, it stays: said before you start, the unmeasured one offered", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 0)];
    const { launchSpy } = mountSheet();
    const note = await screen.findByTestId("launch-out-of-room");
    expect(note.textContent).toMatch(/ is out of room until .+: a session there would stop at its limit at once\./);
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here");
    expect(screen.getByTestId("launch-use-roomier").textContent).toBe("Use Claude Code CLI · Side (its room isn't measured)");
    // Still the user's: Start goes where it said.
    fireEvent.click(screen.getByTestId("launch-now"));
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", undefined));
  });

  it("taking the offer changes the choice", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 0)];
    mountSheet();
    fireEvent.click(await screen.findByTestId("launch-use-roomier"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side"));
    expect(screen.queryByTestId("launch-out-of-room")).toBeNull();
  });
});

// D-96 E (persona D5): the right App, and now the reason too.
describe("a skip says why (D-96 E)", () => {
  // No history and no default: room picks, over Work Max, the App it
  // would have opened on.
  const fresh = PROJECTS.map((row) => ({ ...row, accounts_used: [] }));

  it("room picked over an App almost out: the sentence names that one and its room, as the Low mark says it", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 5), display("claude-side", 80)];
    const { launchSpy } = mountSheet({ projects: fresh });
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side — Claude Code CLI · Work Max is almost out (5% left)"));
    // The choice has room: nothing to warn about below.
    expect(screen.queryByTestId("launch-low")).toBeNull();
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-3", "/Users/dev/api", undefined));
  });

  it("an estimated figure says about, as the Low mark does", async () => {
    const estimated = { ...display("claude-work", 5), weekly: { ...display("claude-work", 5).weekly, badge: "estimated" } };
    w().__AIJ_QUOTA_DISPLAY__ = [estimated, display("claude-side", 80)];
    mountSheet({ projects: fresh });
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side — Claude Code CLI · Work Max is almost out (about 5% left)"));
  });

  it("names the Account by its own name, out of room with its reset", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 0), display("claude-side", 80)];
    w().__AIJ_FIXTURE__ = {
      subscriptions: [{ key: "k1", account_id: "acct-1", name: "Work", tool: "claude-code", state: "signed_in", email: null, organisation: null, reason: null, plan: "max", renewal_date: null, fee_usd: null, differs: [], profiles: [{ id: "claude-work", label: "Work Max", folder: "~/.claude-work", plan: null, renewal_date: null, fee_usd: null }] }],
    };
    mountSheet({ projects: fresh });
    await waitFor(() => expect(sentence()).toMatch(/^Start in api on Claude Code CLI · Side — Work is out of room until \S/));
  });

  it("an App the user picked is never said as a skip", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 5), display("claude-side", 80)];
    mountSheet({ projects: fresh, account: "claude-side" });
    await waitFor(() => expect(screen.getByTestId("launch-runway")).toBeTruthy());
    expect(sentence()).toBe("Start in api on Claude Code CLI · Side");
  });
});

describe("words", () => {
  it("the admission verdict is words; unknown room says nothing, never a fabricated %", async () => {
    mountSheet();
    expect(await screen.findByText("Tight — this session may use most of the room left")).toBeTruthy();
    expect(screen.getByTestId("launch-preselected").textContent).not.toMatch(/%/);
    expect(screen.queryByTestId("launch-runway")).toBeNull();
  });

  it("names an App by its App name and its Account by its own, never a profile", async () => {
    w().__AIJ_FIXTURE__ = {
      subscriptions: [{ key: "k1", account_id: "acct-1", name: "Work", tool: "claude-code", state: "signed_in", email: null, organisation: null, reason: null, plan: "max", renewal_date: null, fee_usd: null, differs: [], profiles: [{ id: "claude-work", label: "Work Max", folder: "~/.claude-work", plan: null, renewal_date: null, fee_usd: null }] }],
    };
    const { container } = mountSheet();
    await waitFor(() => expect(screen.getByTestId("launch-preselected").textContent).toMatch(/Account\s*Work/));
    expect(container.textContent).not.toMatch(/\bprofile\b/i);
    // The 0.4 design review, 71: the Account's own dot on its line, in its colour.
    await waitFor(() => expect(within(screen.getByTestId("launch-preselected")).getByTestId("account-avatar").dataset.tone).toBe("colour"));
  });
});
