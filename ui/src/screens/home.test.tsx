/**
 * D-25 vitest — Home: what needs you (one sentence, one action), what's
 * running, each Account's runway (measured, or a session's own reading,
 * or "Not measured yet"), and the plain next step when nothing is set up.
 * AJ-E1021 (D-25 amendment): Home is also the morning triage — parked
 * sessions, what ended and isn't reviewed, J/K/Enter/A (from AJ-E1005).
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeScreen, needOf, needWords, placeOf, runwayOf, toReview } from "./Home";
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
  { id: "proj-1", root: "/w/ai-juggler", name: "ai-juggler", last_activity_ms: null, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
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

  it("an account that needs its sign-in is a Needs you item; Sign in opens the login terminal", async () => {
    const opened: string[] = [];
    const accounts = [{ ...ACCOUNTS[0]!, login_state: "logged_out" }, ACCOUNTS[1]!] as AccountInfo[];
    const { getByTestId } = render(
      <HomeScreen
        sessions={[]}
        accounts={accounts}
        projects={PROJECTS}
        now={NOW}
        onOpenSession={() => null}
        onNewSession={() => null}
        onAddAccounts={() => null}
        openLoginTerminal={(_id, title) => opened.push(title)}
      />,
    );
    expect(getByTestId("home").textContent).toContain("Work Max needs its sign-in");
    const item = getByTestId("home-signin");
    fireEvent.click(within(item).getByRole("button", { name: /^Sign in/ }));
    await waitFor(() => expect(opened).toEqual(["Sign in · Work Max"]));
    expect(item.textContent).toContain("Finish signing in in the terminal that opened");
  });

  it("a Codex account's sign-in names Codex's own sign-in (OpenAI)", () => {
    const accounts = [{ ...ACCOUNTS[0]!, provider: "codex", login_state: "logged_out" }] as AccountInfo[];
    const { getByTestId } = render(
      <HomeScreen
        sessions={[]}
        accounts={accounts}
        projects={PROJECTS}
        now={NOW}
        onOpenSession={() => null}
        onNewSession={() => null}
        onAddAccounts={() => null}
        openLoginTerminal={() => null}
      />,
    );
    const item = getByTestId("home-signin");
    expect(item.textContent).toContain("Codex's own sign-in (OpenAI)");
    expect(item.textContent).not.toContain("Claude's own sign-in");
  });

  // AJ-E1524: no accounts is the welcome (welcome.test.tsx holds its own
  // tests); Find my accounts still opens first run.
  it("no accounts: the welcome, with Find my accounts first", () => {
    const onAddAccounts = vi.fn();
    const { getByTestId, getByRole } = renderHome([], [], { onAddAccounts });
    expect(getByRole("heading", { level: 1 }).textContent).toBe("All your AI coding accounts, in one calm place.");
    // In place is the default, and not only Claude Code (0.0.11 walk).
    expect(getByTestId("home-empty-accounts").textContent).toContain("used where they are");
    fireEvent.click(getByRole("button", { name: "Find my accounts" }));
    expect(onAddAccounts).toHaveBeenCalledOnce();
  });
});

describe("AJ-E1021 Home as the morning triage (from AJ-E1005)", () => {
  const calls = () => [
    ...((w().__AIJ_TRIAGE_CALLS__ as string[] | undefined) ?? []),
    ...((w().__AIJ_PARK_CALLS__ as string[] | undefined) ?? []),
  ];
  const renderHome = (sessions: InstanceInfo[], props: { onOpenSession?: (id: string) => void; onChanged?: () => void } = {}) =>
    render(
      <HomeScreen
        sessions={sessions}
        accounts={ACCOUNTS}
        projects={PROJECTS}
        now={NOW}
        onOpenSession={props.onOpenSession ?? (() => null)}
        onNewSession={() => null}
        onAddAccounts={() => null}
        onChanged={props.onChanged}
      />,
    );

  it("to review: ended this week and not reviewed or archived", () => {
    expect(toReview(mk("a", { status: "finished" }), NOW)).toBe(true);
    expect(toReview(mk("b", { status: "stopped" }), NOW)).toBe(true);
    expect(toReview(mk("c", { status: "finished", lifecycle: "reviewed" }), NOW)).toBe(false);
    expect(toReview(mk("d", { status: "stopped", lifecycle: "archived" }), NOW)).toBe(false);
    expect(toReview(mk("e", { status: "finished", last_activity_ms: NOW - 8 * 24 * 3_600_000 }), NOW)).toBe(false);
    expect(toReview(mk("f", { status: "running" }), NOW)).toBe(false);
    // It goes on in another session; that one is what to pick up.
    expect(toReview(mk("g", { status: "handed_over" }), NOW)).toBe(false);
  });

  it("a parked session waits in its own section, not under Needs you; at the reset it offers Resume", async () => {
    let changed = 0;
    const { getByTestId, queryByTestId, getByRole } = renderHome(
      [
        mk("p-later", { status: "quota_blocked", parked: "measured", parked_resets_at_ms: NOW + 2 * 3_600_000 + 13 * 60_000 }),
        mk("p-due", { status: "quota_blocked", parked: "measured", parked_resets_at_ms: NOW - 1 }),
      ],
      { onChanged: () => changed++ },
    );
    expect(queryByTestId("home-needs")).toBeNull();
    const parked = getByTestId("home-parked");
    expect(parked.textContent).toContain("Parked until a reset");
    expect(parked.textContent).toContain("resets in 2h 13m");
    expect(parked.textContent).toContain("reset reached");
    await act(async () => {
      fireEvent.click(getByRole("button", { name: /^Resume/ }));
    });
    expect(calls()).toEqual(["resume:p-due"]);
    expect(changed).toBe(1);
  });

  it("pick up where you left off: three, then Show all; Mark reviewed takes one off", async () => {
    let changed = 0;
    const ended = Array.from({ length: 5 }, (_, n) => mk(`e-${n}`, { status: "finished", last_activity_ms: NOW - (n + 1) * 60_000 }));
    const { getByTestId, getAllByTestId, getAllByRole } = renderHome(
      [...ended, mk("old", { status: "finished", lifecycle: "reviewed" })],
      { onChanged: () => changed++ },
    );
    expect(getAllByTestId("home-recent-item")).toHaveLength(3);
    fireEvent.click(getByTestId("home-recent-all"));
    expect(getAllByTestId("home-recent-item")).toHaveLength(5);
    await act(async () => {
      fireEvent.click(getAllByRole("button", { name: /^Mark reviewed/ })[0]!);
    });
    expect(calls()).toEqual(["reviewed:e-0"]);
    expect(changed).toBe(1);
  });

  it("is keyboard operable: J/K or arrows move, A runs the row's action, Enter opens", async () => {
    const user = userEvent.setup();
    const opened: string[] = [];
    // The parent refreshes after an action: a reviewed session leaves the list.
    function Stateful() {
      const [rows, setRows] = useState([
        mk("w", { status: "waiting_user" }),
        mk("r"),
        mk("d1", { status: "finished", last_activity_ms: NOW - 60_000 }),
        mk("d2", { status: "finished", last_activity_ms: NOW - 120_000 }),
      ]);
      return (
        <HomeScreen
          sessions={rows}
          accounts={ACCOUNTS}
          projects={PROJECTS}
          now={NOW}
          onOpenSession={(id) => opened.push(id)}
          onNewSession={() => null}
          onAddAccounts={() => null}
          onChanged={() => setRows((current) => current.filter((row) => !calls().some((call) => call === `reviewed:${row.id}`)))}
        />
      );
    }
    const { container } = render(<Stateful />);
    const rows = () => Array.from(container.querySelectorAll<HTMLElement>("[data-home-row]"));
    expect(rows()).toHaveLength(4);
    rows()[0]!.focus();
    await user.keyboard("j");
    expect(document.activeElement).toBe(rows()[1]);
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(rows()[2]);
    // A marks d1 reviewed; its row disappears; focus lands on the row now
    // in its place (d2), not on <body>, and the keys go on working.
    await user.keyboard("a");
    await waitFor(() => expect(rows()).toHaveLength(3));
    expect(calls()).toEqual(["reviewed:d1"]);
    await waitFor(() => expect(document.activeElement).toBe(rows()[2]));
    await user.keyboard("k");
    expect(document.activeElement).toBe(rows()[1]);
    await user.keyboard("k");
    await user.keyboard("{Enter}");
    expect(opened).toEqual(["w"]);
    expect(container.querySelector("[data-testid='home-keys']")!.textContent).toContain("run the row's action");
  });

  it("A on a row's own Mark reviewed button acts on that row; J moves on from it", async () => {
    const user = userEvent.setup();
    const { container, getAllByRole } = renderHome([mk("d1", { status: "finished" }), mk("d2", { status: "finished", last_activity_ms: NOW - 120_000 })]);
    const markFirst = getAllByRole("button", { name: /^Mark reviewed/ })[0]!;
    markFirst.focus();
    await user.keyboard("a");
    await waitFor(() => expect(calls()).toEqual(["reviewed:d1"]));
    markFirst.focus();
    await user.keyboard("j");
    expect(document.activeElement).toBe(container.querySelectorAll<HTMLElement>("[data-home-row]")[1]);
  });

  it("keys still work after focus fell to <body> while Home is shown", async () => {
    const user = userEvent.setup();
    const { container } = renderHome([mk("w", { status: "waiting_user" }), mk("r")]);
    (document.activeElement as HTMLElement | null)?.blur();
    expect(document.activeElement).toBe(document.body);
    await user.keyboard("j");
    expect(document.activeElement).toBe(container.querySelectorAll<HTMLElement>("[data-home-row]")[0]);
  });

  it("a dialog over Home owns the keys, even with focus on <body>", async () => {
    const user = userEvent.setup();
    const { container } = renderHome([mk("d1", { status: "finished" }), mk("d2", { status: "finished", last_activity_ms: NOW - 120_000 })]);
    const modal = document.createElement("div");
    modal.setAttribute("role", "dialog");
    modal.setAttribute("aria-modal", "true");
    document.body.appendChild(modal);
    try {
      (document.activeElement as HTMLElement | null)?.blur();
      await user.keyboard("j");
      expect(document.activeElement).toBe(document.body);
      // A reaches no row behind the dialog either.
      container.querySelectorAll<HTMLElement>("[data-home-row]")[0]!.focus();
      (document.activeElement as HTMLElement).blur();
      await user.keyboard("a");
      expect(calls()).toEqual([]);
    } finally {
      modal.remove();
    }
    // Gone, the keys work again.
    await user.keyboard("j");
    expect(document.activeElement).toBe(container.querySelectorAll<HTMLElement>("[data-home-row]")[0]);
  });

  it("a key another handler already took isn't run again from <body>", () => {
    const { container } = renderHome([mk("w", { status: "waiting_user" })]);
    (document.activeElement as HTMLElement | null)?.blur();
    const event = new KeyboardEvent("keydown", { key: "j", bubbles: true, cancelable: true });
    event.preventDefault();
    window.dispatchEvent(event);
    expect(document.activeElement).toBe(document.body);
    expect(container.querySelectorAll("[data-home-row]").length).toBe(1);
  });

  it("renders 50 sessions well inside 300 ms", () => {
    const statuses = ["waiting_user", "stuck", "running", "finished", "quota_blocked"];
    const sessions = Array.from({ length: 50 }, (_, n) => mk(`s-${n}`, { status: statuses[n % 5]! }));
    const started = performance.now();
    renderHome(sessions);
    expect(performance.now() - started).toBeLessThan(300);
  });
});
