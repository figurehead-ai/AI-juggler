/**
 * AJ-E3011 round 3: New pool and the diagram (a made pool's inspector)
 * must say the same pool the same way. One pool — an App whose next
 * session leads, everyone in web joining as Member, Work together, the
 * team tools not asked yet — rendered through both paths; the lead
 * control, the team tools and "on their own" must have the same words
 * and the same controls, and the summary the same sentences as Review.
 * If either path drifts, this fails.
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetPreviewAgentBridge } from "../../../lib/ipc/agent-bridge";
import { scopeOf, type PoolInfo } from "../../../lib/ipc/pools";
import type { PoolRuleInfo, PoolRulesInfo } from "../../../lib/ipc/pool-model";
import { draftOf, type Draft } from "../../../lib/new-pool";
import { templateById } from "../../../lib/pool-templates";
import { NewPoolLead } from "../../../screens/pools/NewPoolLead";
import { NewPoolOwn } from "../../../screens/pools/NewPoolOwn";
import { NewPoolReview } from "../../../screens/pools/NewPoolReview";
import { PoolSetupSections } from "../PoolSetupSections";
import { PoolSummary, shapeOfPool } from "../PoolSummary";
import { nextLine, shapeOfDraft } from "../../../lib/pool-sentences";
import { NewPoolTools } from "../../../screens/pools/NewPoolTools";
import { agentBridgeAnswer } from "../../../lib/ipc/agent-bridge";
import { TeamToolsQuestion } from "./TeamTools";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../../preview/fixtures";
import { NewPool } from "../../../screens/pools/NewPool";
import { EmptyPoolSetup, existingOf } from "../EmptyPoolSetup";
import { opensInSetup } from "../mode";
import { SetupView } from "../PoolOverview";
import { setupRows } from "../../../lib/pool-setup";
import { stepValue } from "../../../lib/pool-wizard-words";
import { SETUP_STEPS, STEP_WORDS } from "../../../lib/pool-words";

beforeEach(() => resetPreviewAgentBridge());
afterEach(cleanup);

const APP = "Claude Code · Work";
const WHERE = "web";
const label = (rule: PoolRuleInfo) => (rule.member === "all" ? "Everyone in web" : APP);

const draft: Draft = {
  ...draftOf(templateById("blank"), "p-web"),
  members: [
    { ref: "app:cli:work", kind: "app", label: APP, role: "member", level: null, reach: "later" },
    { ref: "all", kind: "everyone", label: "Everyone in this pool's projects", role: "member", level: null, reach: "later" },
  ],
  leadNext: "app:cli:work",
};
const nameOf = (ref: string, text: string) => (ref === "all" ? "Everyone in web" : text);

const pool: PoolInfo = { id: "pool-web", name: "web team", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0, ...scopeOf("user", null), scope_all: false, scope: ["p-web"] };
const rule = (member: string, kind: string): PoolRuleInfo => ({ member, kind, role: "member", level: null, joined_at: 1, sessions: 0, session_ids: [] });
const rulesInfo: PoolRulesInfo = {
  pool_id: pool.id,
  members: [rule("app:cli:work", "app"), rule("all", "everyone")],
  talk: [
    { from: "role:observer", to: "role:lead", rule: "blocked" },
    { from: "role:observer", to: "role:member", rule: "blocked" },
    { from: "role:observer", to: "role:reviewer", rule: "blocked" },
    { from: "role:observer", to: "role:observer", rule: "blocked" },
  ],
  lead_next: "cli:work",
};
const tools = { projects: [{ id: "p-web", name: WHERE }], apps: [{ id: "work", name: APP }] };

/** An area's words and controls: its radios' names and the hints that
 * describe them, its buttons' names, and its text, testids aside. What
 * only a made pool can offer (Start one now) is left out. */
function snapshot(shown: HTMLElement): { radios: string[]; buttons: string[]; text: string } {
  const area = shown.cloneNode(true);
  if (!(area instanceof HTMLElement)) throw new Error("not an element");
  for (const only of area.querySelectorAll("[data-made-pool-only]")) only.remove();
  document.body.append(area);
  const named = (element: Element) => {
    const by = element.getAttribute("aria-labelledby");
    const describedBy = (element.getAttribute("aria-describedby") ?? "").split(" ").filter(Boolean);
    const name = by ? (document.getElementById(by)?.textContent ?? "") : (element.getAttribute("aria-label") ?? element.textContent ?? "");
    return [name, ...describedBy.map((id) => document.getElementById(id)?.textContent ?? "")].join(" | ");
  };
  const out = {
    radios: [...area.querySelectorAll('input[type="radio"]')].map(named),
    buttons: [...area.querySelectorAll("button")].map((button) => (button.textContent ?? "").trim()),
    text: (area.textContent ?? "").replace(/\s+/g, " ").trim(),
  };
  area.remove();
  return out;
}

/** The setup path's three areas and its Review sentences. */
async function setupPath() {
  const lead = render(<NewPoolLead draft={draft} onDraft={() => undefined} sessions={[]} nameOf={nameOf} where={WHERE} />);
  const leadArea = snapshot(lead.getByTestId("new-pool-lead"));
  lead.unmount();
  const own = render(<NewPoolOwn draft={draft} onDraft={() => undefined} where={WHERE} />);
  const ownArea = snapshot(own.getByTestId("new-pool-level"));
  own.unmount();
  // D-30's question: New pool's own step (round 4).
  const step = render(<NewPoolTools />);
  const toolsArea = snapshot(await step.findByTestId("new-pool-tools"));
  step.unmount();
  const review = render(<NewPoolReview draft={draft} name="web team" where={WHERE} nameOf={nameOf} openNow={new Map([["app:cli:work", 0]])} projects={[{ id: "p-web", name: WHERE }]} />);
  await review.findByTestId("new-pool-review-tools-line");
  const sentence = (row: string) => review.getByTestId("new-pool-review").querySelector(`[data-row="${row}"] dd`)?.textContent ?? "";
  const sentences = { lead: sentence("lead"), own: sentence("own"), tools: sentence("tools"), who: sentence("who") };
  review.unmount();
  return { lead: leadArea, own: ownArea, tools: toolsArea, sentences };
}

/** The diagram path: the pool page's question at its top, and the pool's
 * own inspector (its summary and its setup sections). */
async function diagramPath() {
  const view = render(
    <>
      <PoolSummary shape={shapeOfPool(pool, rulesInfo, [], label)} where={WHERE} needs={0} tools={tools} onOpen={() => undefined} />
      <TeamToolsQuestion testId="pool-team-tools" projects={tools.projects} apps={tools.apps} />
      <PoolSetupSections pool={pool} rules={{ rules: rulesInfo, watched: [], reload: () => undefined }} members={[]} ruleLabel={label} where={WHERE} selection={{ kind: "pool" }} onAddMembers={() => undefined} />
    </>,
  );
  const lead = snapshot(view.getByTestId("pool-lead"));
  const own = snapshot(view.getByTestId("pool-own"));
  const toolsArea = snapshot(await view.findByTestId("pool-team-tools"));
  await view.findByTestId("pool-summary-sentences-tools-line");
  const sentence = (row: string) => view.getByTestId("pool-summary-sentences").querySelector(`[data-row="${row}"] dd`)?.textContent ?? "";
  return { lead, own, tools: toolsArea, sentences: { lead: sentence("lead"), own: sentence("own"), tools: sentence("tools"), who: sentence("who") } };
}

describe("AJ-E3011 round 3: New pool and the diagram say one pool the same way", () => {
  it("the lead control: the same choices, hints and read-back", async () => {
    const setup = await setupPath();
    cleanup();
    const diagram = await diagramPath();
    // A made pool also offers Start one now (no session can start into a
    // pool that isn't made); the rest is the same control.
    expect(diagram.lead).toEqual(setup.lead);
    expect(setup.lead.radios.some((name) => name.startsWith(`${APP}'s next session | It takes charge as soon as one of its sessions starts in web.`))).toBe(true);
    expect(setup.lead.text).toContain(`${APP} takes charge as soon as one of its sessions starts in web`);
  });

  it("the team tools: D-30's one question, the same card, words and buttons on both paths while unanswered", async () => {
    const setup = await setupPath();
    cleanup();
    const diagram = await diagramPath();
    expect(diagram.tools).toEqual(setup.tools);
    expect(setup.tools.buttons).toEqual(["Settings", "Yes, let them work together", "Not now"]);
    expect(setup.tools.text).toContain("Let the sessions in your pools message each other and share notes and tasks?");
    expect(setup.tools.text).toContain("nothing extra");
    // The sentence beside it, with its one-click fix, in Review and the summary alike.
    expect(diagram.sentences.tools).toBe(setup.sentences.tools);
    expect(setup.sentences.tools).toContain("Turn on");
  });

  it("answered Not now: no question on either path, the same quiet line with Turn on", async () => {
    await agentBridgeAnswer(false);
    resetPreviewAgentBridge();
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: { asked: true, on: false } });
    resetPreviewAgentBridge();
    const step = render(<NewPoolTools />);
    expect(await step.findByTestId("new-pool-tools-quiet")).toBeTruthy();
    expect(step.queryByTestId("new-pool-tools")).toBeNull();
    const setupLine = snapshot(step.getByTestId("new-pool-tools-quiet"));
    step.unmount();
    const page = render(<TeamToolsQuestion testId="pool-team-tools" projects={tools.projects} apps={tools.apps} />);
    expect(await page.findByTestId("pool-team-tools-quiet")).toBeTruthy();
    expect(page.queryByTestId("pool-team-tools")).toBeNull();
    expect(snapshot(page.getByTestId("pool-team-tools-quiet"))).toEqual(setupLine);
    expect(setupLine.buttons).toEqual(["Turn on"]);
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: undefined });
  });

  it("on their own: the same choice, its scope line and the example", async () => {
    const setup = await setupPath();
    cleanup();
    const diagram = await diagramPath();
    expect(diagram.own.radios).toEqual(setup.own.radios);
    expect(diagram.own.text).toBe(setup.own.text);
    expect(setup.own.text).toContain("This is how they work together through AI Juggler.");
  });

  it("the diagram's summary is Review's sentences, word for word", async () => {
    const setup = await setupPath();
    cleanup();
    const diagram = await diagramPath();
    expect(diagram.sentences.lead).toBe(setup.sentences.lead);
    expect(diagram.sentences.own).toBe(setup.sentences.own);
    expect(diagram.sentences.tools).toBe(setup.sentences.tools);
    expect(diagram.sentences.who).toBe(setup.sentences.who);
    // Round 4 (#3): what happens next, from the same shapes.
    expect(nextLine(shapeOfPool(pool, rulesInfo, [], label), WHERE)).toBe(nextLine(shapeOfDraft(draft, nameOf), WHERE));
  });
});

// AJ-E3011 round 5: an empty pool's page is New pool's own flow, in place,
// prefilled with the pool's projects (and a project's own pool keeps its
// scope). Same steps, same step components, the same controls; if either
// drifts, this fails.
describe("AJ-E3011 round 5: an empty pool's setup is New pool's flow", () => {
  // D-126: the steps are the kit's FlowSteps; each says its value too, which
  // a prefilled pool's differs in by design, so the same steps means the
  // same ids and the same names.
  const steps = () =>
    [...document.querySelectorAll<HTMLElement>('[data-testid="flow-step"]')].map(
      (step) => `${step.dataset.step}: ${document.getElementById(step.getAttribute("aria-labelledby") ?? "")?.textContent ?? ""}`,
    );
  const stepTitle = () => document.querySelector('[data-testid="new-pool-step-title"]')?.textContent;

  it("the same steps and the same first step, in place, with Show the diagram for Cancel", async () => {
    const shown = render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />);
    await waitFor(() => expect(steps().length).toBeGreaterThan(0));
    const fromNew = { steps: steps(), title: stepTitle(), dialog: shown.getByTestId("new-pool").tagName };
    shown.unmount();
    let diagram = 0;
    const empty: PoolInfo = { ...pool, scope: ["proj-ai-juggler"] };
    const page = render(<EmptyPoolSetup pool={empty} rules={{ rules: { ...rulesInfo, members: [] }, watched: [], reload: () => undefined }} projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} accounts={[]} onDiagram={() => (diagram += 1)} />);
    await waitFor(() => expect(steps().length).toBeGreaterThan(0));
    expect(steps()).toEqual(fromNew.steps);
    expect(stepTitle()).toBe(fromNew.title);
    expect(fromNew.dialog).toBe("DIALOG");
    expect(page.getByTestId("new-pool").getAttribute("data-embedded")).toBe("true");
    expect(page.queryByTestId("new-pool-cancel")).toBeNull();
    fireEvent.click(page.getByTestId("new-pool-show-diagram"));
    expect(diagram).toBe(1);
  });

  it("prefilled: a pool's projects in Where; a project's own pool says its scope and keeps everyone in it", async () => {
    const web = MOCK_PROJECTS[0]!;
    const own: PoolInfo = { ...pool, id: "pool-own", kind: "default", project_id: web.id, ...scopeOf("default", web.id) };
    const view = render(<EmptyPoolSetup pool={own} rules={{ rules: { ...rulesInfo, members: [{ ...rule("all", "everyone"), role: "reviewer" }], lead_next: null }, watched: [], reload: () => undefined }} projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} accounts={[]} onDiagram={() => undefined} />);
    fireEvent.click(await view.findByTestId("new-pool-next"));
    expect(view.getByTestId("new-pool-where-fixed").textContent).toBe(`Every session in ${web.name} is part of this team.`);
    expect(view.getByTestId("new-pool-name")).toHaveProperty("value", `${web.name}'s own pool`);
    // Everyone else: its join role is the pool's own (here Reviewer).
    expect(existingOf(own, [{ ...rule("all", "everyone"), role: "reviewer" }])).toEqual({ every: false, projects: [web.id], everyoneRole: "reviewer", fixedScope: true });
    expect(existingOf({ ...pool, scope: ["a", "b"] }, [])).toEqual({ every: false, projects: ["a", "b"], everyoneRole: null, fixedScope: false });
  });

  it("opens in the setup only while empty, its rules read, and the diagram not chosen", () => {
    expect(opensInSetup({ peopled: false, rulesRead: true, diagramChosen: false })).toBe(true);
    expect(opensInSetup({ peopled: true, rulesRead: true, diagramChosen: false })).toBe(false);
    expect(opensInSetup({ peopled: false, rulesRead: false, diagramChosen: false })).toBe(false);
    expect(opensInSetup({ peopled: false, rulesRead: true, diagramChosen: true })).toBe(false);
  });
});

// D-126: each step's value under its name — New pool's steps and the
// canvas's checklist — is one function of one shape. The same pool (an
// App whose next session leads, Codex reviewing, everyone in web, Members
// ask you first) made both ways must read the same, word for word: before
// the consolidation the checklist said "Codex · Personal checks the work"
// and "Some ask you first" where the steps said "Codex checks the work" and
// "Members ask you first".
describe("D-126: the wizard's steps and the diagram's checklist say each value the same", () => {
  const NAMES: Record<string, string> = { "app:cli:work": APP, "app:app:codex": "Codex · Personal", all: "Everyone in web" };
  const team: Draft = {
    ...draftOf(templateById("feature-team"), "p-web"),
    members: [
      { ref: "app:cli:work", kind: "app", label: APP, role: "member", level: null, reach: "later" },
      { ref: "app:app:codex", kind: "app", label: NAMES["app:app:codex"]!, role: "reviewer", level: null, reach: "later" },
      { ref: "all", kind: "everyone", label: "Everyone", role: "member", level: null, reach: "later" },
    ],
    leadNext: "app:cli:work",
  };
  const made: PoolRulesInfo = {
    pool_id: pool.id,
    members: [rule("app:cli:work", "app"), { ...rule("app:app:codex", "app"), role: "reviewer" }, rule("all", "everyone")],
    talk: [...rulesInfo.talk, { from: "role:member", to: "role:member", rule: "ask" }],
    lead_next: "cli:work",
  };

  it("each of the six, as the checklist shows it under the step's name", () => {
    const shape = shapeOfDraft(team, (ref, text) => NAMES[ref] ?? text);
    const wizard = SETUP_STEPS.map((step) => `${STEP_WORDS[step].short}: ${stepValue(step, { shape, where: WHERE, template: null, tools: null })}`);
    const view = render(
      <SetupView
        pool={pool}
        canvas={null}
        members={[]}
        rules={{ rules: made, watched: [], reload: () => undefined }}
        ruleLabel={(one) => NAMES[one.member]!}
        where={WHERE}
        needs={0}
        now={0}
        setup={{ rows: setupRows({ own: false, every: false, whoIn: true, led: false, leadNext: true }, new Set(SETUP_STEPS)), onOpen: () => undefined, tools: { projects: [], apps: [] } }}
        progress={{ tools: null, toolsItem: false, blockers: [], done: 6, total: 6, complete: false }}
        onSelect={() => undefined}
      />,
    );
    const checklist = view.getAllByTestId("pool-checklist-step").map((row) => `${STEP_WORDS[SETUP_STEPS.find((step) => step === row.dataset.step)!].short}: ${row.querySelector('[data-testid="pool-checklist-value"]')?.textContent ?? ""}`);
    expect(checklist).toEqual(wizard);
    expect(wizard).toEqual(["Where: web", "Who's in: Claude Code, Codex, everyone in web", "Who leads: Claude Code's next session", "Roles: Codex checks the work", "Messages: Members ask you first", "On their own: Work together"]);
  });
});
