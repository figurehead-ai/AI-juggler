/**
 * D-126: How they work and Finish as the mock-ups draw them — Who leads,
 * the roles, the messages presets, On their own, the team tools and
 * Review — each as the kit's cards, with the same words and choices as
 * before (paths-parity keeps the diagram's alike).
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetPreviewAgentBridge } from "../../lib/ipc/agent-bridge";
import { draftOf, type Draft } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { LeadCards } from "./LeadCards";
import { NewPoolLead } from "./NewPoolLead";
import { NewPoolTalk } from "./NewPoolTalk";
import { NewPoolOwn } from "./NewPoolOwn";
import { NewPoolTools } from "./NewPoolTools";
import { NewPoolReview } from "./NewPoolReview";
import { NewPoolSteps } from "./NewPoolSteps";

beforeEach(() => resetPreviewAgentBridge());
afterEach(cleanup);

const app = (ref: string, label: string, role: "member" | "reviewer" = "member") => ({ ref, kind: "app" as const, label, role, level: null, reach: "later" as const });
const NO_LOGOS = { apps: [], sessions: [] };

// AJ-E2311 item 5: the wizard's steps couldn't be pressed from its rail,
// and a step not yet open said nothing.
describe("the steps' rail", () => {
  it("the next step is one press away, as Next is; one further says after which", () => {
    const steps = ["template", "where", "who", "lead", "everyone", "talk", "own", "review"] as const;
    const picked: number[] = [];
    render(<NewPoolSteps steps={steps} step={0} furthest={0} valueOf={() => null} onPick={(at) => picked.push(at)} />);
    const rail = screen.getAllByTestId("flow-step");
    const at = (id: string) => rail.find((one) => one.dataset.step === id)!;
    fireEvent.click(at("where"));
    expect(picked).toEqual([1]);
    expect(at("lead").hasAttribute("disabled")).toBe(true);
    expect(at("lead").getAttribute("title")).toBe("After Where");
    expect(screen.getByRole("button", { name: /Who leads.*After Where/ })).toBe(at("lead"));
  });
});

describe("Who may message whom (D-126 presets)", () => {
  it("three ways as cards set the whole table; a cell changed by hand is its own mix, no way chosen", () => {
    let draft: Draft = draftOf(templateById("blank"), "web");
    const view = render(<NewPoolTalk draft={draft} onDraft={(next) => (draft = next)} />);
    const ways = screen.getByRole("radiogroup", { name: "Who may message whom" });
    expect(within(ways).getByRole("radio", { name: "Everyone talks freely" })).toHaveProperty("checked", true);
    fireEvent.click(within(ways).getByRole("radio", { name: "Members ask you first" }));
    expect(draft.talk["member>member"]).toBe("ask");
    view.rerender(<NewPoolTalk draft={draft} onDraft={(next) => (draft = next)} />);
    const cell = screen.getAllByTestId("new-pool-talk-cell").find((one) => one.dataset.pair === "member>member")!;
    expect(cell.dataset.rule).toBe("ask");
    fireEvent.click(screen.getAllByTestId("new-pool-talk-cell").find((one) => one.dataset.pair === "lead>member")!);
    view.rerender(<NewPoolTalk draft={draft} onDraft={(next) => (draft = next)} />);
    expect(within(ways).getAllByRole("radio").filter((radio) => radio instanceof HTMLInputElement && radio.checked)).toEqual([]);
    expect(screen.getByTestId("new-pool-talk-mix").textContent).toBe("Your own mix");
    // Feature team's table is Members ask you first.
    view.rerender(<NewPoolTalk draft={draftOf(templateById("feature-team"), "web")} onDraft={() => undefined} />);
    expect(within(ways).getByRole("radio", { name: "Members ask you first" })).toHaveProperty("checked", true);
  });
});

describe("On their own (D-126 cards)", () => {
  it("three ways as cards with a drawing each, the chosen one's example and the wake one's cost said; the seal a switch", () => {
    let draft: Draft = draftOf(templateById("blank"), "web");
    const view = render(<NewPoolOwn draft={draft} onDraft={(next) => (draft = next)} where="web" />);
    const ways = screen.getByRole("radiogroup", { name: "What sessions in this pool may do on their own" });
    expect(within(ways).getAllByRole("radio").map((radio) => document.getElementById(radio.getAttribute("aria-labelledby")!)?.textContent)).toEqual(["Look only", "Work together", "Work together and wake each other"]);
    expect(within(ways).getByRole("radio", { name: "Work together" })).toHaveProperty("checked", true);
    expect(screen.getByTestId("new-pool-level-example").textContent).toMatch(/^Example: /);
    expect(screen.getByTestId("new-pool-level-example-full").textContent).toMatch(/^Example: /);
    fireEvent.keyDown(within(ways).getByRole("radio", { name: "Work together" }), { key: "ArrowUp" });
    expect(draft.level).toBe("read");
    const seal = screen.getByRole("switch", { name: "Keep this pool's work apart" });
    fireEvent.click(seal);
    expect(draft.sealed).toBe(true);
    view.rerender(<NewPoolOwn draft={draft} onDraft={(next) => (draft = next)} where="web" />);
    expect(screen.getByRole("switch", { name: "Keep this pool's work apart" }).getAttribute("aria-checked")).toBe("true");
  });
});

describe("Team tools (D-126 cards)", () => {
  it("the question drawn, what it gives, costs and changes side by side, its two answers as cards (no filled primary)", async () => {
    render(<NewPoolTools />);
    const question = await screen.findByRole("region", { name: "Let the sessions in your pools message each other and share notes and tasks?" });
    expect(within(question).getByTestId("new-pool-art-tools")).toBeTruthy();
    expect(within(question).getAllByRole("button").map((one) => one.textContent)).toEqual(["Settings", "Yes, let them work together", "Not now"]);
    expect(question.querySelectorAll("[data-primary]")).toHaveLength(0);
  });
});

describe("Review (D-126)", () => {
  const draft = (): Draft => ({ ...draftOf(templateById("blank"), "web"), members: [app("app:cli:work", "Claude Code · Work"), app("app:cli:codex", "Codex · Personal", "reviewer")], leadNext: "app:cli:work" });

  it("says first what still keeps it from working (team tools not answered), then each sentence with its Change", async () => {
    const steps: string[] = [];
    render(<NewPoolReview draft={draft()} name="web team" where="web" onChange={(step) => steps.push(step)} />);
    expect(screen.getByRole("heading", { name: "Ready to make “web team”" })).toBeTruthy();
    await screen.findByTestId("new-pool-review-tools-line");
    expect(screen.getByTestId("new-pool-review-state").dataset.state).toBe("needs");
    expect(screen.getByTestId("new-pool-review-state").textContent).toContain("Not answered yet: they can't message each other until you say yes.");
    // Turn on beside it, in the state card (round 4).
    expect(screen.getByTestId("new-pool-review-state-tools-on")).toBeTruthy();
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual(["Where", "Who's in", "Lead", "Roles", "Messages", "On their own", "Kept apart"]);
    fireEvent.click(screen.getByRole("button", { name: "Change roles" }));
    fireEvent.click(screen.getByRole("button", { name: "Change kept apart" }));
    expect(steps).toEqual(["everyone", "own"]);
  });

  // The rc3 critique: Roles read three sentences about each member, the
  // canvas's own; Review says each row in one short sentence, as mocked,
  // every rule in full one click away under them.
  it("each row is one short sentence; every rule in full is one click away; what happens next still shows", async () => {
    render(<NewPoolReview draft={draft()} name="web team" where="web" />);
    await screen.findByTestId("new-pool-review-tools-line");
    const rows = within(screen.getByTestId("new-pool-review-rows"));
    expect(rows.getAllByTestId("new-pool-review-line").map((line) => line.textContent)).toEqual([
      "“web team” works in web.",
      "Claude Code and Codex.",
      "Claude Code's next session in web leads.",
      "Codex checks the work. Everyone else builds.",
      "Everyone may message everyone.",
      "They work together. Anything more asks you.",
      "No. It can link to your other pools.",
    ]);
    const full = screen.getByTestId("new-pool-review-full");
    expect(within(full).getByRole("button", { name: "Every rule in full" }).getAttribute("aria-expanded")).toBe("false");
    expect(screen.getByTestId("new-pool-review").querySelector('[data-row="everyone"] dd')?.textContent).toContain("Codex · Personal is a Reviewer and may Work together, as the pool.");
    expect(screen.getAllByTestId("new-pool-review-next-step")).toHaveLength(3);
  });

  it("with the team tools on and nothing waiting: everything it needs is set", async () => {
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: { asked: true, on: true } });
    resetPreviewAgentBridge();
    render(<NewPoolReview draft={draft()} name="web team" where="web" />);
    await waitFor(() => expect(screen.getByTestId("new-pool-review-state").dataset.state).toBe("set"));
    expect(screen.getByTestId("new-pool-review-state").textContent).toBe("Everything it needs is setTeam tools are on. Nothing is waiting on you.");
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: undefined });
  });
});

describe("Who leads (D-126 cards)", () => {
  it("one radio group of cards with logos: open sessions, each App's next session, no lead; the arrows choose", () => {
    const picked: string[] = [];
    render(
      <LeadCards
        sessions={[{ ref: "session:s1", name: "Planner", detail: "web · Claude Code · working" }]}
        apps={[{ ref: "app:cli:a", name: "Claude Code · Work" }, { ref: "app:cli:b", name: "Codex · Personal", blocked: "Can't lead: its app can't take the connector." }]}
        value={{ kind: "next", app: "app:cli:a" }}
        onChange={(choice) => picked.push(choice.kind === "next" ? choice.app : choice.kind)}
        where="web"
        logos={NO_LOGOS}
      />,
    );
    const group = screen.getByRole("radiogroup", { name: "Who leads" });
    expect(within(group).getAllByRole("radio").map((radio) => radio.getAttribute("aria-labelledby") && document.getElementById(radio.getAttribute("aria-labelledby")!)?.textContent)).toEqual([
      "Planner",
      "Claude Code · Work's next session",
      "Codex · Personal's next session",
      "No lead for now",
    ]);
    expect(within(group).getByRole("radio", { name: "Claude Code · Work's next session" })).toHaveProperty("checked", true);
    expect(within(group).getByRole("radio", { name: "Codex · Personal's next session" })).toHaveProperty("disabled", true);
    expect(screen.getAllByTestId("member-logo")).toHaveLength(3);
    expect(screen.getByTestId("lead-cards-sentence").textContent).toBe("Claude Code · Work takes charge as soon as one of its sessions starts in web: it plans the work and hands out tasks.");
    // The arrows skip the one that can't lead.
    fireEvent.keyDown(within(group).getByRole("radio", { name: "Claude Code · Work's next session" }), { key: "ArrowDown" });
    expect(picked).toEqual(["none"]);
  });

  it("the step draws what a lead does in three steps, then the cards", () => {
    let draft: Draft = { ...draftOf(templateById("blank"), "web"), members: [app("app:cli:a", "Claude Code · Work")] };
    render(<NewPoolLead draft={draft} onDraft={(next) => (draft = next)} sessions={[]} nameOf={(_, label) => label} where="web" />);
    expect(within(screen.getByTestId("new-pool-lead-how")).getAllByRole("listitem").map((one) => one.textContent)).toEqual(["1You tell the lead what you want.", "2It splits that into tasks for the others.", "3You approve anything that needs you."]);
    fireEvent.click(screen.getByRole("radio", { name: "Claude Code · Work's next session" }));
    expect(draft.leadNext).toBe("app:cli:a");
  });
});
