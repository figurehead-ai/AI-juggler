/**
 * AJ-E3011 round 3: New pool and the diagram (a made pool's inspector)
 * must say the same pool the same way. One pool — an App whose next
 * session leads, everyone in web joining as Member, Work together, the
 * team tools not asked yet — rendered through both paths; the lead
 * control, the team tools and "on their own" must have the same words
 * and the same controls, and the summary the same sentences as Review.
 * If either path drifts, this fails.
 */
import { cleanup, render } from "@testing-library/react";
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
import { PoolToolsCard } from "./PoolToolsCard";

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
  const lead = render(<NewPoolLead draft={draft} onDraft={() => undefined} sessions={[]} nameOf={nameOf} where={WHERE} openNow={new Map([["app:cli:work", 0]])} />);
  const leadArea = snapshot(lead.getByTestId("new-pool-lead"));
  lead.unmount();
  const own = render(<NewPoolOwn draft={draft} onDraft={() => undefined} where={WHERE} />);
  const ownArea = snapshot(own.getByTestId("new-pool-level"));
  own.unmount();
  const review = render(<NewPoolReview draft={draft} name="web team" where={WHERE} nameOf={nameOf} openNow={new Map([["app:cli:work", 0]])} projects={[{ id: "p-web", name: WHERE }]} />);
  const toolsArea = snapshot(await review.findByTestId("new-pool-tools"));
  const sentence = (row: string) => review.getByTestId("new-pool-review").querySelector(`[data-row="${row}"] dd`)?.textContent ?? "";
  const sentences = { lead: sentence("lead"), own: sentence("own"), tools: sentence("tools"), who: sentence("who") };
  review.unmount();
  return { lead: leadArea, own: ownArea, tools: toolsArea, sentences };
}

/** The diagram path: the pool's own inspector (its summary, the team
 * tools card and its setup sections). */
async function diagramPath() {
  const view = render(
    <>
      <PoolSummary shape={shapeOfPool(pool, rulesInfo, [], label)} where={WHERE} rules={rulesInfo.members} needs={0} tools={tools} onOpen={() => undefined} />
      <PoolToolsCard testId="pool-checklist-tools" projects={tools.projects} apps={tools.apps} />
      <PoolSetupSections pool={pool} rules={{ rules: rulesInfo, watched: [], reload: () => undefined }} members={[]} ruleLabel={label} where={WHERE} selection={{ kind: "pool" }} onAddMembers={() => undefined} />
    </>,
  );
  const lead = snapshot(view.getByTestId("pool-lead"));
  const own = snapshot(view.getByTestId("pool-own"));
  const toolsArea = snapshot(await view.findByTestId("pool-checklist-tools"));
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
    expect(setup.lead.radios.some((name) => name.startsWith(`${APP}'s next session | It leads from its next session in web.`))).toBe(true);
    expect(setup.lead.text).toContain(`${APP} leads from its next session in web (none is open now)`);
  });

  it("the team tools: the same card, words and buttons", async () => {
    const setup = await setupPath();
    cleanup();
    const diagram = await diagramPath();
    expect(diagram.tools).toEqual(setup.tools);
    expect(setup.tools.buttons).toEqual(expect.arrayContaining(["Turn on team tools", "Not now", "Settings › Agent access"]));
    expect(setup.tools.text).toContain("Let your assistants work as a team");
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
  });
});
