/**
 * AJ-E2804: the Projects screen's row menu and its empty state.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AppActions } from "../../components/AppActions";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { ProjectsScreen } from "./ProjectsScreen";

const NOW = Date.now();

afterEach(cleanup);

describe("the Projects screen (AJ-E2804)", () => {
  it("gives each row the project's own menu: ⋯, right-click and ⇧F10 (AJ-E2804 R4)", () => {
    // The app's own host (not a hand-made one): the gap the E28 walk
    // found was a menu that only had what this screen was handed.
    const calls: string[] = [];
    const none = () => {};
    const view = render(
      <AppActions
        selectConversation={none}
        selectSession={none}
        selectProject={(id) => void calls.push(`open:${id}`)}
        selectAccount={none}
        selectTool={none}
        openToolSettings={none}
        selectPool={none}
        openAccounts={none}
        openHome={none}
        stopSession={none}
        newSessionIn={(p) => void calls.push(`new:${p.id}`)}
        refresh={none}
      >
        <ProjectsScreen projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} loading={false} now={NOW} onNewProject={() => undefined} />
      </AppActions>,
    );
    const row = view.getAllByTestId("project-row")[0]!;
    const id = row.dataset.projectId!;
    fireEvent.contextMenu(row, { clientX: 40, clientY: 40, button: 2 });
    const menu = view.getByRole("menu");
    const lines = within(menu).getAllByRole("menuitem").map((item) => item.textContent);
    for (const words of ["Open its page", "Open in Finder", "New session here", "Copy path", "Rename…", "Pin to top", "Forget…"]) expect(lines.some((line) => line?.includes(words)), words).toBe(true);
    // every line has its icon
    for (const item of within(menu).getAllByRole("menuitem")) expect(item.querySelector("svg"), item.textContent ?? "").toBeTruthy();
    fireEvent.click(within(menu).getByRole("menuitem", { name: /New session here/ }));
    expect(calls).toEqual([`new:${id}`]);
    fireEvent.contextMenu(row, { clientX: 40, clientY: 40, button: 2 });
    fireEvent.click(within(view.getByRole("menu")).getByRole("menuitem", { name: /Open its page/ }));
    expect(calls).toEqual([`new:${id}`, `open:${id}`]);
    fireEvent.contextMenu(row, { clientX: 40, clientY: 40, button: 2 });
    fireEvent.click(within(view.getByRole("menu")).getByRole("menuitem", { name: /Rename…/ }));
    expect(view.getByTestId("rename-project-name")).toBeTruthy();
    fireEvent.keyDown(document.activeElement!, { key: "Escape" });
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

describe("P6 (the R3 persona walk): each column says what it measures", () => {
  const mk = (id: string, status: string, projectId: string) => ({ ...MOCK_SESSIONS[0]!, id, status, project_id: projectId, needs_attention: null, attention: null });

  it("Sessions counts all of a project's sessions, as its tab does, then the open ones; the tokens column says tokens", () => {
    const [one, two] = MOCK_PROJECTS;
    const sessions = [mk("a", "finished", one!.id), mk("b", "stopped", one!.id), mk("c", "running", one!.id), mk("d", "finished", two!.id)];
    const view = render(<ProjectsScreen projects={[one!, two!]} sessions={sessions} loading={false} now={NOW} onNewProject={() => undefined} />);
    const cell = (id: string) => within(view.getAllByTestId("project-row").find((row) => row.dataset.projectId === id)!).getByTestId("project-row-open").textContent;
    // Three in all, one of them open (it read "1 open" beside a tab of 3).
    expect(cell(one!.id)).toMatch(/^3 · (1 working · )?1 open$/);
    // Ended ones only: their count, never "Nothing open" alone.
    expect(cell(two!.id)).toBe("1");
    // "This week" read as sessions; it is tokens.
    expect(view.container.textContent).toContain("Tokens this week");
    expect(view.container.textContent).not.toMatch(/(?<!Tokens )This week/);
  });
});
