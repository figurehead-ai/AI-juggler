/**
 * D-25 vitest — Home: what needs you (one sentence, one action), what's
 * running, each Account's runway (measured, or a session's own reading,
 * or "Not measured yet"), and the plain next step when nothing is set up.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeScreen, needOf, needWords, placeOf, runwayOf } from "./Home";
import type { AccountInfo, InstanceInfo, ProjectInfo, QuotaDisplayData } from "../lib/bridge";

const NOW = 1_728_800_000_000;
const w = () => window as unknown as Record<string, unknown>;

function mk(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: "main",
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-1",
    conversation_id: null,
    ...overrides,
  };
}

const ACCOUNTS: AccountInfo[] = [
  { id: "work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in" },
  { id: "personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in" },
] as AccountInfo[];
const PROJECTS: ProjectInfo[] = [
  { id: "proj-1", root: "/w/ai-juggler", name: "ai-juggler", last_activity_ms: null, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, missing: false },
];

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
});

describe("D-25 Home — the rules", () => {
  it("a need comes from the status, or from the attention flag", () => {
    expect(needOf(mk("a", { status: "failed" }))).toBe("failed");
    expect(needOf(mk("b", { status: "running", needs_attention: "waiting_user" }))).toBe("waiting_user");
    expect(needOf(mk("c"))).toBeNull();
  });

  it("each need is one sentence and one action", () => {
    expect(needWords("waiting_user", mk("a"))).toEqual({ what: "Waiting for your reply", action: "Open" });
    expect(needWords("quota_blocked", mk("a")).what).toBe("Work Max hit its limit");
    expect(needWords("quota_blocked", mk("a")).action).toBe("Continue elsewhere");
  });

  it("a place is the project's name and the branch with the account", () => {
    expect(placeOf(mk("a"), PROJECTS)).toEqual({ title: "ai-juggler", detail: "main · Work Max" });
    expect(placeOf(mk("a", { project_id: null, cwd: undefined as never }), []).title).toBe("Work Max");
  });

  it("runway: the tighter measured window wins", () => {
    const display = {
      account: "work",
      five_hour: { percentage: 30, state: "measured", resets_at_ms: null },
      weekly: { percentage: 80, state: "measured", resets_at_ms: null },
    } as unknown as QuotaDisplayData;
    expect(runwayOf(display)).toMatchObject({ left: 20, words: "20% left this week" });
  });

  it("runway: no account reading → the newest session's own reading, then 'Not measured yet'", () => {
    const older = mk("o", { quota: { used_percentage: 10, resets_at_ms: null, confidence: "computed" }, last_activity_ms: NOW - 9_000_000 });
    const newer = mk("n", { quota: { used_percentage: 75, resets_at_ms: null, confidence: "computed" } });
    expect(runwayOf(undefined, [older, newer])).toMatchObject({ left: 25, words: "25% left, from its latest session" });
    expect(runwayOf(undefined, [mk("x")])).toMatchObject({ left: null, words: "Not measured yet" });
  });
});

describe("D-25 Home — the screen", () => {
  const renderHome = (sessions: InstanceInfo[], accounts = ACCOUNTS, props: { onOpenSession?: (id: string) => void; onAddAccounts?: () => void } = {}) =>
    render(
      <HomeScreen
        sessions={sessions}
        accounts={accounts}
        projects={PROJECTS}
        now={NOW}
        onOpenSession={props.onOpenSession ?? (() => null)}
        onNewSession={() => null}
        onAddAccounts={props.onAddAccounts ?? (() => null)}
      />,
    );

  it("leads with how many need you, and each one's action opens it", () => {
    const onOpenSession = vi.fn();
    const { getByTestId } = renderHome(
      [mk("w", { status: "waiting_user" }), mk("f", { status: "failed" }), mk("r")],
      ACCOUNTS,
      { onOpenSession },
    );
    const home = getByTestId("home");
    expect(home.textContent).toContain("2 sessions need you");
    expect(home.textContent).toContain("1 running");
    fireEvent.click(within(home).getByRole("button", { name: /^Open — ai-juggler/ }));
    expect(onOpenSession).toHaveBeenCalledWith("w");
  });

  it("all clear while something runs; ready when nothing does", () => {
    expect(renderHome([mk("r")]).getByTestId("home").textContent).toContain("All clear — nothing needs you");
    cleanup();
    expect(renderHome([]).getByTestId("home").textContent).toContain("Ready when you are");
  });

  it("runway reads each account's measured window", async () => {
    w().__AIJ_QUOTA_DISPLAY__ = [
      { account: "work", five_hour: { percentage: 90, state: "measured", resets_at_ms: null }, weekly: { percentage: 20, state: "measured", resets_at_ms: null } },
    ];
    const { getByTestId } = renderHome([mk("r")]);
    await waitFor(() => expect(getByTestId("home").textContent).toContain("10% left in the 5-hour window"));
    expect(getByTestId("home").textContent).toContain("Not measured yet");
  });

  it("no accounts: one plain next step", () => {
    const onAddAccounts = vi.fn();
    const { getByTestId, getByRole } = renderHome([], [], { onAddAccounts });
    expect(getByTestId("home-empty-accounts").textContent).toContain("Bring in your accounts to start.");
    fireEvent.click(getByRole("button", { name: "Find my accounts" }));
    expect(onAddAccounts).toHaveBeenCalledOnce();
  });
});
