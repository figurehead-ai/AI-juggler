/**
 * The New session sheet (AJ-E306 flow (e), reshaped by AJ-E3207): it
 * opens with the usual folder and App chosen and said in one sentence
 * with a short why, from what the app holds (no wait); ⌘↩ or the one
 * primary button starts it (N1). Change folder and Change app are one
 * step away and the sentence follows (N2). The rare options are folded,
 * their values in the fold's label (N3). With no history the guess says
 * why (N4); out of room is said and the App with room offered, never
 * taken by itself (N5, D-31). The admission verdict is words, never a
 * fabricated %; Sign in first keeps the sheet open (Req.8); "Profile"
 * never names a thing here (D-02/D-10).
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as bridgeModule from "../lib/bridge";
import type { AccountInfo, LaunchRecipeInfo, ProjectInfo, QuotaDisplayData } from "../lib/bridge";

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

const mountSheet = ({ projects = PROJECTS, dir = null as string | null, account = null as string | null, kept = RECIPES as LaunchRecipeInfo[] | undefined } = {}) => {
  const launchSpy = vi.spyOn(bridgeModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 } as never);
  const signinSpy = vi.spyOn(bridgeModule, "terminalSpawnLogin").mockResolvedValue({ id: "t1" } as never);
  const launched = vi.fn();
  const mounted = render(
    <NewSessionModal projects={projects} accounts={ACCOUNTS} recipes={kept} initialDirectory={dir} initialAccount={account} onClose={() => {}} onLaunched={launched} openLoginTerminal={() => {}} />,
  );
  return { ...mounted, launchSpy, signinSpy, launched };
};

const sentence = () => screen.getByTestId("new-session-sentence").textContent;

describe("N1 the usual, made and said", () => {
  it("opens with the folder and App chosen in its first frame, said in one sentence with why", () => {
    mountSheet();
    // No wait: the launch settings the app holds make the choice at once.
    expect(sentence()).toBe("Start in api on Claude Code · Work Max — the last you used here");
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
    expect(sentence()).toBe("Start in api on Claude Code · Side");
  });

  it("without the app's launch settings it reads them, then says the choice", async () => {
    mountSheet({ kept: undefined });
    await waitFor(() => expect(sentence()).toContain("on Claude Code · Work Max"));
  });
});

describe("N2 change it, one step away", () => {
  it("Change folder lists Recent then All; a pick updates the sentence and the App's why", async () => {
    mountSheet();
    fireEvent.click(screen.getByTestId("change-folder"));
    fireEvent.click(await screen.findByTestId("change-folder-/Users/dev/web"));
    await waitFor(() => expect(sentence()).toBe("Start in web on Claude Code · Side — the last you used here"));
    expect(screen.getByTestId("launch-now").textContent).toContain("Start in web");
  });

  it("Change app lists the Apps with their room; one not signed in is last and says so", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 60), display("claude-side", 80)];
    mountSheet();
    fireEvent.click(screen.getByTestId("change-app"));
    const side = await screen.findByTestId("change-app-claude-side");
    expect(side.textContent).toContain("Claude Code · Side");
    await waitFor(() => expect(screen.getByTestId("change-app-claude-side").textContent).toContain("80% left"));
    const items = screen.getAllByRole("menuitemradio").map((item) => item.getAttribute("data-testid"));
    expect(items.at(-1)).toBe("change-app-claude-personal");
    expect(screen.getByTestId("change-app-claude-personal").textContent).toContain("Not signed in");
    fireEvent.click(screen.getByTestId("change-app-claude-side"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code · Side"));
  });

  it("an App not signed in: Sign in first detours WITHOUT closing the sheet, and Start waits", async () => {
    const { signinSpy } = mountSheet({ account: "claude-personal" });
    expect(screen.getByTestId("launch-signed-out").textContent).toContain("Claude Code · Personal isn't signed in");
    expect((screen.getByTestId("launch-now") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("sign-in-first"));
    await waitFor(() => expect(signinSpy).toHaveBeenCalledWith("claude-personal"));
    expect(screen.getByTestId("new-session-sheet")).toBeTruthy();
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
    expect(fold.textContent).toContain("How it runs");
    expect(fold.textContent).toContain("Start from a saved handover…");
    expect(fold.textContent).toContain("Advanced…");
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
    await waitFor(() => expect(sentence()).toBe("Start a session on Claude Code · Side — the most room left"));
    expect(screen.getByTestId("launch-preselected").textContent).toContain("None yet: it starts in a temporary folder");
    expect(screen.getByTestId("launch-now").textContent).toContain("Start on Claude Code · Side");
  });
});

describe("N5 the usual is out of room", () => {
  it("says so before you start, offers the App with room, and never switches by itself", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 0), display("claude-side", 80)];
    const { launchSpy } = mountSheet();
    const note = await screen.findByTestId("launch-out-of-room");
    expect(note.textContent).toMatch(/ is out of room until .+: a session there would stop at its limit at once\./);
    expect(sentence()).toBe("Start in api on Claude Code · Work Max — the last you used here");
    expect(screen.getByTestId("launch-use-roomier").textContent).toBe("Use Claude Code · Side (80% left)");
    // Still the user's: Start goes where it said.
    fireEvent.click(screen.getByTestId("launch-now"));
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", undefined));
  });

  it("taking the offer changes the choice", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [display("claude-work", 0), display("claude-side", 80)];
    mountSheet();
    fireEvent.click(await screen.findByTestId("launch-use-roomier"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code · Side"));
    expect(screen.queryByTestId("launch-out-of-room")).toBeNull();
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
