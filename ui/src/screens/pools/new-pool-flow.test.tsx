/**
 * D-126: New pool in the making screens' layout — the steps beside with
 * their values, the live preview matching the form, the start's side,
 * the first thing to tell the lead, and each member's logo.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools } from "../../lib/pools";
import type { AppModel } from "../../lib/apps";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { LeadPrompt } from "./LeadPrompt";
import { logoOf, MemberLogo } from "./MemberLogo";
import { NewPool } from "./NewPool";
import { NewPoolPreview, PreviewDiagram, PreviewStrip, type PreviewNode, type PreviewRow } from "./NewPoolPreview";
import { NewPoolStartSide } from "./NewPoolStartSide";
import { NewPoolSteps } from "./NewPoolSteps";
import { NewPoolTemplate } from "./NewPoolTemplate";
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
    expect(value("template")).toBe("From scratch · done");
    expect(value("where")).toBe("ai-juggler · done");
    // How they work says it's prefilled until it's reached.
    expect(screen.getByTestId("new-pool-steps").textContent).toContain("prefilled");
    nextTo("lead");
    expect(screen.getByTestId("new-pool-steps").textContent).not.toContain("prefilled");
  });

  it("is a list of steps, the ones not reached yet unreachable, the current one marked", () => {
    let picked = -1;
    render(<NewPoolSteps steps={["template", "where", "who", "lead", "review"]} step={1} furthest={1} valueOf={() => null} onPick={(at) => (picked = at)} />);
    const nav = screen.getByRole("navigation", { name: "Steps" });
    expect(within(nav).getByRole("button", { name: "Where" }).getAttribute("aria-current")).toBe("step");
    expect(within(nav).getByRole("button", { name: "Who leads" })).toHaveProperty("disabled", true);
    fireEvent.click(within(nav).getByRole("button", { name: "Start" }));
    expect(picked).toBe(0);
    expect(within(nav).getByRole("button", { name: "Read the 2-minute guide" })).toBeTruthy();
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

describe("a member's logo", () => {
  const app = (id: string, kind: AppModel["kind"], tool: string, appId: string, name: string): AppModel => ({ id, kind, tool, appId, app: name, profile: null, name, defaultName: name, named: false, railKey: id, folder: null, accountId: null, place: null, slot: 0 });
  const apps = [app("cli:work", "cli", "claude-code", "claude-app", "Claude Code"), app("app:codex-app:/p", "desktop", "codex", "codex-app", "Codex")];
  const sessions: InstanceInfo[] = [{ ...MOCK_SESSIONS[0]!, id: "s1", provider: "claude-code", app_id: null }];

  it("is its tool's logo for an App or a session (a CLI with its badge), a tile for the rest", () => {
    expect(logoOf("app:cli:work", { apps, sessions })).toEqual({ kind: "tool", tool: "claude-code", look: "cli", name: "Claude Code" });
    expect(logoOf("app:app:codex-app:/p", { apps, sessions })).toMatchObject({ kind: "tool", tool: "codex", look: "desktop", appId: "codex-app" });
    expect(logoOf("session:s1", { apps, sessions })).toMatchObject({ kind: "tool", tool: "claude-code", look: "cli" });
    for (const ref of ["all", "account:a", "mac:m", "session:gone"]) expect(logoOf(ref, { apps, sessions }).kind).toBe("tile");
  });

  it("carries the lead's crown or a reviewer's shield at its corner, and nothing for a member", () => {
    const view = render(<MemberLogo spec={logoOf("all", { apps, sessions })} badge="lead" />);
    expect(view.getByTestId("member-logo-role").dataset.role).toBe("lead");
    view.rerender(<MemberLogo spec={logoOf("all", { apps, sessions })} badge="member" />);
    expect(view.queryByTestId("member-logo-role")).toBeNull();
  });
});
