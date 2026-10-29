/**
 * AJ-E1704 vitest — the agent tools in sessions (D-30).
 *
 * - The one-time question shows until it's answered, never again after;
 *   "Yes, let them" turns the tools on, "Not now" leaves them off.
 * - Settings › Agent access: the question until answered, then one
 *   switch for every session and, behind a fold read only when it opens,
 *   a switch per project and per profile.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MOCK_PROJECTS } from "../lib/bridge";
import { resetPreviewAgentBridge } from "../lib/ipc/agent-bridge";
import { AgentToolsQuestion, AgentToolsSettings, QUESTION } from "./AgentTools";

type Seams = { __AIJ_AGENT_BRIDGE__?: unknown; __AIJ_AGENT_BRIDGE_CALLS__?: string[] };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_AGENT_BRIDGE_CALLS__ ?? [];

beforeEach(() => {
  delete seams().__AIJ_AGENT_BRIDGE__;
  delete seams().__AIJ_AGENT_BRIDGE_CALLS__;
  resetPreviewAgentBridge();
});
afterEach(() => cleanup());

describe("the one-time question", () => {
  it("asks once, and yes turns the tools on", async () => {
    const { findByTestId, queryByTestId, getByText } = render(<AgentToolsQuestion />);
    await findByTestId("agent-tools-question");
    expect(getByText(QUESTION)).toBeTruthy();
    expect(QUESTION).toBe("Let agents in your projects see each other and work together?");
    await act(async () => fireEvent.click(await findByTestId("agent-tools-yes")));
    await waitFor(() => expect(queryByTestId("agent-tools-question")).toBeNull());
    expect(calls()).toEqual(["answer:true"]);
  });

  it("not now leaves them off and doesn't ask again", async () => {
    const first = render(<AgentToolsQuestion />);
    await act(async () => fireEvent.click(await first.findByTestId("agent-tools-no")));
    await waitFor(() => expect(first.queryByTestId("agent-tools-question")).toBeNull());
    first.unmount();
    const again = render(<AgentToolsQuestion />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(again.queryByTestId("agent-tools-question")).toBeNull();
    expect(calls()).toEqual(["answer:false"]);
  });

  it("says nothing once answered", async () => {
    seams().__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
    const { queryByTestId } = render(<AgentToolsQuestion />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(queryByTestId("agent-tools-question")).toBeNull();
  });
});

describe("Settings › Agent access", () => {
  it("shows the question until it's answered", async () => {
    const { getByTestId } = render(<AgentToolsSettings />);
    await waitFor(() => expect(getByTestId("settings-agent-tools").textContent).toContain(QUESTION));
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
});
