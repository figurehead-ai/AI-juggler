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
import { draftOf, sessionCandidates, talkChanges, type Candidate, type Draft } from "../../lib/new-pool";
import type { MemberKind } from "../../lib/ipc/pool-model";
import { CANDIDATES_PAGE, NewPoolWho } from "./NewPoolWho";
import { templateById } from "../../lib/pool-templates";
import { sessionState } from "../../lib/rail-items";
import { shortName } from "../../lib/session-titles";
import type { AgentBridgeInfo } from "../../types/generated/AgentBridgeInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { PREVIEW_TOOLS } from "../../preview/tools";
import { NewPool, busiestProject, freeName, nameOnChoose } from "./NewPool";
import { NewPoolReview, talkSentence } from "./NewPoolReview";
import { NewPoolRoles } from "./NewPoolRoles";

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

  // D-96 D in the wizard (the rc27 pool pass): four untitled Lab sessions
  // in one folder read "team · Lab · started 9:2x AM", alike to the minute.
  it("alike untitled sessions are told apart in the wizard's list", () => {
    const base = MOCK_SESSIONS.find((one) => sessionState(one).state !== "ended") ?? MOCK_SESSIONS[0]!;
    const at = new Date(2026, 10, 10, 9, 25).getTime();
    const lab = (id: string, started: number): InstanceInfo => ({ ...base, id, label: undefined, title: undefined, git_branch: null, cwd: "/Users/dev/team", project_id: "team", account_label: "Lab", started_at_ms: started });
    const sessions = [lab("a", at), lab("b", at + 20_000), lab("c", at - 60_000), lab("d", at - 50_000)];
    const draft = { ...draftOf(templateById("feature-team"), "team"), projects: ["team"] };
    const rows = sessionCandidates({ draft, sessions, conversations: [], pools: null, tools: [], projectName: (id) => id ?? "No project", nameOf: (s) => shortName(s, at + 3_600_000), now: at + 3_600_000 });
    const labels = rows.map((row) => row.label);
    expect(new Set(labels).size).toBe(4);
    expect(labels.every((label) => /^team · .* · started .*\(\d\)$/.test(label))).toBe(true);
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

  // The rc26 P1 walk: Roles and talk told the user to pick a Lead where
  // no row offered one (two Apps, D-92), and Review listed the members
  // without their roles.
  it("Writer and reviewer of two Apps says how a Lead gets chosen, and Review shows each member's role", () => {
    const apps = [
      { ref: "app:cli:a", kind: "app" as const, label: "Claude Code · Work · work", role: "member" as const, level: null, reach: "later" as const },
      { ref: "app:cli:b", kind: "app" as const, label: "Codex · Personal · default", role: "reviewer" as const, level: null, reach: "later" as const },
    ];
    const draft = { ...draftOf(templateById("writer-reviewer"), "web"), members: apps };
    const roles = render(<NewPoolRoles draft={draft} onDraft={() => undefined} />);
    expect(roles.queryByTestId("new-pool-roles-to-choose")).toBeNull();
    expect(roles.getByTestId("new-pool-lead-later").textContent).toMatch(/^No Lead yet: Lead is one session/);
    roles.unmount();
    const review = render(<NewPoolReview draft={draft} onName={() => undefined} projectName={(id) => id ?? "No project"} />);
    expect(review.getAllByTestId("new-pool-review-role").map((chip) => chip.textContent)).toEqual(["Member", "Reviewer"]);
    expect(review.getByTestId("new-pool-review").textContent).toContain("No Lead yet");
  });

  it("Escape leaves nothing made", () => {
    let closed = false;
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => (closed = true)} onByHand={() => undefined} />);
    fireEvent.keyDown(screen.getByTestId("new-pool"), { key: "Escape" });
    expect(closed).toBe(true);
    expect(seams().__AIJ_POOL_CALLS__ ?? []).toEqual([]);
  });
});

/** rc27 live pass 2: step 2 drew 143 session rows at once and was slow. A
 * group shows a page, and Show more adds the next. */
describe("New pool's Who list is paged", () => {
  it("draws a page of a long group, and Show more adds another", () => {
    const rows: Candidate[] = Array.from({ length: 143 }, (_, i) => ({
      ref: `sess-${i}`,
      kind: "session",
      label: `Session ${i}`,
      detail: "lab · ended",
      group: "recent",
      reach: "full",
      blocked: null,
      lastMs: 143 - i,
    }));
    const candidates = { session: rows, app: [], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    render(<NewPoolWho draft={draftOf(templateById("feature-team"), null)} onDraft={() => undefined} candidates={candidates} projects={[]} onFix={() => undefined} />);
    const list = screen.getByTestId("new-pool-candidates");
    expect(within(list).getAllByRole("listitem")).toHaveLength(CANDIDATES_PAGE);
    fireEvent.click(screen.getByTestId("new-pool-more-recent"));
    expect(within(list).getAllByRole("listitem")).toHaveLength(2 * CANDIDATES_PAGE);
    expect(screen.getByTestId("new-pool-more-recent").textContent).toContain(`${143 - 2 * CANDIDATES_PAGE} more`);
    // A search pages its matches the same way.
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Session 1" } });
    expect(within(list).getAllByRole("listitem").length).toBeLessThanOrEqual(CANDIDATES_PAGE);
  });
});

// The P8 walk (rc32, step 4): adding a Kimi Code App left the counters at
// "0 full, 0 shell only", and its row said only "new ones join".
describe("P8 an App member that works through its shell, in the wizard", () => {
  it("its row says Shell only beside new ones join, and the counter counts it", () => {
    const why = "Its tool has no agent tools yet: it works through its shell, and can't read the pool, post or take a task.";
    const kimi: Candidate = { ref: "app:cli:kimi-1", kind: "app", label: "Kimi Code · Lab", detail: "none open now", group: "all", reach: "later", blocked: null, shell: why, lastMs: 0 };
    const candidates = { session: [], app: [kimi], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    let draft: Draft = { ...draftOf(templateById("blank"), null), members: [] };
    const { rerender } = render(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} projects={[]} onFix={() => undefined} />);
    fireEvent.click(screen.getByRole("radio", { name: /Apps/ }));
    const row = screen.getByTestId("new-pool-candidate");
    expect(row.textContent).toContain("new ones join");
    expect(within(row).getByTestId("shell-only").getAttribute("aria-description")).toBe(why);
    fireEvent.click(within(row).getByRole("checkbox"));
    rerender(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} projects={[]} onFix={() => undefined} />);
    expect(screen.getByTestId("new-pool-side").textContent).toContain("1 shell only");
  });
});
