/**
 * AJ-E1704 vitest — the agent tools in sessions (D-30).
 *
 * - The one-time question shows until it's answered, never again after;
 *   "Yes, let them work together" turns the tools on, "Not now" leaves
 *   them off with one quiet line and Turn on.
 * - Settings › Agent access: the question until answered, then one
 *   switch for every session and, behind a fold read only when it opens,
 *   a switch per project and per profile.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MOCK_PROJECTS } from "../preview/fixtures";
import { resetPreviewAgentBridge } from "../lib/ipc/agent-bridge";
import { AgentToolsSettings } from "./AgentTools";
import { TEAM_TOOLS, teamTools, teamToolsLine } from "../lib/team-tools";
import { TeamToolsQuestion } from "../pages/pool/setup/TeamTools";

type Seams = { __AIJ_AGENT_BRIDGE__?: unknown; __AIJ_AGENT_BRIDGE_CALLS__?: string[] };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_AGENT_BRIDGE_CALLS__ ?? [];

beforeEach(() => {
  delete seams().__AIJ_AGENT_BRIDGE__;
  delete seams().__AIJ_AGENT_BRIDGE_CALLS__;
  resetPreviewAgentBridge();
});
afterEach(() => cleanup());

describe("the one-time question (D-30; AJ-E3011 round 4: one component everywhere)", () => {
  it("asks once, and Yes turns the tools on; then it's gone", async () => {
    const { findByTestId, queryByTestId } = render(<TeamToolsQuestion />);
    const card = await findByTestId("team-tools");
    expect(card.querySelector("h3")?.textContent).toBe(TEAM_TOOLS.question);
    expect(TEAM_TOOLS.question).toBe("Let the sessions in your pools message each other and share notes and tasks?");
    expect(card.textContent).toContain("It costs nothing extra");
    expect(card.textContent).toContain("This is for every pool you make; you can switch it off for a project or an app in Settings.");
    await act(async () => fireEvent.click(await findByTestId("team-tools-yes")));
    await waitFor(() => expect(queryByTestId("team-tools")).toBeNull());
    expect(queryByTestId("team-tools-quiet")).toBeNull();
    expect(calls()).toEqual(["answer:true"]);
  });

  it("Not now never asks again, and leaves one quiet line with Turn on", async () => {
    const first = render(<TeamToolsQuestion />);
    await act(async () => fireEvent.click(await first.findByTestId("team-tools-not-now")));
    await waitFor(() => expect(first.queryByTestId("team-tools")).toBeNull());
    expect((await first.findByTestId("team-tools-line")).textContent).toBe("Off: they can't message each other yet.");
    first.unmount();
    const again = render(<TeamToolsQuestion />);
    expect(await again.findByTestId("team-tools-quiet")).toBeTruthy();
    expect(again.queryByTestId("team-tools")).toBeNull();
    await act(async () => fireEvent.click(again.getByTestId("team-tools-on")));
    await waitFor(() => expect(again.queryByTestId("team-tools-quiet")).toBeNull());
    expect(calls()).toEqual(["answer:false", "answer:true"]);
  });

  it("says nothing once on", async () => {
    seams().__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
    const { queryByTestId } = render(<TeamToolsQuestion />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(queryByTestId("team-tools")).toBeNull();
    expect(queryByTestId("team-tools-quiet")).toBeNull();
  });
});

describe("Settings › Agent access", () => {
  it("shows the question until it's answered", async () => {
    const { getByTestId } = render(<AgentToolsSettings />);
    await waitFor(() => expect(getByTestId("settings-agent-tools").textContent).toContain(TEAM_TOOLS.question));
  });

  it("switches off for every session, or a project, read when the fold opens", async () => {
    seams().__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
    const { findByTestId, queryByText, findByLabelText, getByTestId } = render(<AgentToolsSettings />);
    const all = (await findByTestId("pref-agent-tools")).querySelector("input")!;
    expect(all.checked).toBe(true);
    const fold = getByTestId("agent-tools-where") as HTMLDetailsElement;
    expect(fold.textContent).toContain("None");
    const project = MOCK_PROJECTS[0]!;
    expect(queryByText(project.name), "nothing is read before the fold opens").toBeNull();
    await act(async () => {
      fold.open = true;
      fireEvent(fold, new Event("toggle"));
    });
    const row = (await findByLabelText(project.name)) as HTMLInputElement;
    expect(row.checked).toBe(true);
    await act(async () => fireEvent.click(row));
    await waitFor(() => expect(calls()).toContain(`set:project:${project.id}:false`));
    await waitFor(() => expect(getByTestId("agent-tools-where").textContent).toContain("1 off"));
    await act(async () => fireEvent.click(all));
    await waitFor(() => expect(calls()).toContain("answer:false"));
  });

  // The rc19 walk: an unchecked project still read "Its sessions get the
  // agent tools."
  it("a project's line follows its switch", async () => {
    seams().__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
    const { findByTestId, findByLabelText, getByTestId } = render(<AgentToolsSettings />);
    await findByTestId("pref-agent-tools");
    const fold = getByTestId("agent-tools-where");
    await act(async () => {
      fold.setAttribute("open", "");
      fireEvent(fold, new Event("toggle"));
    });
    const project = MOCK_PROJECTS[0]!;
    const row = await findByLabelText(project.name);
    const line = () => getByTestId(`pref-agent-tools-project-${project.id}`).textContent ?? "";
    expect(line()).toContain("Its sessions get the agent tools.");
    await act(async () => fireEvent.click(row));
    await waitFor(() => expect(line()).toContain("Its sessions don't get the agent tools."));
    expect(line()).not.toContain("Its sessions get the agent tools.");
    await act(async () => fireEvent.click(row));
    await waitFor(() => expect(line()).toContain("Its sessions get the agent tools."));
  });
});

// AJ-E3013: run from a folder others can write, the bridge is never run;
// the Dev window said "Not in this build". Each place says which it is,
// and none offers Yes or Turn on for what can't run.
describe("when the team tools can't run", () => {
  const UNSAFE = "AI Juggler's team tools can't run from a folder others can write (~/Downloads/AI Juggler.app). Move AI Juggler to Applications, then open it again.";
  const unsafe = { available: false, unavailable: "unsafe_location", unsafe_folder: "~/Downloads/AI Juggler.app" };

  it.each([
    ["unanswered", { asked: false, on: false }],
    ["answered Not now", { asked: true, on: false }],
  ])("from a folder others can write (%s): the card's quiet line says why and what to do, with no Yes or Turn on", async (_, answer) => {
    seams().__AIJ_AGENT_BRIDGE__ = { ...answer, ...unsafe };
    const { findByTestId, queryByTestId } = render(<TeamToolsQuestion />);
    expect((await findByTestId("team-tools-line")).textContent).toBe(UNSAFE);
    expect(teamToolsLine(teamTools({ ...answer, ...unsafe, off_projects: [], off_profiles: [] }, [], []))).toBe(UNSAFE);
    for (const button of ["team-tools", "team-tools-yes", "team-tools-not-now", "team-tools-on"]) expect(queryByTestId(button)).toBeNull();
  });

  it("from a folder others can write: Settings says the same, its switch can't turn them on", async () => {
    seams().__AIJ_AGENT_BRIDGE__ = { asked: false, on: false, ...unsafe };
    const { findByTestId, queryByTestId } = render(<AgentToolsSettings />);
    const row = await findByTestId("pref-agent-tools");
    expect(row.textContent).toContain(UNSAFE);
    expect(row.querySelector("input")!.disabled).toBe(true);
    expect(queryByTestId("settings-team-tools-yes")).toBeNull();
  });

  it("not in this build: the line says so; no card, no Yes, no Turn on, and Settings can't turn them on", async () => {
    const missing = { asked: true, on: false, available: false, unavailable: "missing" };
    expect(teamToolsLine(teamTools({ ...missing, off_projects: [], off_profiles: [] }, [], []))).toBe("Not in this build of AI Juggler.");
    seams().__AIJ_AGENT_BRIDGE__ = missing;
    const card = render(<TeamToolsQuestion />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    for (const button of ["team-tools", "team-tools-quiet", "team-tools-yes", "team-tools-on"]) expect(card.queryByTestId(button)).toBeNull();
    card.unmount();
    seams().__AIJ_AGENT_BRIDGE__ = { ...missing, asked: false };
    resetPreviewAgentBridge();
    const { findByTestId, queryByTestId } = render(<AgentToolsSettings />);
    const row = await findByTestId("pref-agent-tools");
    expect(row.textContent).toContain("Not in this build of AI Juggler.");
    expect(row.querySelector("input")!.disabled).toBe(true);
    expect(queryByTestId("settings-team-tools")).toBeNull();
  });
});
