/**
 * AJ-E1502 vitest — sessions and projects named, pinned, opened and
 * tidied from the rail's row menus: pinned rows lead (sessions in their
 * group, a project's group among groups), a label shows over the
 * session's own title and is searchable, Rename (blank goes back to the
 * title), Pin, Open in the chosen editor else Finder, Relocate, Forget
 * (a refusal in words).
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { repoGroups } from "./rail-groups";
import * as manage from "../lib/ipc/manage";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import { filterSessions, groupSessions, sortSessions, NO_CHIP_FILTERS } from "../lib/railview";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const NOW = 1_728_800_000_000;

function mk(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "finished",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: `branch-${id}`,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000,
    machine_id: "this-mac",
    cwd: `/w/${id}`,
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-1",
    conversation_id: null,
    ...overrides,
  };
}

const project = (id: string, name: string, extra: Partial<ProjectInfo> = {}): ProjectInfo => ({
  id,
  root: `/w/${name}`,
  name,
  last_activity_ms: null,
  active_instance: null,
  accounts_used: [],
  instance_count: 0,
  history_sessions: 0,
  history_last_used_ms: null,
  repo: null,
  missing: false,
  ...extra,
});

const board = (rows: InstanceInfo[], projects: ProjectInfo[], onChanged = vi.fn()) => {
  const view = render(
    <InstanceBoard
      sessions={rows}
      loginTerminals={[]}
      selectedId={null}
      onSelect={() => {}}
      density="comfortable"
      freshness={{}}
      now={NOW}
      projects={projects}
      snooze={{}}
      onSnooze={() => {}}
      onSelectProject={() => {}}
      onChanged={onChanged}
    />,
  );
  return { ...view, onChanged };
};

beforeEach(() => {
  vi.spyOn(manage, "editorChoices").mockResolvedValue({ installed: ["Zed"], chosen: null });
});

describe("AJ-E1502 pins lead and labels show", () => {
  it("pinned sessions lead in every sort, in the sort's own order", () => {
    const rows = [mk("a"), mk("b", { pinned: true }), mk("c"), mk("d", { pinned: true })];
    for (const mode of ["needs", "age", "name", "account"] as const) {
      const ids = sortSessions(rows, mode, {}, NOW).map((row) => row.id);
      expect(ids.slice(0, 2).sort()).toEqual(["b", "d"]);
    }
  });

  it("a pinned project's group leads the rail, and its row leads the history list", () => {
    const projects = [project("p1", "alpha"), project("p2", "beta", { pinned: true })];
    const rows = [mk("a", { project_id: "p1" }), mk("b", { project_id: "p2" })];
    expect(groupSessions(rows, "project", projects).map((group) => group.label)).toEqual(["beta", "alpha"]);
    const history = [project("p1", "alpha", { history_last_used_ms: NOW }), project("p2", "beta", { pinned: true, history_last_used_ms: NOW - 86_400_000 })];
    expect(repoGroups(history).map((group) => group.name)).toEqual(["beta", "alpha"]);
  });

  it("a label shows over the session's own title, with a pin mark, and search finds it", () => {
    const rows = [mk("a", { label: "fix the login bug", pinned: true }), mk("b")];
    const { container } = board(rows, [project("proj-1", "alpha")]);
    const first = container.querySelector('[data-instance-id="a"]')!;
    expect(first.textContent).toContain("fix the login bug");
    expect(first.textContent).not.toContain("branch-a");
    expect(first.querySelector('[data-testid="pin-mark"]')).not.toBeNull();
    expect(first.textContent).toContain("pinned");
    expect(filterSessions(rows, NO_CHIP_FILTERS, "login bug").map((row) => row.id)).toEqual(["a"]);
  });
});

describe("AJ-E1502 the session row's menu", () => {
  it("renames (blank goes back to the title) and pins", async () => {
    const rename = vi.spyOn(manage, "sessionRename").mockResolvedValue();
    const pin = vi.spyOn(manage, "sessionSetPinned").mockResolvedValue();
    const { getByRole, getByTestId, onChanged } = board([mk("a")], [project("proj-1", "alpha")]);

    fireEvent.click(getByRole("button", { name: "Session actions for branch-a" }));
    const menu = getByRole("menu", { name: "Actions for branch-a" });
    expect([...menu.querySelectorAll("[role='menuitem']")].map((item) => item.textContent).slice(0, 3)).toEqual(["Open", "Rename…", "Pin to top"]);
    fireEvent.click(within(menu).getByText("Rename…"));
    const name = getByTestId("rename-session-name") as HTMLInputElement;
    expect(name.placeholder).toBe("branch-a");
    fireEvent.change(name, { target: { value: "the login fix" } });
    fireEvent.click(getByTestId("rename-session-save"));
    await waitFor(() => expect(rename).toHaveBeenCalledWith("a", "the login fix"));
    await waitFor(() => expect(onChanged).toHaveBeenCalled());

    fireEvent.click(getByRole("button", { name: "Session actions for branch-a" }));
    fireEvent.click(within(getByRole("menu")).getByText("Rename…"));
    fireEvent.change(getByTestId("rename-session-name"), { target: { value: "  " } });
    fireEvent.click(getByTestId("rename-session-save"));
    await waitFor(() => expect(rename).toHaveBeenLastCalledWith("a", null));

    fireEvent.click(getByRole("button", { name: "Session actions for branch-a" }));
    fireEvent.click(within(getByRole("menu")).getByText("Pin to top"));
    await waitFor(() => expect(pin).toHaveBeenCalledWith("a", true));
  });
});

describe("AJ-E1502 the project row's menu", () => {
  const projects = [project("p1", "alpha"), project("p2", "beta", { missing: true })];

  it("offers rename, pin, open, relocate and forget; open goes to the chosen editor, else Finder", async () => {
    const open = vi.spyOn(manage, "projectOpen").mockResolvedValue();
    const pin = vi.spyOn(manage, "projectSetPinned").mockResolvedValue();
    const { getByRole, findByRole, onChanged } = board([], projects);
    fireEvent.click(getByRole("button", { name: "Project actions for alpha" }));
    const menu = await findByRole("menu", { name: "Actions for alpha" });
    await waitFor(() =>
      expect([...menu.querySelectorAll("[role='menuitem']")].map((item) => item.textContent)).toEqual([
        "Open",
        "Open in Finder",
        "Copy path",
        "Rename…",
        "Change its mark…",
        "Pin to top",
        "Compare rules and servers…",
        "Forget…",
      ]),
    );
    fireEvent.click(within(menu).getByText("Open in Finder"));
    await waitFor(() => expect(open).toHaveBeenCalledWith("p1", "editor"));

    fireEvent.click(getByRole("button", { name: "Project actions for alpha" }));
    fireEvent.click(within(getByRole("menu")).getByText("Pin to top"));
    await waitFor(() => expect(pin).toHaveBeenCalledWith("p1", true));
    expect(onChanged).toHaveBeenCalled();
  });

  it("with an editor chosen: Open in it, and Show in Finder too", async () => {
    vi.spyOn(manage, "editorChoices").mockResolvedValue({ installed: ["Zed"], chosen: "Zed" });
    const open = vi.spyOn(manage, "projectOpen").mockResolvedValue();
    const { getByRole, findByRole } = board([], projects);
    fireEvent.click(getByRole("button", { name: "Project actions for alpha" }));
    const menu = await findByRole("menu");
    await waitFor(() => expect(within(menu).getByText("Open in Zed")).toBeTruthy());
    fireEvent.click(within(menu).getByText("Show in Finder"));
    await waitFor(() => expect(open).toHaveBeenCalledWith("p1", "finder"));
  });

  it("a missing folder can't be opened, and Relocate asks the backend (which picks the folder itself)", async () => {
    const relocate = vi.spyOn(manage, "projectRelocate").mockResolvedValue(true);
    const { getByRole } = board([], projects);
    fireEvent.click(getByRole("button", { name: "Project actions for beta" }));
    const menu = getByRole("menu");
    expect((within(menu).getByText("Open in Finder").closest("button") as HTMLButtonElement).disabled).toBe(true);
    expect((within(menu).getByText("Compare rules and servers…").closest("button") as HTMLButtonElement).disabled).toBe(true);
    expect(within(menu).getAllByText("Its folder isn't there any more").length).toBe(2);
    fireEvent.click(within(menu).getByText("Relocate…"));
    await waitFor(() => expect(relocate).toHaveBeenCalledWith("p2"));
  });

  it("rename keeps the folder's name as the fallback; forget asks first and shows a refusal in words", async () => {
    const rename = vi.spyOn(manage, "projectRename").mockResolvedValue();
    const forget = vi
      .spyOn(manage, "projectForget")
      .mockRejectedValueOnce({ code: "refused", retryable: false, user_message: "alpha has a session open: main. Stop it first, then forget the project" })
      .mockResolvedValue();
    const { getByRole, getByTestId, queryByTestId } = board([], projects);
    fireEvent.click(getByRole("button", { name: "Project actions for alpha" }));
    fireEvent.click(within(getByRole("menu")).getByText("Rename…"));
    expect((getByTestId("rename-project-name") as HTMLInputElement).placeholder).toBe("alpha");
    fireEvent.change(getByTestId("rename-project-name"), { target: { value: "Billing" } });
    fireEvent.click(getByTestId("rename-project-save"));
    await waitFor(() => expect(rename).toHaveBeenCalledWith("p1", "Billing"));

    fireEvent.click(getByRole("button", { name: "Project actions for alpha" }));
    fireEvent.click(within(getByRole("menu")).getByText("Forget…"));
    expect(getByTestId("forget-project").textContent).toContain("until you start a session there again");
    fireEvent.click(getByTestId("forget-project-confirm"));
    await waitFor(() => expect(getByRole("alert").textContent).toContain("Stop it first"));
    fireEvent.click(getByTestId("forget-project-confirm"));
    await waitFor(() => expect(queryByTestId("forget-project")).toBeNull());
    expect(forget).toHaveBeenLastCalledWith("p1", true);
  });
});
