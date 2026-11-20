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
import { draftOf, freeName, makePool, sessionCandidates, talkChanges, type Draft } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { sessionState } from "../../lib/rail-items";
import { shortName } from "../../lib/session-titles";
import type { AgentBridgeInfo } from "../../types/generated/AgentBridgeInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { PREVIEW_TOOLS } from "../../preview/tools";
import { NewPool, busiestProject, nameOnChoose } from "./NewPool";
import { NewPoolReview } from "./NewPoolReview";
import { talkSentence } from "../../lib/pool-sentences";
import { NewPoolLead } from "./NewPoolLead";
import type { SetupStep } from "../../lib/pool-words";

type Seams = { __AIJ_POOL_CALLS__?: string[] };
const seams = () => globalThis as unknown as Seams;

beforeEach(async () => {
  delete seams().__AIJ_POOL_CALLS__;
  localStorage.clear();
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
  // in one folder read "team · Lab · started 9:2x AM", alike to the minute
  // (D-103.8: "team · started 9:2x AM", their Account on the line under).
  it("alike untitled sessions are told apart in the wizard's list", () => {
    const base = MOCK_SESSIONS.find((one) => sessionState(one).state !== "ended") ?? MOCK_SESSIONS[0]!;
    const at = new Date(2026, 10, 10, 9, 25).getTime();
    const lab = (id: string, started: number): InstanceInfo => ({ ...base, id, label: undefined, title: undefined, git_branch: null, cwd: "/Users/dev/team", project_id: "team", account_label: "Lab", started_at_ms: started });
    const sessions = [lab("a", at), lab("b", at + 20_000), lab("c", at - 60_000), lab("d", at - 50_000)];
    const draft = { ...draftOf(templateById("feature-team"), "team"), projects: ["team"] };
    const rows = sessionCandidates({ draft, sessions, conversations: [], pools: null, tools: [], projectName: (id) => id ?? "No project", nameOf: (s) => shortName(s, at + 3_600_000), now: at + 3_600_000 });
    const labels = rows.map((row) => row.label);
    expect(new Set(labels).size).toBe(4);
    expect(labels.every((label) => /^team · started .*\(\d\)$/.test(label))).toBe(true);
    expect(rows.every((row) => row.detail.includes("Lab"))).toBe(true);
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

/** Next, until step `step` shows (D-126: its title is its question). */
function nextTo(step: SetupStep): void {
  for (let n = 0; n < 8 && !screen.queryByTestId(`new-pool-step-${step}`); n += 1) fireEvent.click(screen.getByTestId("new-pool-next"));
  expect(screen.getByTestId(`new-pool-step-${step}`)).toBeTruthy();
}

/** The steps beside the form, by their names. */
const stepNames = () => screen.getAllByTestId("flow-step").map((step) => step.querySelector('[id$="-name"]')?.textContent);

/** The preview's row for `row`, its value. */
const previewValue = (row: string) => screen.getAllByTestId("new-pool-preview-row").find((one) => one.dataset.row === row)?.querySelector("[data-testid=new-pool-preview-value]")?.textContent;

describe("AJ-E3007, AJ-E3011 the setup, start to end", () => {
  it("picks, sets talk, reviews and makes the pool with the user's own calls", async () => {
    let opened: string | null = null;
    render(
      <ActionHostProvider host={{ openPool: (id: string) => (opened = id) }}>
        <NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => undefined} />
      </ActionHostProvider>,
    );
    const wizard = screen.getByTestId("new-pool");
    fireEvent.click(within(within(wizard).getAllByTestId("new-pool-template").find((card) => card.dataset.template === "feature-team")!).getByRole("radio"));
    nextTo("where");
    fireEvent.change(screen.getByTestId("new-pool-name"), { target: { value: "Checkout team" } });
    // Every project, so every open session can join.
    fireEvent.click(within(screen.getByTestId("new-pool-where")).getByText("Every project"));
    nextTo("who");
    const first = screen.getAllByTestId("new-pool-candidate").find((row) => !row.querySelector("[role=checkbox]")!.hasAttribute("disabled"))!;
    fireEvent.click(within(first).getByRole("checkbox"));
    // Round 2 (#6, #7): who's in, in words, without "full / shell only / watched".
    expect(previewValue("who")).toBe(`${first.querySelector(".font-medium, .font-semibold")!.textContent.split(" · ")[0]}, and every new session in every project`);
    expect(screen.getByTestId("new-pool-preview").textContent).not.toMatch(/full tools|shell only|watched/);
    nextTo("talk");
    // The template's member↔member ask first, then blocked on a click.
    const cell = screen.getAllByTestId("new-pool-talk-cell").find((one) => one.dataset.pair === "member>member")!;
    expect(cell.dataset.rule).toBe("ask");
    expect(cell.textContent).toContain("Ask first");
    // D-95: the table says what it can't hold.
    expect(screen.getByTestId("new-pool-talk-below").textContent).toContain("Claude's own messages between sessions aren't held by this.");
    fireEvent.click(cell);
    expect(cell.dataset.rule).toBe("blocked");
    nextTo("review");
    expect(screen.getByTestId("new-pool-review").textContent).toContain("Member to Member: blocked");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    await waitFor(() => expect(opened).not.toBeNull());
    const calls = seams().__AIJ_POOL_CALLS__ ?? [];
    expect(calls).toContain("create:Checkout team");
    expect(calls.some((call) => call.startsWith(`scope:${opened}:every`))).toBe(true);
    expect(calls.some((call) => call.startsWith(`join:${opened}:session:`) && call.endsWith(":lead:"))).toBe(true);
    expect(calls).toContain(`talk:${opened}:role:member:role:member:blocked`);
  });

  it("the steps in order, each with a heading and a line on why; D-30's question its own step while unanswered", async () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => undefined} />);
    await waitFor(() => expect(screen.getAllByTestId("flow-step")).toHaveLength(9));
    expect(stepNames()).toEqual(["Start", "Where", "Who's in", "Who leads", "Who builds, who checks", "Who may message whom", "On their own", "Team tools", "Review"]);
    const seen: string[] = [];
    for (let n = 0; n < 9; n += 1) {
      seen.push(`${screen.getByTestId("new-pool-step-eyebrow").textContent} | ${screen.getByTestId("new-pool-step-title").textContent}: ${screen.getByTestId("new-pool-step-why").textContent}`);
      if (n < 8) fireEvent.click(screen.getByTestId("new-pool-next"));
    }
    // D-126: each step's question, in three groups.
    expect(seen).toEqual([
      "Step 1 of 9 · The basics | How do you want to start?: A pool is a team of your AI sessions on the same project. Start from scratch and each step explains itself, or pick a team that is already set up.",
      "Step 2 of 9 · The basics | Name it and pick its project: Only sessions working in these projects can join this pool.",
      "Step 3 of 9 · The basics | Who's in this team?: Add the apps whose sessions should work together. Each one you add can get its own role in the next steps, such as the reviewer.",
      "Step 4 of 9 · How they work | Who leads the team?: The lead turns what you ask into tasks and hands them out. You can leave this for later. What a lead does",
      "Step 5 of 9 · How they work | Who builds, and who checks?: Give each member a role. A reviewer looks over the others' work before it counts as done. About roles",
      "Step 6 of 9 · How they work | Who may message whom?: Most teams let everyone talk. Pick a stricter way only if you want to approve some messages first.",
      "Step 7 of 9 · How they work | What may they do on their own?: How much they do together without asking you. Anything beyond this waits for your OK in Needs you.",
      "Step 8 of 9 · Finish | Let your assistants message each other?: Your team is set up. One last switch lets its sessions actually talk. You answer this once, for all your pools.",
      "Step 9 of 9 · Finish | Ready to make “ai-juggler team”: Here is the whole team. Change anything now, or later on its diagram.",
    ]);
  });

  it("Start from scratch is a full card, first; a name and its projects make a working pool from Who's in on", async () => {
    let opened: string | null = null;
    render(
      <ActionHostProvider host={{ openPool: (id: string) => (opened = id) }}>
        <NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />
      </ActionHostProvider>,
    );
    const cards = screen.getAllByTestId("new-pool-template");
    expect(cards[0]!.dataset.template).toBe("blank");
    expect(cards[0]!.textContent).toContain("Start from scratch");
    expect(within(cards[0]!).getByRole("radio")).toHaveProperty("checked", true);
    expect(screen.queryByTestId("new-pool-by-hand")).toBeNull();
    // No Make pool before Where is done.
    expect(screen.queryByTestId("new-pool-make")).toBeNull();
    nextTo("where");
    expect(screen.queryByTestId("new-pool-make")).toBeNull();
    nextTo("who");
    // From here on, Make pool; the foot says the rest is prefilled, and
    // the preview what with (D-126: each row, "· prefilled").
    expect(screen.getByTestId("new-pool-prefilled").textContent).toBe("The rest is prefilled. You can make the pool now.");
    expect(previewValue("who")).toBe("Every new session in ai-juggler");
    // AJ-E3014 item 4: the side panel says who joins now and who later.
    // One line, its full words its tooltip (the visual review: two lines
    // pushed the step's own row out of view).
    const joins = screen.getByTestId("new-pool-preview-joins");
    expect(joins.getAttribute("title")).toMatch(/^Now: (no session open|\d+ open sessions?) · Later: every new session in ai-juggler$/);
    expect(joins.firstChild?.textContent).toMatch(/^(None|\d+) open now · new ones join$/);
    expect(joins.className).toContain("truncate");
    // The step says how a desktop session takes part at most once.
    expect(screen.queryAllByTestId("new-pool-desktop-note").length).toBeLessThanOrEqual(1);
    expect(previewValue("everyone")).toBe("Everyone builds (Member) · prefilled");
    expect(previewValue("talk")).toBe("Everyone may message everyone; an Observer only reads. · prefilled");
    expect(previewValue("own")).toBe("Work together · prefilled");
    fireEvent.click(screen.getByTestId("new-pool-make"));
    await waitFor(() => expect(opened).not.toBeNull());
    const calls = seams().__AIJ_POOL_CALLS__ ?? [];
    expect(calls).toContain("create:ai-juggler team");
    expect(calls).toContain(`join:${opened}:all:member:`);
    // The canvas's checklist: Where and Who's in were walked here.
    expect(localStorage.getItem(`aij.pool-setup.${opened}`)).toContain("where");
  });

  it("the preview's Messages line is the Review's: an Observer set to Ask first is an exception there too", () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />);
    nextTo("talk");
    expect(previewValue("talk")).toBe("Everyone may message everyone; an Observer only reads.");
    const cell = screen.getAllByTestId("new-pool-talk-cell").find((one) => one.dataset.pair === "observer>lead")!;
    expect(cell.dataset.rule).toBe("blocked");
    fireEvent.click(cell);
    expect(cell.dataset.rule).toBe("ask");
    expect(previewValue("talk")).toContain("Observer to Lead: ask first");
    nextTo("review");
    expect(screen.getByTestId("new-pool-review").textContent).toContain(previewValue("talk") ?? "–");
  });

  it("AJ-E3007 R5: Back keeps every choice — the template, the members, the talk and the name", () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => undefined} />);
    const wizard = screen.getByTestId("new-pool");
    fireEvent.click(within(within(wizard).getAllByTestId("new-pool-template").find((card) => card.dataset.template === "feature-team")!).getByRole("radio"));
    nextTo("where");
    fireEvent.change(screen.getByTestId("new-pool-name"), { target: { value: "Checkout team" } });
    fireEvent.click(within(screen.getByTestId("new-pool-where")).getByText("Every project"));
    nextTo("who");
    const rows = screen.getAllByTestId("new-pool-candidate");
    const picked = rows.findIndex((row) => !row.querySelector("[role=checkbox]")!.hasAttribute("disabled"));
    fireEvent.click(within(rows[picked]!).getByRole("checkbox"));
    nextTo("talk");
    const cell = () => screen.getAllByTestId("new-pool-talk-cell").find((one) => one.dataset.pair === "member>member")!;
    fireEvent.click(cell());
    expect(cell().dataset.rule).toBe("blocked");
    // Back to the first step and forward again: nothing was lost.
    for (let n = 0; n < 5; n += 1) fireEvent.click(screen.getByTestId("new-pool-back"));
    expect(within(within(screen.getByTestId("new-pool")).getAllByTestId("new-pool-template").find((card) => card.dataset.template === "feature-team")!).getByRole("radio")).toHaveProperty("checked", true);
    nextTo("where");
    expect(screen.getByTestId("new-pool-name")).toHaveProperty("value", "Checkout team");
    nextTo("who");
    const again = screen.getAllByTestId("new-pool-candidate")[picked]!;
    expect(within(again).getByRole("checkbox").getAttribute("aria-checked")).toBe("true");
    nextTo("talk");
    expect(cell().dataset.rule).toBe("blocked");
    expect(seams().__AIJ_POOL_CALLS__ ?? []).toEqual([]);
  });

  it("AJ-E3007 R2–R5: search past 8, the seal's consequence, what each member can do, Cancel makes nothing", () => {
    let closed = 0;
    // Twelve open sessions: more than eight to choose from.
    const many = Array.from({ length: 12 }, (_, n) => ({ ...MOCK_SESSIONS[0]!, id: `s-many-${n}`, label: `Session number ${n}`, status: "running" }));
    render(<NewPool sessions={many} projects={MOCK_PROJECTS} accounts={[]} project={null} startWith="feature-team" onClose={() => (closed += 1)} />);
    nextTo("where");
    fireEvent.click(within(screen.getByTestId("new-pool-where")).getByText("Every project"));
    nextTo("who");
    // R2: past eight a search field; typing keeps what matches.
    const search = screen.getByRole("searchbox");
    fireEvent.change(search, { target: { value: "number 11" } });
    const shown = within(screen.getByTestId("new-pool-sessions-open")).getAllByTestId("new-pool-candidate");
    expect(shown).toHaveLength(1);
    expect(shown[0]!.textContent).toContain("Session number 11");
    fireEvent.click(within(shown[0]!).getByRole("checkbox"));
    nextTo("own");
    // R3: the seal says what it does, in plain words, with an example.
    expect(screen.getByTestId("new-pool-seal").textContent).toContain("Its sessions leave other pools. No links, and no messages or docs out");
    expect(screen.getByTestId("new-pool-seal").textContent).toContain("Example:");
    nextTo("review");
    // R4: the review says what each member can actually do here.
    const can = screen.getByRole("region", { name: "What each member can do here" });
    expect(can.textContent).toContain("Session number 11");
    expect(can.textContent).toMatch(/sees · messages · tasks · can be asked|acts through its shell|watched: shown/);
    fireEvent.click(screen.getByTestId("new-pool-cancel"));
    expect(closed).toBe(1);
    expect(seams().__AIJ_POOL_CALLS__ ?? []).toEqual([]);
  });

  // The newcomer walks (AJ-E3011): with two Apps and no session open, the
  // lead was a dead end. Now the picker offers each App's next session.
  it("two Apps: the lead picker offers each App's next session and no lead; Review reads it back", async () => {
    const apps = [
      { ref: "app:cli:a", kind: "app" as const, label: "Claude Code · Work", role: "member" as const, level: null, reach: "later" as const },
      { ref: "app:cli:b", kind: "app" as const, label: "Codex · Personal", role: "reviewer" as const, level: null, reach: "later" as const },
    ];
    let draft: Draft = { ...draftOf(templateById("writer-reviewer"), "web"), members: apps };
    const lead = render(<NewPoolLead draft={draft} onDraft={(next) => (draft = next)} sessions={[]} nameOf={(_, label) => label} where="web" />);
    const options = lead.getAllByTestId("new-pool-lead-option");
    expect(options.map((one) => one.dataset.value?.split("|")[0])).toEqual(["next", "next", "none"]);
    expect(lead.getByTestId("new-pool-lead-none-open").textContent).toBe("No session is open in these projects now.");
    expect(lead.container.textContent).toContain("Claude Code · Work's next session");
    // Round 4 (#4): it takes charge as soon as one of its sessions starts, and the pool's
    // diagram can start one once it's made.
    expect(lead.container.textContent).toContain("It takes charge as soon as one of its sessions starts in web.");
    expect(lead.getByTestId("new-pool-lead-start-later").textContent).toBe("Once the pool is made, Start one now on its App's card starts that session.");
    fireEvent.click(options[0]!);
    expect(draft.leadNext).toBe("app:cli:a");
    lead.rerender(<NewPoolLead draft={draft} onDraft={(next) => (draft = next)} sessions={[]} nameOf={(_, label) => label} where="web" />);
    expect(lead.getByTestId("new-pool-lead-sentence").textContent).toBe("Claude Code · Work takes charge as soon as one of its sessions starts in web: it plans the work and hands out tasks.");
    lead.unmount();
    const review = render(<NewPoolReview draft={draft} name="Web team" where="web" openNow={new Map([["app:cli:a", 0]])} />);
    expect(review.getByTestId("new-pool-review").textContent).toContain("Claude Code · Work takes charge as soon as one of its sessions starts in web: it plans the work and hands out tasks.");
    // Round 2 (#2): one member, not a Lead beside a Member.
    expect(review.getAllByTestId("new-pool-review-role").map((line) => line.textContent)).toEqual([
      "Claude Code · Work takes charge as soon as one of its sessions starts; its others join as Member and may Work together, as the pool.",
      "Codex · Personal is a Reviewer and may Work together, as the pool.",
    ]);
  });

  it("makes the wait for the App's next session the pool's own setting", async () => {
    const apps = [{ ref: "app:cli:a", kind: "app" as const, label: "Claude Code · Work", role: "member" as const, level: null, reach: "later" as const }];
    const draft: Draft = { ...draftOf(templateById("blank"), "web"), name: "Web team", members: apps, leadNext: "app:cli:a" };
    const { id } = await makePool(draft);
    expect(seams().__AIJ_POOL_CALLS__).toContain(`lead_next:${id}:cli:a`);
  });

  it("Escape leaves nothing made", () => {
    let closed = false;
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => (closed = true)} />);
    fireEvent.keyDown(screen.getByTestId("new-pool"), { key: "Escape" });
    expect(closed).toBe(true);
    expect(seams().__AIJ_POOL_CALLS__ ?? []).toEqual([]);
  });
});
