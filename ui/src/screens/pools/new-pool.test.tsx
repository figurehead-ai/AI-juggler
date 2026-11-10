/**
 * AJ-E3007: the New pool wizard. A template sets roles and talk; step 2
 * picks from what exists, greyed rows saying why; step 3's grid cycles a
 * pair's rule; the review says it in sentences; Make sends the user's own
 * calls in one go (create, scope, members, talk) and opens the canvas;
 * Escape leaves nothing made.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools } from "../../lib/pools";
import { draftOf, sessionCandidates, talkChanges } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { sessionState } from "../../lib/rail-items";
import type { AgentBridgeInfo } from "../../types/generated/AgentBridgeInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { PREVIEW_TOOLS } from "../../preview/tools";
import { NewPool, busiestProject, freeName, nameOnChoose } from "./NewPool";
import { talkSentence } from "./NewPoolReview";

type Seams = { __AIJ_POOL_CALLS__?: string[] };
const seams = () => globalThis as unknown as Seams;

beforeEach(async () => {
  delete seams().__AIJ_POOL_CALLS__;
  resetPreviewPools();
  resetPreviewPoolModel();
  resetPools();
  await refreshPools();
});
afterEach(() => cleanup());

describe("AJ-E3007 the templates and the draft", () => {
  it("a template's talk and roles become the draft's", () => {
    const team = draftOf(templateById("feature-team"), "p1");
    expect(team.projects).toEqual(["p1"]);
    expect(talkChanges(team.talk)).toContainEqual({ from: "role:member", to: "role:member", rule: "ask" });
    const everywhere = draftOf(templateById("reviewer-for-everything"), "p1");
    expect(everywhere.every).toBe(true);
    expect(talkSentence(everywhere)).toContain("Member to Member: blocked");
    expect(draftOf(templateById("sealed-client"), "p1").sealed).toBe(true);
  });

  it("a session out of scope says why and offers the scope", () => {
    const draft = { ...draftOf(templateById("writer-reviewer"), "nowhere"), projects: ["nowhere"] };
    const rows = sessionCandidates({ draft, sessions: MOCK_SESSIONS, conversations: [], pools: null, tools: [], projectName: (id) => id ?? "No project", nameOf: (s) => s.id, now: Date.now() });
    const out = rows.find((row) => row.blocked?.includes("not in this pool's projects"));
    expect(out?.fix?.action).toBe("scope");
  });

  // The rc18 walk (E3002-R3 step 12): the count follows the agent tools'
  // switches as the launch does, each member by its own project.
  it("a member counts as shell only where the tools are off: everywhere, for its project, or for its App", () => {
    const base = MOCK_SESSIONS.find((one) => sessionState(one).state !== "ended") ?? MOCK_SESSIONS[0]!;
    const session = (id: string, project: string, account: string) => ({ ...base, id, project_id: project, account_id: account });
    const sessions = [session("s-on", "p-on", "app-a"), session("s-off", "p-off", "app-a"), session("s-app-off", "p-on", "app-off")];
    const tools = PREVIEW_TOOLS.filter((tool) => tool.id === base.provider && tool.supports.agent_tools);
    expect(tools).toHaveLength(1);
    const draft = { ...draftOf(templateById("blank"), "p-on"), projects: ["p-on", "p-off"] };
    const reach = (bridge: AgentBridgeInfo | null) =>
      Object.fromEntries(sessionCandidates({ draft, sessions, conversations: [], pools: null, tools, bridge, projectName: (id) => id ?? "No project", nameOf: (s) => s.id, now: Date.now() }).map((row) => [row.ref, row.reach]));
    const switches = { asked: true, on: true, available: true, off_projects: ["p-off"], off_profiles: ["app-off"] };
    expect(reach(switches)).toEqual({ "session:s-on": "full", "session:s-off": "shell", "session:s-app-off": "shell" });
    expect(Object.values(reach({ ...switches, on: false, off_projects: [], off_profiles: [] }))).toEqual(["shell", "shell", "shell"]);
    // Nothing claimed while the switches load or before the question.
    expect(Object.values(reach(null))).toEqual(["full", "full", "full"]);
    expect(Object.values(reach({ ...switches, asked: false, on: false }))).toEqual(["full", "full", "full"]);
  });

  it("rc7 walk: choosing a template replaces its automatic name, never one typed; Blank's is none", () => {
    const taken = ["Writer and reviewer"];
    const auto = { template: "writer-reviewer", name: "Writer and reviewer 2" };
    expect(nameOnChoose(auto, templateById("blank"), taken)).toBe("");
    expect(nameOnChoose(auto, templateById("feature-team"), taken)).toBe("Feature team");
    expect(nameOnChoose({ template: "blank", name: "" }, templateById("feature-team"), taken)).toBe("Feature team");
    expect(nameOnChoose({ template: "writer-reviewer", name: "Checkout team" }, templateById("blank"), taken)).toBe("Checkout team");
  });

  it("a name already taken gets the next number", () => {
    expect(freeName("Scouts", ["scouts", "Scouts 2"])).toBe("Scouts 3");
  });

  // The rc26 P1 walk: every web session had ended, so a one-project
  // template started with Projects "none yet", though web was the lab's
  // one project and the template's card said "one project".
  it("a one-project template starts where the work is: open now, else worked in last, else the only project", () => {
    const base = MOCK_SESSIONS[0]!;
    const ended = (id: string, project: string, at: number): InstanceInfo => ({ ...base, id, project_id: project, status: "finished", last_activity_ms: at, started_at_ms: at - 1 });
    const open = MOCK_SESSIONS.find((one) => sessionState(one).state !== "ended")!;
    expect(sessionState(ended("x", "web", 1)).state).toBe("ended");
    expect(busiestProject([ended("a", "web", 200), ended("b", "api", 100)])).toBe("web");
    expect(busiestProject([ended("a", "web", 200), { ...open, project_id: "api" }])).toBe("api");
    expect(busiestProject([], [{ id: "web" }])).toBe("web");
    expect(busiestProject([], [{ id: "web" }, { id: "api" }])).toBeNull();
  });
});

describe("AJ-E3007 the wizard, start to end", () => {
  it("picks, sets talk, reviews and makes the pool with the user's own calls", async () => {
    let opened: string | null = null;
    render(
      <ActionHostProvider host={{ openPool: (id: string) => (opened = id) }}>
        <NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => undefined} onByHand={() => undefined} />
      </ActionHostProvider>,
    );
    const wizard = screen.getByTestId("new-pool");
    fireEvent.click(within(wizard).getAllByTestId("new-pool-template").find((card) => card.dataset.template === "feature-team")!);
    fireEvent.click(screen.getByTestId("new-pool-next"));
    // Every project, so every open session can join.
    fireEvent.click(within(screen.getByTestId("new-pool-where")).getByText("Every project"));
    const first = screen.getAllByTestId("new-pool-candidate").find((row) => !row.querySelector("[role=checkbox]")!.hasAttribute("disabled"))!;
    fireEvent.click(within(first).getByRole("checkbox"));
    expect(within(screen.getByTestId("new-pool-side")).getByText(/1 full|0 full/)).toBeTruthy();
    fireEvent.click(screen.getByTestId("new-pool-next"));
    // The template's member↔member ask first, then blocked on a click.
    const cell = screen.getAllByTestId("new-pool-talk-cell").find((one) => one.dataset.pair === "member>member")!;
    expect(cell.textContent).toBe("Ask first");
    // D-95: the grid says what it can't hold.
    expect(screen.getByTestId("new-pool-talk-claude").textContent).toBe("Claude's own messages between sessions aren't held by this.");
    fireEvent.click(cell);
    expect(cell.textContent).toBe("Blocked");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    expect(screen.getByTestId("new-pool-review").textContent).toContain("Member to Member: blocked");
    fireEvent.change(screen.getByTestId("new-pool-name"), { target: { value: "Checkout team" } });
    fireEvent.click(screen.getByTestId("new-pool-next"));
    await waitFor(() => expect(opened).not.toBeNull());
    const calls = seams().__AIJ_POOL_CALLS__ ?? [];
    expect(calls).toContain("create:Checkout team");
    expect(calls.some((call) => call.startsWith(`scope:${opened}:every`))).toBe(true);
    expect(calls.some((call) => call.startsWith(`join:${opened}:session:`) && call.endsWith(":lead:"))).toBe(true);
    expect(calls).toContain(`talk:${opened}:role:member:role:member:blocked`);
  });

  it("AJ-E3007 R5: Back keeps every choice — the template, the members, the talk and the name", () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => undefined} onByHand={() => undefined} />);
    const wizard = screen.getByTestId("new-pool");
    fireEvent.click(within(wizard).getAllByTestId("new-pool-template").find((card) => card.dataset.template === "feature-team")!);
    fireEvent.click(screen.getByTestId("new-pool-next"));
    fireEvent.click(within(screen.getByTestId("new-pool-where")).getByText("Every project"));
    const rows = screen.getAllByTestId("new-pool-candidate");
    const picked = rows.findIndex((row) => !row.querySelector("[role=checkbox]")!.hasAttribute("disabled"));
    fireEvent.click(within(rows[picked]!).getByRole("checkbox"));
    fireEvent.click(screen.getByTestId("new-pool-next"));
    const cell = () => screen.getAllByTestId("new-pool-talk-cell").find((one) => one.dataset.pair === "member>member")!;
    fireEvent.click(cell());
    expect(cell().textContent).toBe("Blocked");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    fireEvent.change(screen.getByTestId("new-pool-name"), { target: { value: "Checkout team" } });
    // Back to the first step and forward again: nothing was lost.
    for (let n = 0; n < 3; n += 1) fireEvent.click(screen.getByTestId("new-pool-back"));
    expect(within(screen.getByTestId("new-pool")).getAllByTestId("new-pool-template").find((card) => card.dataset.template === "feature-team")!.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    const again = screen.getAllByTestId("new-pool-candidate")[picked]!;
    expect(within(again).getByRole("checkbox").getAttribute("aria-checked")).toBe("true");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    expect(cell().textContent).toBe("Blocked");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    expect(screen.getByTestId("new-pool-name")).toHaveProperty("value", "Checkout team");
    expect(seams().__AIJ_POOL_CALLS__ ?? []).toEqual([]);
  });

  it("AJ-E3007 R2–R5: search past 8, the seal's consequence, what each member can do, by hand on every step, Cancel makes nothing", () => {
    let byHand = 0;
    let closed = 0;
    // Twelve open sessions: more than eight to choose from.
    const many = Array.from({ length: 12 }, (_, n) => ({ ...MOCK_SESSIONS[0]!, id: `s-many-${n}`, label: `Session number ${n}`, status: "running" }));
    render(<NewPool sessions={many} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => (closed += 1)} onByHand={() => (byHand += 1)} />);
    const byHandOnEveryStep: boolean[] = [];
    const seeByHand = () => byHandOnEveryStep.push(screen.queryByTestId("new-pool-by-hand")?.textContent === "Set up by hand on the canvas");
    seeByHand();
    fireEvent.click(screen.getByTestId("new-pool-next"));
    seeByHand();
    fireEvent.click(within(screen.getByTestId("new-pool-where")).getByText("Every project"));
    // R2: past eight a search field; typing keeps what matches.
    const search = screen.getByRole("searchbox");
    fireEvent.change(search, { target: { value: "number 11" } });
    const shown = screen.getAllByTestId("new-pool-candidate");
    expect(shown).toHaveLength(1);
    expect(shown[0]!.textContent).toContain("Session number 11");
    fireEvent.click(within(shown[0]!).getByRole("checkbox"));
    fireEvent.click(screen.getByTestId("new-pool-next"));
    seeByHand();
    // R3: the seal says what it does, in plain words.
    expect(screen.getByTestId("new-pool-seal").textContent).toContain("Its sessions leave other pools. No links, and no messages or docs out");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    seeByHand();
    // R4: the review says what each member can actually do here.
    const can = screen.getByRole("region", { name: "What each member can do here" });
    expect(can.textContent).toContain("Session number 11");
    expect(can.textContent).toMatch(/sees · messages · tasks · can be asked|acts through its shell|watched: shown/);
    // R5: by hand on every step; Cancel closes, nothing made.
    expect(byHandOnEveryStep).toEqual([true, true, true, true]);
    fireEvent.click(screen.getByTestId("new-pool-by-hand"));
    expect(byHand).toBe(1);
    fireEvent.click(screen.getByTestId("new-pool-cancel"));
    expect(closed).toBe(1);
    expect(seams().__AIJ_POOL_CALLS__ ?? []).toEqual([]);
  });

  it("Escape leaves nothing made", () => {
    let closed = false;
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => (closed = true)} onByHand={() => undefined} />);
    fireEvent.keyDown(screen.getByTestId("new-pool"), { key: "Escape" });
    expect(closed).toBe(true);
    expect(seams().__AIJ_POOL_CALLS__ ?? []).toEqual([]);
  });
});
