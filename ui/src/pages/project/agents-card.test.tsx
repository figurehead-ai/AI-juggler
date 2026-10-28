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
import { MOCK_PROJECTS } from "../../lib/bridge";
import { ProjectAgentsCard } from "./AgentsCard";

afterEach(() => {
  cleanup();
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
        <ProjectAgentsCard projectId={project.id} />
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
    const view = render(<ProjectAgentsCard projectId={MOCK_PROJECTS[0]!.id} />);
    expect(view.queryByTestId("project-agents")).toBeNull();
  });
});
