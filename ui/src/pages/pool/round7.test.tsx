/**
 * AJ-E3011 round 7 (after the round-6 done-gate walks): the ready line
 * names each member as its card does where two share a tool's name; the
 * connector asked for on Who's in's own rows; the roles step's title says
 * what's chosen there; Show the files opens on an accessibility press.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CONNECTOR } from "../../lib/connector";
import { resetPreviewConnector } from "../../lib/ipc/connector";
import { poolJoin, type MemberKind } from "../../lib/ipc/pool-model";
import { setPools } from "../../lib/pools";
import { previewPoolsInfo, resetPreviewPools } from "../../lib/ipc/pools";
import { draftOf, type Candidate, type Draft } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { firstPrompt } from "../../lib/pool-sentences";
import { STEP_WORDS } from "../../lib/pool-words";
import { forgetSharedReads } from "../../lib/shared-read";
import { NewPoolWho } from "../../screens/pools/NewPoolWho";
import { ConnectorCard } from "./connector/ConnectorCard";
import { RuleNodes } from "./RuleNodes";
import { SENTENCE_TITLES } from "./setup/PoolSentences";

describe("round 7 #1: the ready line names the member, not the tool", () => {
  it("two profiles of the Claude app: the reviewer by its whole name; a tool alone keeps its short name", () => {
    const member = (ref: string, label: string, role: "member" | "reviewer") => ({ ref, label, role, level: null, later: true });
    const two = { members: [member("app:claude-app:work", "Claude app · Work Max", "member"), member("app:claude-app:side", "Claude app · Side", "reviewer")], leadNext: "app:claude-app:work" };
    expect(firstPrompt(two, "web")).toEqual({ to: "Claude app · Work Max", words: "Plan the work in web, hand out tasks, and ask Claude app · Side to review." });
    const mixed = { members: [member("app:claude-app:work", "Claude app · Work Max", "member"), member("app:cli:codex", "Codex · Personal", "reviewer")], leadNext: "app:claude-app:work" };
    expect(firstPrompt(mixed, "web")!.words).toBe("Plan the work in web, hand out tasks, and ask Codex to review.");
  });
});

// #2 (Casey's worst moment, the round-6 done-gate): Who's in asks for the
// connector on the desktop App's own row, before Next.
describe("round 7 #2: the connector on Who's in's own row", () => {
  const WORK = "app:claude-app:/Users/you/Profiles/Work";
  const place = { id: WORK, app_id: "claude-app", app: "Claude", instance: "Work", profile: "Work Max", label: "Claude · Work Max", account_id: "claude-work", app_pid: null, lost: false, keep_alive: false, paused: null, seen_ms: 0, cannot_start: null, recent: [], found: false };
  beforeEach(() => {
    forgetSharedReads();
    resetPreviewConnector();
    Object.assign(window, { __AIJ_PLACES__: { places: [place] }, __AIJ_CONNECTOR__: undefined, __AIJ_CONNECTOR_CALLS__: undefined });
  });
  afterEach(() => {
    cleanup();
    Object.assign(window, { __AIJ_PLACES__: undefined });
  });

  it("checked, its row says Watched: can't talk in the pool yet, with Add; Add is the consent card's own preview, then added", async () => {
    const row: Candidate = { ref: `app:${WORK}`, kind: "app", label: "Claude app · Work Max", detail: "none open now", group: "all", reach: "later", blocked: null, note: "Runs in the Claude app: …", lastMs: 0 };
    const candidates = { session: [], app: [row], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    let draft: Draft = { ...draftOf(templateById("blank"), null), members: [] };
    const view = render(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} onFix={() => undefined} />);
    fireEvent.click(screen.getByRole("radio", { name: /Apps/ }));
    // Not in yet: what joining means, said once.
    expect(screen.getByTestId("new-pool-candidate-note")).toBeTruthy();
    fireEvent.click(within(screen.getByTestId("new-pool-candidate")).getByRole("checkbox"));
    view.rerender(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} onFix={() => undefined} />);
    const state = await screen.findByTestId("new-pool-connector-card");
    expect(within(screen.getByTestId("new-pool-candidate")).getByTestId("new-pool-connector-card")).toBe(state);
    expect(state.getAttribute("data-state")).toBe("absent");
    expect(state.textContent).toContain("Watched: can't talk in the pool yet.");
    expect(state.textContent).toContain("AI Juggler's connector (a small helper AI Juggler adds to an app");
    expect(screen.queryByTestId("new-pool-candidate-note")).toBeNull();
    await act(async () => fireEvent.click(screen.getByTestId("new-pool-connector-card-add")));
    expect(screen.getByTestId("new-pool-connector-card-preview-sentence").textContent).toContain("This adds a small helper to your Claude app (Work Max)");
    await act(async () => fireEvent.click(screen.getByTestId("new-pool-connector-card-confirm")));
    expect(screen.getByTestId("new-pool-connector-card-added").textContent).toBe(CONNECTOR.added);
  });

  it("with the connector in it, the row says it can talk; one that can't take it says why, with no Add", async () => {
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "current" } } });
    render(<ConnectorCard compact appId={WORK} app="Claude app · Work Max" testId="row" />);
    expect((await screen.findByTestId("row")).textContent).toBe("Can talk in the pool: AI Juggler's connector is in it.");
    cleanup();
    forgetSharedReads();
    render(<ConnectorCard compact appId="app:codex-app:" app="ChatGPT" testId="row" />);
    expect((await screen.findByTestId("row-why")).textContent).toBe("AI Juggler can't add its connector to this app yet: it can't tell this app's sessions apart.");
    expect(screen.queryByTestId("row-add")).toBeNull();
  });
});

// #3 (Sam's walk): the roles step's title says what's chosen there.
describe("round 7 #3: the roles step's title", () => {
  it("reads Who builds, who checks, in the setup's steps and the pool's own sections", () => {
    expect(STEP_WORDS.everyone).toEqual({ title: "Who builds, who checks", nav: "Who builds, who checks", why: "Each member's role, and how new sessions join." });
    expect(SENTENCE_TITLES.everyone).toBe("Who builds, who checks");
  });
});

// #4 (the aij-test driver pressed Show the files 4 times): the app's
// Disclosure is a button with aria-expanded, so an accessibility press
// (a button's own action) opens it; keyboard and mouse still do.
describe("round 7 #4: Show the files opens on an accessibility press", () => {
  beforeEach(() => {
    forgetSharedReads();
    resetPreviewConnector();
    Object.assign(window, { __AIJ_CONNECTOR__: undefined });
  });
  afterEach(cleanup);

  it("Show the files and each file's text are buttons with aria-expanded, opened by a press", async () => {
    render(<ConnectorCard appId="app:claude-app:/Users/you/Profiles/Work" app="Claude · Work" testId="c" />);
    await act(async () => fireEvent.click(await screen.findByTestId("c-add")));
    const show = screen.getByRole("button", { name: CONNECTOR.showFiles });
    expect(show.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryAllByRole("button", { name: "Its text" })).toEqual([]);
    // A press is the button's click: an accessibility press, Enter or Space.
    fireEvent.click(show);
    expect(show.getAttribute("aria-expanded")).toBe("true");
    const text = screen.getAllByRole("button", { name: "Its text" })[0]!;
    expect(text.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(text);
    expect(text.getAttribute("aria-expanded")).toBe("true");
    expect(document.getElementById(text.getAttribute("aria-controls")!)!.hidden).toBe(false);
  });
});

// #5 (aij-test run 2): after Add in "Add a session", the new node read
// "Its app has no AI Juggler connector" from a list read before the Add.
// Every change to the pool's members reads the desktop lists again.
describe("round 7 #5: the desktop lists follow the pool's members", () => {
  const APP = "app:claude-app:/Users/you/Profiles/aij-test";
  const ref = `app:${APP}`;
  beforeEach(() => {
    forgetSharedReads();
    resetPreviewConnector();
  });
  afterEach(() => {
    cleanup();
    Object.assign(window, { __AIJ_DESKTOP_MEMBERS__: undefined, __AIJ_CONNECTOR__: undefined, __AIJ_POOLS__: undefined });
    resetPreviewPools();
  });

  it("an Add changes the App node's words from the stale ones to the App's", async () => {
    const POOL = "pool-careful";
    Object.assign(window, { __AIJ_POOLS__: { pools: [{ id: POOL, name: "Careful", rules: [] }], members: [] } });
    resetPreviewPools();
    const desktop = (connector: string, connected: boolean) => Object.assign(window, { __AIJ_DESKTOP_MEMBERS__: { [POOL]: [{ member: ref, app_id: APP, connector, connected, wake: "next_turn" }] }, __AIJ_CONNECTOR__: { [APP]: { state: connector } } });
    setPools(previewPoolsInfo());
    desktop("absent", false);
    render(<RuleNodes poolId={POOL} rules={[{ member: ref, kind: "app", role: "member", level: null, joined_at: 1, sessions: 0, session_ids: [] }]} watched={[]} labelOf={() => "Claude app · aij-test"} selection={{ kind: "none" }} onSelect={() => undefined} />);
    await waitFor(() => expect(screen.getByTestId("canvas-rule-wake").textContent).toBe("Watched: its app has no AI Juggler connector"));
    // The backend now has it right (the App's connector in, its session
    // calling); the person's Add, as "Add a session" makes it:
    desktop("current", true);
    await act(async () => setPools(await poolJoin(POOL, ref, "member")));
    await waitFor(() => expect(screen.getByTestId("canvas-rule-wake").textContent).toBe("Its sessions: reads messages on its next turn"));
  });
});
