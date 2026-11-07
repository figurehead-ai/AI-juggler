/**
 * AJ-E2804: the Projects screen's row menu and its empty state.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ActionHostProvider } from "../../components/ItemMenu";
import type { ActionHost } from "../../lib/action-host";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { ProjectsScreen } from "./ProjectsScreen";

const NOW = Date.now();

afterEach(cleanup);

describe("the Projects screen (AJ-E2804)", () => {
  it("gives each row the project's own menu: ⋯, right-click and ⇧F10 (AJ-E2804 R4)", () => {
    const calls: string[] = [];
    const actions: ActionHost = {
      openProject: (p) => void calls.push(`open:${p.id}`),
      openProjectIn: (p, where) => void calls.push(`${where}:${p.id}`),
      newSessionIn: (p) => void calls.push(`new:${p.id}`),
      renameProject: (p) => void calls.push(`rename:${p.id}`),
      forgetProject: (p) => void calls.push(`forget:${p.id}`),
    };
    const view = render(
      <ActionHostProvider host={actions}>
        <ProjectsScreen projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} loading={false} now={NOW} onNewProject={() => undefined} />
      </ActionHostProvider>,
    );
    const row = view.getAllByTestId("project-row")[0]!;
    const id = row.dataset.projectId!;
    fireEvent.contextMenu(row, { clientX: 40, clientY: 40, button: 2 });
    const menu = view.getByRole("menu");
    const lines = within(menu).getAllByRole("menuitem").map((item) => item.textContent);
    for (const words of ["Open its page", "Open in Finder", "New session here", "Rename…", "Forget…"]) expect(lines.some((line) => line?.includes(words)), words).toBe(true);
    // every line has its icon
    for (const item of within(menu).getAllByRole("menuitem")) expect(item.querySelector("svg"), item.textContent ?? "").toBeTruthy();
    fireEvent.click(within(menu).getByRole("menuitem", { name: /New session here/ }));
    expect(calls).toEqual([`new:${id}`]);
    // from the keyboard: ⇧F10 on the row
    fireEvent.keyDown(row, { key: "F10", shiftKey: true });
    expect(view.getByRole("menu")).toBeTruthy();
    fireEvent.keyDown(document.activeElement!, { key: "Escape" });
    // and its ⋯ button
    fireEvent.click(within(row.closest("li")!).getByTestId("project-row-menu"));
    expect(view.getByRole("menu")).toBeTruthy();
  });

  it("says how a project appears when there are none, and offers New project… (AJ-E2804 R5)", () => {
    const newProject = vi.fn();
    const view = render(<ProjectsScreen projects={[]} sessions={[]} loading={false} now={NOW} onNewProject={newProject} />);
    expect(view.getByTestId("projects-empty").textContent).toContain("No projects yet. A project appears when a session runs in its folder.");
    fireEvent.click(view.getByTestId("projects-empty-new"));
    expect(newProject).toHaveBeenCalledOnce();
    expect(view.getByTestId("projects-empty-new").getAttribute("title")).toContain("⌘⇧N");
  });

});
