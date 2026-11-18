/**
 * D-126: New pool in the making screens' layout — the steps beside with
 * their values, the live preview matching the form, the start's side,
 * the first thing to tell the lead (a member's logo is
 * components/member-logo.test.tsx's).
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools } from "../../lib/pools";
import { LeadPrompt } from "./LeadPrompt";
import { NewPool } from "./NewPool";
import { NewPoolPreview, PreviewDiagram, PreviewStrip, type PreviewNode, type PreviewRow } from "./NewPoolPreview";
import { NewPoolStartSide } from "./NewPoolStartSide";
import { NewPoolSteps } from "./NewPoolSteps";
import { NewPoolTemplate } from "./NewPoolTemplate";
import { cardProjects, NewPoolWhere, PROJECT_CARDS, projectLine } from "./NewPoolWhere";
import { draftOf } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { StepEyebrow, StepHead } from "./StepHead";

beforeEach(async () => {
  localStorage.clear();
  resetPreviewPools();
  resetPreviewPoolModel();
  resetPools();
  await refreshPools();
});
afterEach(cleanup);

const nextTo = (step: string) => {
  for (let n = 0; n < 8 && !screen.queryByTestId(`new-pool-step-${step}`); n += 1) fireEvent.click(screen.getByTestId("new-pool-next"));
  expect(screen.getByTestId(`new-pool-step-${step}`)).toBeTruthy();
};

/** A step button by its step name (its name goes on with its value). */
const STEP = (name: string) => new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(\\s|$)`);

describe("the steps beside the form (D-126)", () => {
  // Each step's value under its name: what it's set to once walked, what
  // it's filled in with before; The basics say nothing yet.
  it("the steps say their values: walked ones what they're set to, the prefilled ones what they'll be", () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />);
    const value = (step: string) => screen.getAllByTestId("flow-step").find((one) => one.dataset.step === step)!.querySelector('[id$="-more"]')!.textContent;
    expect(value("where")).toBe("");
    expect(value("lead")).toBe("No lead yet");
    expect(value("talk")).toBe("Everyone, freely");
    expect(value("own")).toBe("Work together");
    nextTo("who");
    expect(value("template")).toBe("From scratch, done");
    expect(value("where")).toBe("ai-juggler, done");
    // How they work says it's prefilled until it's reached.
    expect(screen.getByTestId("new-pool-steps").textContent).toContain("prefilled");
    nextTo("lead");
    expect(screen.getByTestId("new-pool-steps").textContent).not.toContain("prefilled");
  });

  it("is a list of steps, the ones not reached yet unreachable, the current one marked", () => {
    let picked = -1;
    render(<NewPoolSteps steps={["template", "where", "who", "lead", "review"]} step={1} furthest={1} valueOf={() => null} onPick={(at) => (picked = at)} />);
    const nav = screen.getByRole("navigation", { name: "Steps" });
    expect(within(nav).getByRole("button", { name: STEP("Where") }).getAttribute("aria-current")).toBe("step");
    expect(within(nav).getByRole("button", { name: STEP("Who leads") })).toHaveProperty("disabled", true);
    fireEvent.click(within(nav).getByRole("button", { name: STEP("Start") }));
    expect(picked).toBe(0);
    expect(within(nav).getByRole("button", { name: "Read the 2-minute guide" })).toBeTruthy();
  });
});

describe("one primary (D-126)", () => {
  it("one filled primary in the whole view, on every step", () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />);
    for (let n = 0; n < 9; n += 1) {
      const step = screen.getByTestId("new-pool-step-title").textContent;
      const primaries = [...document.querySelectorAll("[data-primary]")].map((one) => `${step}: ${one.textContent}`);
      expect(primaries.length, primaries.join(" | ")).toBe(1);
      if (screen.queryByTestId("new-pool-step-review")) break;
      fireEvent.click(screen.getByTestId("new-pool-next"));
    }
  });
});

describe("the step's head", () => {
  it("says where it is from the wizard around it, its question, and a guide link where it has one", () => {
    render(
      <StepEyebrow.Provider value="Step 4 of 9 · How they work">
        <StepHead step="lead" />
      </StepEyebrow.Provider>,
    );
    expect(screen.getByText("Step 4 of 9 · How they work")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Who leads the team?" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "What a lead does" })).toBeTruthy();
  });
});

const rows: PreviewRow[] = [
  { id: "where", label: "Where", value: "web", pending: false, prefilled: false },
  { id: "who", label: "Who's in", value: "Claude and Codex, and every new session in web", pending: false, prefilled: false },
  { id: "lead", label: "Lead", value: "No lead yet", pending: true, prefilled: true },
];
const node = (ref: string, name: string, role: PreviewNode["role"]): PreviewNode => ({ ref, name, line: role, logo: { kind: "tile", icon: () => <svg /> }, role });

describe("the live preview", () => {
  it("is the pool's drawing and its settings in words, the step being set lit", () => {
    render(<NewPoolPreview name="web team" diagram={<PreviewDiagram lead={null} members={[node("app:a", "Claude", "member"), node("all", "Everyone in web", "member")]} reached focus="members" />} rows={rows} current="who" />);
    const preview = screen.getByRole("complementary", { name: "Preview" });
    expect(within(preview).getByText("web team")).toBeTruthy();
    // The drawing is decorative; the rows say it in words.
    expect(screen.getByTestId("new-pool-preview-diagram").getAttribute("aria-hidden")).toBe("true");
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual(["Where", "Who's in", "Lead"]);
    expect(screen.getAllByRole("definition")[2]!.textContent).toBe("No lead yet · prefilled");
    expect(screen.getAllByTestId("new-pool-preview-row").find((row) => row.dataset.current)?.dataset.row).toBe("who");
    // Who's in lights the members; with no lead, its place says so.
    expect(screen.getAllByTestId("new-pool-preview-node").map((one) => one.dataset.lit)).toEqual(["true", "true"]);
    expect(screen.getByTestId("new-pool-preview-ghost").textContent).toBe("No lead yet");
  });

  it("before Who's in, a ghost of the shape to come; past three members, the rest counted", () => {
    const view = render(<PreviewDiagram lead={null} members={[]} reached={false} focus={null} />);
    expect(view.getAllByTestId("new-pool-preview-ghost").map((one) => one.textContent)).toEqual(["Lead", "Members", "Members"]);
    view.unmount();
    render(<PreviewDiagram lead={node("app:a", "Claude", "lead")} members={["b", "c", "d", "e"].map((id) => node(`app:${id}`, id, "member"))} reached focus="lead" />);
    expect(screen.getAllByTestId("new-pool-preview-node").map((one) => one.dataset.ref)).toEqual(["app:a", "app:b", "app:c"]);
    expect(screen.getByTestId("new-pool-preview-ghost").textContent).toBe("2 more");
    expect(screen.getAllByTestId("new-pool-preview-node")[0]!.dataset.lit).toBe("true");
  });

  it("folds into a strip: the name, who's in and the lead", () => {
    render(<PreviewStrip name="web team" rows={rows} />);
    expect(screen.getByTestId("new-pool-summary").textContent).toBe("web team · Claude and Codex, and every new session in web · No lead yet");
  });

  it("matches the form: a member ticked in Who's in is on the drawing at once", () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} startWith="feature-team" onClose={() => undefined} />);
    nextTo("where");
    fireEvent.click(within(screen.getByTestId("new-pool-where")).getByText("Every project"));
    nextTo("who");
    const before = screen.queryAllByTestId("new-pool-preview-node").length;
    const row = screen.getAllByTestId("new-pool-candidate").find((one) => !one.querySelector("[role=checkbox]")!.hasAttribute("disabled"))!;
    fireEvent.click(within(row).getByRole("checkbox"));
    expect(screen.getAllByTestId("new-pool-preview-node")).toHaveLength(before + 1);
  });
});

describe("How to start (D-126 cards)", () => {
  it("is one radio group: Start from scratch first and suggested for a first pool, each template with its drawing; the arrows choose", () => {
    const chosen: string[] = [];
    const view = render(<NewPoolTemplate chosen="blank" onChoose={(id) => chosen.push(id)} first />);
    const group = screen.getByRole("radiogroup", { name: "How to start" });
    const scratch = within(group).getByRole("radio", { name: /^Start from scratch/ });
    expect(scratch).toHaveProperty("checked", true);
    expect(screen.getByText("Suggested for a first pool")).toBeTruthy();
    expect(within(group).getAllByRole("radio")).toHaveLength(9);
    expect(screen.getAllByTestId("new-pool-art-template")).toHaveLength(8);
    fireEvent.keyDown(scratch, { key: "ArrowDown" });
    expect(chosen).toEqual(["writer-reviewer"]);
    view.rerender(<NewPoolTemplate chosen="blank" onChoose={() => undefined} first={false} />);
    expect(screen.queryByText("Suggested for a first pool")).toBeNull();
  });
});

describe("Where (D-126 cards)", () => {
  const NOW = 1_800_000_000_000;
  const at = (id: string, project: string, status: string, ms: number): InstanceInfo => ({ ...MOCK_SESSIONS[0]!, id, project_id: project, provider: "claude-code", status, started_at_ms: ms, last_activity_ms: ms });
  const sessions = [at("a", "web", "running", NOW - 60_000), at("b", "web", "running", NOW - 30_000), at("c", "api", "finished", NOW - 3 * 3_600_000)];
  const projects = [
    { id: "web", name: "web", open: true },
    { id: "api", name: "api", open: false },
  ];

  it("each project's card says what's open in it now, or when it was last worked in", () => {
    expect(projectLine("web", sessions, NOW)).toMatch(/^2 sessions open now · /);
    expect(projectLine("api", sessions, NOW)).toBe("Nothing open now · last used 3 h ago");
    expect(projectLine("docs", sessions, NOW)).toBe("Nothing open now");
  });

  it("shows the chosen, then open now, then the latest, at most six; the rest a search away", () => {
    const many = Array.from({ length: 9 }, (_, n) => ({ id: `p${n}`, name: `p${n}`, open: n === 7 }));
    const shown = cardProjects(many, ["p8"], []).map((one) => one.id);
    expect(shown.slice(0, 2)).toEqual(["p8", "p7"]);
    expect(shown).toHaveLength(PROJECT_CARDS);
  });

  it("the projects are checkboxes; Every project takes them all in; none chosen asks for one", () => {
    let draft = { ...draftOf(templateById("blank"), "web") };
    const view = render(<NewPoolWhere draft={draft} onDraft={(next) => (draft = next)} projects={projects} sessions={sessions} suggestedName="web team" now={NOW} />);
    const group = screen.getByRole("group", { name: "Which project" });
    expect(within(group).getByRole("checkbox", { name: "web" })).toHaveProperty("checked", true);
    fireEvent.click(within(group).getByRole("checkbox", { name: "api" }));
    expect(draft.projects).toEqual(["web", "api"]);
    fireEvent.click(within(group).getByRole("checkbox", { name: "Every project" }));
    expect(draft.every).toBe(true);
    view.rerender(<NewPoolWhere draft={draft} onDraft={(next) => (draft = next)} projects={projects} sessions={sessions} suggestedName="web team" now={NOW} />);
    expect(within(group).getByRole("checkbox", { name: "web" })).toHaveProperty("disabled", true);
    view.rerender(<NewPoolWhere draft={{ ...draft, every: false, projects: [] }} onDraft={() => undefined} projects={projects} sessions={sessions} suggestedName="web team" now={NOW} />);
    expect(screen.getByTestId("new-pool-where-words").dataset.level).toBe("needs-you");
    expect(screen.getByRole("textbox", { name: "Pool name" })).toHaveProperty("placeholder", "web team");
  });
});

describe("How to start's side", () => {
  it("says what a pool does, in four lines, with the guide", () => {
    render(<NewPoolStartSide first />);
    const side = screen.getByRole("complementary", { name: "What a pool does" });
    expect(within(side).getAllByRole("listitem")).toHaveLength(4);
    expect(within(side).getByRole("button", { name: "How pools work" })).toBeTruthy();
  });
});

describe("the first thing to tell the lead (Review)", () => {
  it("is built from the roles, with Copy; nothing without a lead", () => {
    const members = [
      { ref: "app:claude", label: "Claude · Work", role: "member" as const, level: null, later: true },
      { ref: "app:codex", label: "Codex · Personal", role: "reviewer" as const, level: null, later: true },
    ];
    const view = render(<LeadPrompt shape={{ members, leadNext: "app:claude" }} where="web" testId="p" />);
    expect(screen.getByRole("region", { name: "First thing to tell your lead" })).toBeTruthy();
    expect(screen.getByTestId("p-words").textContent).toBe("Plan the work in web, hand out tasks, and ask Codex to review.");
    expect(screen.getByRole("button", { name: "Copy what to tell Claude · Work" })).toBeTruthy();
    view.unmount();
    const none = render(<LeadPrompt shape={{ members, leadNext: null }} where="web" testId="p" />);
    expect(none.container.textContent).toBe("");
  });
});
