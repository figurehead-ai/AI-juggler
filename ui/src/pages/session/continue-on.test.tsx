/**
 * AJ-E2108 vitest — Continue on another account offers the same targets
 * for every session: one another app runs (the Claude app, Codex in the
 * ChatGPT app) as well as one AI Juggler runs. Every signed-in profile of
 * every tool, grouped by account, the session's own account apart; each
 * can be picked in one click while the original stays open in its app.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../../lib/bridge";
import * as setup from "../../lib/ipc/switchSetup";
import { switchSourceOf, type ConversationInfo } from "../../lib/ipc/conversations";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { ContinueOn } from "./ContinueOn";
import { fromConversation, fromInstance, type SessionModel } from "./model";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const account = (key: string, tool: string, ids: string[], plan: string): SubscriptionInfo => ({
  key,
  tool,
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  profiles: ids.map((id) => ({ id, folder: `~/p/${id}`, plan: null, renewal_date: null, fee_usd: null })),
  plan,
  renewal_date: null,
  fee_usd: null,
  differs: [],
});

// Two Claude accounts (Work has two profiles) and a Codex one.
const ACCOUNTS = [
  account("claude-work", "claude-code", ["work", "work-2"], "Max"),
  account("claude-home", "claude-code", ["home"], "Pro"),
  account("codex-personal", "codex", ["codex"], "Plus"),
];

const target = (account_id: string, account_label: string, provider: string, cross: boolean, extra: Partial<bridge.SwitchTargetData> = {}): bridge.SwitchTargetData => ({
  recipe_id: `recipe-${account_id}`,
  provider,
  account_id,
  account_label,
  badge: cross ? "Full story, fresh start · Beta" : "Same conversation, new account · Beta",
  validated: false,
  cross_provider: cross,
  drops: [],
  score: null,
  filtered_reason: null,
  top_factors: [],
  factors: [],
  confidence: "Measured",
  tte_min: null,
  suggested: false,
  ...extra,
});

const conversation = (overrides: Partial<ConversationInfo>): ConversationInfo => ({
  id: "c-1",
  tool: "claude-code",
  account_id: "work",
  account_label: "Work",
  project_id: null,
  cwd: "/w/app",
  title: "Fix the footer",
  git_branch: "main",
  started_ms: 0,
  last_activity_ms: 0,
  tokens: null,
  live: true,
  session_id: null,
  places: [{ app: "Claude", app_id: "claude", profile: "Work", instance: null, pid: 7, kind: "desktop", app_pid: 6, label: "Claude" }],
  pid: 7,
  state: "idle",
  probably: false,
  viewed_ms: null,
  background: false,
  inactive: false,
  subtask_count: null,
  ...overrides,
});

let asked: string[] = [];

beforeEach(() => {
  asked = [];
  vi.spyOn(setup, "switchSetupCheck").mockImplementation(async (source) => {
    asked.push(source);
    return { from: "Work", to: "x", already_there: 2, offers: [] };
  });
});

function withTargets(rows: bridge.SwitchTargetData[]) {
  return vi.spyOn(bridge, "switchTargets").mockResolvedValue(rows);
}

function show(model: SessionModel, onPick = vi.fn(), onCarryIn?: () => void) {
  render(<ContinueOn model={model} accounts={ACCOUNTS} displays={[]} onPick={onPick} onCarryIn={onCarryIn} />);
  return onPick;
}

describe("AJ-E2108 Continue on another account", () => {
  it("a Claude-app session offers every tool's profiles, each one click away while it stays open there", async () => {
    const targets = withTargets([
      target("work-2", "Work 2", "claude-code", false),
      target("home", "Home", "claude-code", false),
      target("codex", "Codex Personal", "codex", true),
    ]);
    const model = fromConversation(conversation({}));
    const onPick = show(model);
    await waitFor(() => expect(screen.getAllByTestId("continue-target").length).toBe(2));
    expect(targets).toHaveBeenCalledWith("conversation:work:c-1");

    const rows = screen.getAllByTestId("continue-target") as HTMLButtonElement[];
    expect(rows.every((row) => !row.disabled)).toBe(true);
    const codex = rows.find((row) => row.dataset.crossTool === "true");
    expect(codex?.textContent).toContain("Codex Personal");
    // The move's badge sits under the other tool's row.
    const badge = screen.getByTestId("fidelity-badge");
    expect(badge.textContent).toContain("Full story, fresh start");
    // Its own account's other profile gives no new room.
    expect(screen.getByTestId("continue-no-room").textContent).toContain("Work 2");

    fireEvent.click(codex!);
    expect(onPick).toHaveBeenCalledWith("recipe-codex", "Codex Personal");

    const outside = screen.getByTestId("continue-outside");
    expect(outside.textContent).toContain("The original stays open in the Claude app, untouched.");
    expect(within(outside).getByTestId("continue-same-profile").textContent).toContain("close it in the Claude app first");
    expect(screen.queryByTestId("continue-carry-in")).toBeNull();
    await waitFor(() => expect(asked.length).toBe(2));
    expect(new Set(asked)).toEqual(new Set(["conversation:work:c-1"]));
  });

  it("a Codex session in the ChatGPT app offers the Claude profiles", async () => {
    withTargets([target("work", "Work", "claude-code", true), target("home", "Home", "claude-code", true)]);
    const model = fromConversation(
      conversation({
        tool: "codex",
        account_id: "codex",
        account_label: "Codex Personal",
        places: [{ app: "ChatGPT", app_id: "chatgpt", profile: null, instance: null, pid: 9, kind: "desktop", app_pid: 8, label: "ChatGPT" }],
      }),
    );
    const onPick = show(model);
    await waitFor(() => expect(screen.getAllByTestId("continue-target").length).toBe(2));
    expect(screen.queryByTestId("continue-none")).toBeNull();
    const home = screen.getAllByTestId("continue-target").find((row) => row.textContent?.includes("Home")) as HTMLButtonElement;
    expect(home.disabled).toBe(false);
    fireEvent.click(home);
    expect(onPick).toHaveBeenCalledWith("recipe-home", "Home");
  });

  it("a session another app ran, closed now, can also be continued on its own profile", async () => {
    withTargets([target("home", "Home", "claude-code", false)]);
    const carryIn = vi.fn();
    show(fromConversation(conversation({ live: false, places: [], pid: null, state: null })), vi.fn(), carryIn);
    await waitFor(() => expect(screen.getAllByTestId("continue-target").length).toBe(1));
    expect(screen.getByTestId("continue-outside").textContent).toContain("The original stays as it is.");
    fireEvent.click(screen.getByTestId("continue-carry-in"));
    expect(carryIn).toHaveBeenCalledTimes(1);
  });

  it("a session AI Juggler runs reads its targets by its id, as before", async () => {
    const targets = withTargets([target("work", "Work", "claude-code", false), target("codex", "Codex Personal", "codex", true)]);
    const session = { id: "sess-1", provider: "claude-code", account_id: "home", account_label: "Home", conversation_id: "c-9", status: "running", cwd: "/w/app" } as unknown as bridge.InstanceInfo;
    const model = fromInstance(session);
    expect(model.switchSource).toBe("sess-1");
    const onPick = show(model);
    await waitFor(() => expect(screen.getAllByTestId("continue-target").length).toBe(2));
    expect(targets).toHaveBeenCalledWith("sess-1");
    expect(screen.queryByTestId("continue-outside")).toBeNull();
    fireEvent.click(screen.getAllByTestId("continue-target")[0]!);
    expect(onPick).toHaveBeenCalledWith("recipe-work", "Work");
  });

  it("a target the ranking holds back says why and can't be picked", async () => {
    withTargets([target("codex", "Codex Personal", "codex", true, { filtered_reason: "Out of room until 3 pm" })]);
    show(fromConversation(conversation({})));
    const row = (await screen.findByTestId("continue-target")) as HTMLButtonElement;
    expect(row.disabled).toBe(true);
    expect(screen.getByTestId("continue-check").textContent).toContain("Out of room until 3 pm");
  });

  it("the key a switch takes: AI Juggler's session, else the conversation's own", () => {
    expect(switchSourceOf({ session_id: null, account_id: "work", id: "c-1" })).toBe("conversation:work:c-1");
    expect(switchSourceOf({ session_id: "sess-2", account_id: "work", id: "c-1" })).toBe("sess-2");
  });
});
