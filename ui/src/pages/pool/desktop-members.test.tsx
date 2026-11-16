/**
 * AJ-E3013 (D-116, D-119): a pool's desktop members on the canvas and in
 * the inspector — connected ("Wakes on new messages", "Reads messages on
 * its next turn"), added and waiting for its next start, Watched with why
 * and Add, a Codex app thread that can't be woken — and Continue in AI
 * Juggler gone from the card, Continue as CLI kept in its menu.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import * as conversations from "../../lib/conversations";
import { forgetSharedReads } from "../../lib/shared-read";
import { CONNECTOR_MEANING, reachOf, wakeLine } from "../../lib/connector";
import { resetPreviewConnector, type DesktopMemberInfo } from "../../lib/ipc/connector";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import { scopeOf, type PoolInfo } from "../../lib/ipc/pools";
import { CONTINUE_WATCHED } from "../../lib/pools";
import { OPEN_ELSEWHERE } from "../../lib/resume-refusal";
import { CONTINUE_AS_CLI } from "./DesktopMemberCard";
import { InspectRule } from "./InspectRule";
import { RuleNodes } from "./RuleNodes";

const POOL = "pool-x";
const WORK = "app:claude-app:/Users/you/Profiles/Work";
const watched = (id: string, title: string) => ({ member: `conversation:claude-work/${id}`, folder: "claude-work", conversation_id: id, title, tool: "Claude app", account_label: null, project_id: null, role: "member", continued_session: null });
const member = (ref: string, over: Partial<DesktopMemberInfo>): DesktopMemberInfo => ({ member: ref, app_id: WORK, connector: "absent", connected: false, wake: "next_turn", ...over });
const seed = (members: DesktopMemberInfo[], connector: Record<string, unknown> = {}) => Object.assign(window, { __AIJ_DESKTOP_MEMBERS__: { [POOL]: members }, __AIJ_CONNECTOR__: connector });
const conversation = (id: string, live: boolean): ConversationInfo => ({ id, tool: "claude-code", account_id: "claude-work", account_label: "Work", project_id: null, cwd: "/w", title: "Fix sign-in copy", git_branch: null, started_ms: 0, last_activity_ms: 0, tokens: null, live, session_id: null, places: [], pid: null, state: null, probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null });

beforeEach(() => {
  forgetSharedReads();
  resetPreviewConnector();
  Object.assign(window, { __AIJ_DESKTOP_MEMBERS__: undefined, __AIJ_CONNECTOR__: undefined, __AIJ_CONNECTOR_CALLS__: undefined });
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const nodes = (rows: ReturnType<typeof watched>[], onSelect = () => undefined) => render(<RuleNodes poolId={POOL} rules={[]} watched={rows} labelOf={() => ""} selection={{ kind: "none" }} onSelect={onSelect} />);

describe("AJ-E3013: a session another app runs, on the canvas", () => {
  it("Watched without a connector says why, with Add, which opens it in the inspector", async () => {
    seed([member("conversation:claude-work/c1", {})]);
    const onSelect = vi.fn();
    nodes([watched("c1", "Fix sign-in copy")], onSelect);
    await waitFor(() => expect(screen.getByTestId("canvas-watched-reach").textContent).toBe("Claude app · Its app has no AI Juggler connector"));
    const card = screen.getByTestId("canvas-watched-node");
    expect(card.textContent).toContain("Watched");
    expect(card.textContent).not.toContain("can't reply");
    fireEvent.click(screen.getByTestId("canvas-watched-add"));
    expect(onSelect).toHaveBeenCalledWith({ kind: "rule", ref: "conversation:claude-work/c1" });
  });

  it.each([
    ["wakes", "Wakes on new messages"],
    ["next_turn", "Reads messages on its next turn"],
  ])("connected, it's a member (%s): \"%s\", never Watched", async (wake, line) => {
    seed([member("conversation:claude-work/c1", { connector: "current", connected: true, wake })], { [WORK]: { state: "current" } });
    nodes([watched("c1", "Fix sign-in copy")]);
    await waitFor(() => expect(screen.getByTestId("canvas-watched-reach").textContent).toBe(`Claude app · ${line}`));
    expect(screen.getByTestId("canvas-watched-node").textContent).not.toContain("Watched");
    expect(screen.queryByTestId("canvas-watched-add")).toBeNull();
  });

  it("added, not called yet: it joins at its next start", async () => {
    seed([member("conversation:claude-work/c1", { connector: "current" })], { [WORK]: { state: "current" } });
    nodes([watched("c1", "Fix sign-in copy")]);
    await waitFor(() => expect(screen.getByTestId("canvas-watched-reach").textContent).toBe("Claude app · Added · joins at its next start · reads messages on its next turn"));
  });

  it("a Codex app thread can't take the connector and can't be woken, and says so", async () => {
    seed([{ member: "conversation:codex/t1", app_id: "app:codex-app:", connector: "unavailable", connected: false, wake: "cant_wake" }]);
    nodes([{ ...watched("t1", "Landing page copy"), member: "conversation:codex/t1", folder: "codex", tool: "Codex" }]);
    await waitFor(() => expect(screen.getByTestId("canvas-watched-reach").textContent).toBe("Codex · Its app can't take AI Juggler's connector yet; it can't be woken"));
    expect(screen.queryByTestId("canvas-watched-add")).toBeNull();
    expect(wakeLine("cant_wake")).toBe("Can't be woken: it reads messages when you next use it");
  });

  it("Continue in AI Juggler is gone from the card; Continue as CLI is in its menu, saying what it does, greyed with why while open elsewhere", async () => {
    seed([member("conversation:claude-work/c1", {})]);
    vi.spyOn(conversations, "useConversations").mockReturnValue({ all: [conversation("c1", true)], refresh: () => {}, loaded: true, build: null, buildAgain: () => {} });
    nodes([watched("c1", "Fix sign-in copy")]);
    expect(screen.queryByText("Continue in AI Juggler")).toBeNull();
    fireEvent.click(screen.getByTestId("canvas-watched-more"));
    const item = await screen.findByTestId("canvas-watched-continue");
    expect(item.textContent).toContain(CONTINUE_AS_CLI);
    expect(item.textContent).toContain(OPEN_ELSEWHERE);
    expect(item.hasAttribute("disabled") || item.getAttribute("aria-disabled") === "true").toBe(true);
    cleanup();
    vi.spyOn(conversations, "useConversations").mockReturnValue({ all: [conversation("c1", false)], refresh: () => {}, loaded: true, build: null, buildAgain: () => {} });
    nodes([watched("c1", "Fix sign-in copy")]);
    fireEvent.click(screen.getByTestId("canvas-watched-more"));
    expect((await screen.findByTestId("canvas-watched-continue")).textContent).toContain(CONTINUE_WATCHED);
  });

  it("a desktop App member's card says its sessions' reach, Watched with Add without the connector", async () => {
    const ref = `app:${WORK}`;
    seed([member(ref, {})]);
    render(<RuleNodes poolId={POOL} rules={[{ member: ref, kind: "app", role: "member", level: null, joined_at: 1, sessions: 0, session_ids: [] }]} watched={[]} labelOf={() => "Claude · Work Max"} selection={{ kind: "none" }} onSelect={() => undefined} />);
    await waitFor(() => expect(screen.getByTestId("canvas-rule-wake").textContent).toBe("Watched: its app has no AI Juggler connector"));
    expect(screen.getByTestId("canvas-rule-connector-add").getAttribute("aria-label")).toBe("Add AI Juggler's connector to Claude · Work Max");
  });
});

describe("AJ-E3013: the inspector", () => {
  const pool: PoolInfo = { id: POOL, name: "web team", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0, ...scopeOf("user", null) };

  it("Watched: why, the consent card, and no Continue button", async () => {
    seed([member("conversation:claude-work/c1", {})]);
    render(<InspectRule pool={pool} rule={undefined} watched={watched("c1", "Fix sign-in copy")} label="Fix sign-in copy" onChanged={() => undefined} onGone={() => undefined} />);
    await waitFor(() => expect(screen.getByTestId("inspect-rule-reach").textContent).toBe(`Watched: its app has no AI Juggler connector (${CONNECTOR_MEANING}). It is shown and counted here, without the pool's tools.`));
    expect(await screen.findByTestId("inspect-connector")).toBeTruthy();
    expect(screen.queryByTestId("inspect-rule-continue")).toBeNull();
  });

  it("connected: how a message reaches it, and where the connector stands with Remove", async () => {
    seed([member("conversation:claude-work/c1", { connector: "current", connected: true, wake: "wakes" })], { [WORK]: { state: "current" } });
    render(<InspectRule pool={pool} rule={undefined} watched={watched("c1", "Fix sign-in copy")} label="Fix sign-in copy" onChanged={() => undefined} onGone={() => undefined} />);
    await waitFor(() => expect(screen.getByTestId("inspect-rule-reach").textContent).toBe("It talks in this pool through AI Juggler's connector. Wakes on new messages."));
    expect((await screen.findByTestId("inspect-connector-there")).textContent).toContain("AI Juggler's connector is in this app.");
  });

  it("Add here reads the members again: the card turns to added", async () => {
    seed([member("conversation:claude-work/c1", {})]);
    render(<InspectRule pool={pool} rule={undefined} watched={watched("c1", "Fix sign-in copy")} label="Fix sign-in copy" onChanged={() => undefined} onGone={() => undefined} />);
    await act(async () => fireEvent.click(await screen.findByTestId("inspect-connector-add")));
    await act(async () => fireEvent.click(await screen.findByTestId("inspect-connector-confirm")));
    await waitFor(() => expect(screen.getByTestId("inspect-rule-reach").textContent).toContain("joins at the next start"));
    expect(reachOf(member("x", { connector: "outdated" })).kind).toBe("added");
  });
});

// The aij-test proof: after Remove, the App's card and inspector still said
// "Its sessions talk … through the connector. Wakes on new messages".
describe("AJ-E3013: the words follow the connector", () => {
  const pool: PoolInfo = { id: POOL, name: "web team", kind: "user", project_id: null, access: { level: "full", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0, ...scopeOf("user", null) };
  const ref = `app:${WORK}`;
  const appRule = { member: ref, kind: "app", role: "member", level: null, joined_at: 1, sessions: 1, session_ids: [] };

  it("after Remove, the App's card and inspector say Watched with the reason, though a session called lately", async () => {
    seed([member(ref, { connector: "current", connected: true, wake: "wakes" })], { [WORK]: { state: "current" } });
    render(
      <>
        <RuleNodes poolId={POOL} rules={[appRule]} watched={[]} labelOf={() => "Claude app · Work Max"} selection={{ kind: "none" }} onSelect={() => undefined} />
        <InspectRule pool={pool} rule={appRule} watched={undefined} label="Claude app · Work Max" onChanged={() => undefined} onGone={() => undefined} />
      </>,
    );
    await waitFor(() => expect(screen.getByTestId("canvas-rule-wake").textContent).toBe("Its sessions: wakes on new messages"));
    expect(screen.getByTestId("inspect-rule-reach").textContent).toBe("Its sessions talk in this pool through AI Juggler's connector. Wakes on new messages.");
    await act(async () => fireEvent.click(await screen.findByTestId("inspect-connector-remove")));
    await act(async () => fireEvent.click(screen.getByTestId("inspect-connector-confirm-remove")));
    await waitFor(() => expect(screen.getByTestId("canvas-rule-wake").textContent).toBe("Watched: its app has no AI Juggler connector"));
    expect(screen.getByTestId("inspect-rule-reach").textContent).toMatch(/^Watched: its app has no AI Juggler connector/);
    expect(screen.getByTestId("inspect-rule-reach").textContent).not.toContain("Wakes on new messages");
  });

  it("a session that calls with no connector in its app is still Watched; changed says so", () => {
    expect(reachOf(member("x", { connector: "absent", connected: true, wake: "wakes" }))).toEqual({ kind: "watched", line: "Its app has no AI Juggler connector", fix: "add" });
    expect(reachOf(member("x", { connector: "changed", connected: true })).line).toBe("Its app's AI Juggler connector was changed");
  });
});

// f1b9bd354: the desktop sessions an App brings in, as session nodes under
// its card.
describe("AJ-E3013: desktop sessions under their App", () => {
  it("each with its name, role and how a message reaches it; a fresh call shows a dot", async () => {
    const ref = `app:${WORK}`;
    Object.assign(window, {
      __AIJ_DESKTOP_SESSIONS__: {
        [POOL]: [
          { member: "conversation:claude-work/c1", session_id: "desktop:claude-work/c1", name: "Fix sign-in copy", place: "Claude app · Work", tool: "Claude Code", account: "Work", role: "member", app_id: WORK, via: "app", wake: "wakes" },
          { member: "conversation:claude-work/c2", session_id: null, name: "Tidy the tests", place: "Claude app · Work", tool: "Claude Code", account: "Work", role: "reviewer", app_id: WORK, via: "app", wake: "next_turn" },
        ],
      },
    });
    seed([member(ref, { connector: "current", connected: true })], { [WORK]: { state: "current" } });
    render(<RuleNodes poolId={POOL} rules={[{ member: ref, kind: "app", role: "member", level: null, joined_at: 1, sessions: 2, session_ids: [] }]} watched={[]} labelOf={() => "Claude app · Work Max"} selection={{ kind: "none" }} onSelect={() => undefined} />);
    const rows = await screen.findAllByTestId("canvas-desktop-session");
    expect(rows.map((row) => row.textContent)).toEqual(["Fix sign-in copyMemberWakes on new messages", "Tidy the testsReviewerReads messages on its next turn"]);
  });
});

// D-119 (5): a person's own Deliver wakes a desktop member whatever the
// pool's level; the inspector says it as Messages' Deliver does.
describe("D-119: Deliver wakes a desktop member at any level", () => {
  it("a member that reads on its next turn says Deliver wakes it; one that wakes says nothing more", async () => {
    const { deliverWakes } = await import("../../lib/connector");
    seed([member("conversation:claude-work/c1", { connector: "current", connected: true, wake: "next_turn" })], { [WORK]: { state: "current" } });
    const pool: PoolInfo = { id: POOL, name: "web team", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0, ...scopeOf("user", null) };
    render(<InspectRule pool={pool} rule={undefined} watched={watched("c1", "Fix sign-in copy")} label="Fix sign-in copy" onChanged={() => undefined} onGone={() => undefined} />);
    await waitFor(() => expect(screen.getByTestId("inspect-rule-reach").textContent).toBe("It talks in this pool through AI Juggler's connector. Reads messages on its next turn. A message you send with Deliver wakes it, whatever the pool's level."));
    expect(deliverWakes("next_turn", false)).toBe(" A message you send with Deliver wakes it, whatever the pool's level.");
    expect(deliverWakes("next_turn", true)).toBe(" A message you send with Deliver wakes them, whatever the pool's level.");
    expect(deliverWakes("wakes", true)).toBe("");
    expect(deliverWakes("cant_wake", false)).toBe("");
  });
});
