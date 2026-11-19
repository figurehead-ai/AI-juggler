/**
 * The 0.2 walk (F10): a project's Tasks and Messages are one click from
 * its page: its own pool's tabs (D-33), not only from a Needs you card or
 * a pool you made.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import { refreshPools, resetPools } from "../../lib/pools";
import { poolsList, resetPreviewPools } from "../../lib/ipc/pools";
import { MOCK_PROJECTS } from "../../preview/fixtures";
import { clearSeams, setSeam } from "../../test/fixtures";
import { ProjectAgentsCard } from "./AgentsCard";

afterEach(() => {
  cleanup();
  clearSeams("__AIJ_POOLS__");
  resetPools();
  resetPreviewPools();
  localStorage.clear();
});

describe("a project's agents", () => {
  it("opens its own pool's Tasks, Messages and Agent access", async () => {
    await act(async () => {
      await refreshPools();
    });
    const project = MOCK_PROJECTS[0]!;
    const pool = (await poolsList()).pools.find((entry) => entry.kind === "default" && entry.project_id === project.id)!;
    expect(pool).toBeTruthy();
    const opened: string[] = [];
    const view = render(
      <ActionHostProvider host={{ openPool: (id) => opened.push(id) }}>
        <ProjectAgentsCard project={project} />
      </ActionHostProvider>,
    );
    fireEvent.click(await view.findByTestId("project-agents-tasks"));
    expect(opened).toEqual([pool.id]);
    expect(localStorage.getItem("aij.page.tab.pool")).toBe("tasks");
    fireEvent.click(view.getByTestId("project-agents-messages"));
    expect(localStorage.getItem("aij.page.tab.pool")).toBe("messages");
    expect(view.getByTestId("project-agents-access")).toBeTruthy();
  });

  it("shows nothing where no pool page can open", async () => {
    await act(async () => {
      await refreshPools();
    });
    const view = render(<ProjectAgentsCard project={MOCK_PROJECTS[0]!} />);
    expect(view.queryByTestId("project-agents")).toBeNull();
  });

  // R4 (Priya, keeping Client A apart; main's rulings): the card says
  // whether the project is kept apart, with the way to do it, and what's
  // true of its agents, the level's name second.
  it("says it isn't kept apart, and offers Keep it apart… where she looks", async () => {
    await act(async () => {
      await refreshPools();
    });
    const project = MOCK_PROJECTS[0]!;
    const kept: string[] = [];
    const view = render(
      <ActionHostProvider host={{ openPool: () => undefined, keepProjectApart: (p) => kept.push(p.id) }}>
        <ProjectAgentsCard project={project} />
      </ActionHostProvider>,
    );
    const apart = await view.findByTestId("project-agents-apart");
    expect(apart.textContent).toContain("Kept apart: no");
    expect(apart.textContent).toContain("its sessions can be in your other pools too");
    expect(view.getByTestId("project-agents").textContent).toContain("Agents here may message each other (Work together)");
    expect(view.getByTestId("project-agents").textContent).not.toContain("on their own:");
    fireEvent.click(view.getByTestId("project-agents-keep-apart"));
    expect(kept).toEqual([project.id]);
  });

  it("says it's kept apart, in which pool, and opens that pool", async () => {
    const project = MOCK_PROJECTS[0]!;
    setSeam("__AIJ_POOLS__", { pools: [{ id: "pool-client-a", name: "Client A only", project_id: project.id, sealed: true }] });
    await act(async () => {
      await refreshPools();
    });
    const opened: string[] = [];
    const view = render(
      <ActionHostProvider host={{ openPool: (id) => opened.push(id), keepProjectApart: () => undefined }}>
        <ProjectAgentsCard project={project} />
      </ActionHostProvider>,
    );
    const apart = await view.findByTestId("project-agents-apart");
    expect(apart.textContent).toContain("Kept apart: yes");
    expect(apart.textContent).toContain("in Client A only, its sessions are in no other pool");
    expect(view.queryByTestId("project-agents-keep-apart")).toBeNull();
    fireEvent.click(view.getByTestId("project-agents-apart-open"));
    expect(opened).toEqual(["pool-client-a"]);
  });
});
