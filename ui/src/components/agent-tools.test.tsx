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
import { TEAM_TOOLS } from "../lib/team-tools";
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
    expect(card.textContent).toContain("You can switch it off per project or app in Settings.");
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
