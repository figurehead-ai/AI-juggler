/**
 * AJ-E1532 vitest — telling projects and sessions apart: a project's mark
 * (its icon, else its letters or emoji on its colour; clones numbered),
 * account avatars and their stacks (live ones ringed, then "+N"), the
 * accounts a project's row stacks, the rows' names in words ("on Work
 * Max"), and the mark sheet from the project's menu.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MarkTile, ProjectMark } from "./ProjectMark";
import { AccountAvatar, AccountStack } from "./Avatar";
import { MarkSheet } from "./MarkSheet";
import { InstanceBoard, RailRow } from "./WorkflowRail";
import { projectAccounts, accountsWords } from "../lib/project-accounts";
import { assignPalette, swatchColor, withMarks } from "../lib/palette";
import * as marks from "../lib/ipc/marks";
import * as manage from "../lib/ipc/manage";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import type { ConversationInfo } from "../types/generated/ConversationInfo";
import type { ProjectMarkInfo } from "../types/generated/ProjectMarkInfo";
import { useProjectLens } from "./project-lens.test-kit";

// AJ-E2104: these read the project board, the rail's Project lens.
useProjectLens();

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete (window as unknown as { __AIJ_PROJECT_ICONS__?: unknown }).__AIJ_PROJECT_ICONS__;
});

beforeEach(() => {
  vi.spyOn(manage, "editorChoices").mockResolvedValue({ installed: [], chosen: null });
  assignPalette([]);
});

const NOW = 1_728_800_000_000;
const PNG = "data:image/png;base64,iVBORw0KGgo=";

const letters = (extra: Partial<ProjectMarkInfo> = {}): ProjectMarkInfo => ({ letters: "AW", swatch: 3, shows: "letters", ...extra });

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

function session(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "a-work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
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
    project_id: "p1",
    conversation_id: null,
    ...overrides,
  };
}

function conversation(id: string, overrides: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id,
    tool: "claude-code",
    account_id: "a-personal",
    account_label: "Personal",
    project_id: "p1",
    cwd: "/w/app",
    title: `Conversation ${id}`,
    git_branch: "main",
    started_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    tokens: null,
    live: false,
    session_id: null,
    places: [],
    pid: null,
    state: null,
    viewed_ms: null, probably: false,
    background: false,
    inactive: false,
    subtask_count: null,
    ...overrides,
  };
}

describe("a project's mark", () => {
  it("is its letters on its colour, a rounded square", () => {
    withMarks([project("p1", "acme-web", { mark: letters() })]);
    render(<ProjectMark projectId="p1" size={24} />);
    const mark = screen.getByTestId("mark-project");
    expect(mark.textContent).toBe("AW");
    expect(Number.parseFloat(mark.style.fontSize)).toBeGreaterThanOrEqual(12);
    expect(mark.getAttribute("data-kind")).toBe("letters");
    const probe = document.createElement("span");
    probe.style.background = swatchColor(3);
    expect(mark.style.background).toBe(probe.style.background);
    expect(mark.style.borderRadius).not.toBe("50%");
    expect(mark.getAttribute("aria-hidden")).toBe("true");
  });

  it("is its own icon when it shows one, ringed in its colour", async () => {
    (window as unknown as { __AIJ_PROJECT_ICONS__?: Record<string, string> }).__AIJ_PROJECT_ICONS__ = { p1: PNG };
    withMarks([project("p1", "acme-web", { mark: letters({ icon: "v1", shows: "icon" }) })]);
    render(<ProjectMark projectId="p1" size={20} />);
    await waitFor(() => expect(screen.getByTestId("mark-project").getAttribute("data-kind")).toBe("icon"));
    expect(screen.getByTestId("mark-project").querySelector("img")?.getAttribute("src")).toBe(PNG);
  });

  it("keeps its letters while its icon is set aside for them", () => {
    (window as unknown as { __AIJ_PROJECT_ICONS__?: Record<string, string> }).__AIJ_PROJECT_ICONS__ = { p1: PNG };
    withMarks([project("p1", "acme-web", { mark: letters({ icon: "v1", shows: "letters", choice: { letters: "AW" } }) })]);
    render(<ProjectMark projectId="p1" />);
    expect(screen.getByTestId("mark-project").getAttribute("data-kind")).toBe("letters");
  });

  it("an emoji shows instead of the letters; a clone carries its number", () => {
    render(<MarkTile mark={letters({ emoji: "🚀", shows: "emoji", clone: 2 })} icon={null} size={24} />);
    expect(screen.getByTestId("mark-project").textContent).toContain("🚀");
    expect(screen.getByTestId("mark-clone").textContent).toBe("2");
  });

  it("a project the list doesn't know yet gets a neutral folder, never a guessed colour", () => {
    withMarks([]);
    render(<ProjectMark projectId="nope" />);
    expect(screen.getByTestId("mark-project").getAttribute("data-kind")).toBeNull();
    expect(screen.getByTestId("mark-project").querySelector("svg")).toBeTruthy();
  });
});

describe("account avatars", () => {
  it("are round, in the account's colour, with its initials and its name as a tooltip", () => {
    assignPalette([
      { id: "a-work", label: "claude-work" },
      { id: "a-personal", label: "claude-personal" },
    ]);
    render(<AccountAvatar accountId="a-work" label="claude-work" size={24} />);
    const avatar = screen.getByTestId("account-avatar");
    expect(avatar.textContent).toBe("Wo");
    expect(avatar.title).toBe("claude-work");
    expect(avatar.className).toContain("rounded-full");
    expect(avatar.getAttribute("aria-hidden")).toBe("true");
  });

  it("under 24 px is its colour alone, never letters under 12 px", () => {
    render(<AccountAvatar accountId="a-work" label="claude-work" size={16} />);
    expect(screen.getByTestId("account-avatar").textContent).toBe("");
    expect(screen.getByTestId("account-avatar").title).toBe("claude-work");
  });

  it("stack up to three, the rest as +N, live ones ringed", () => {
    const accounts = ["A", "B", "C", "D", "E"].map((label, n) => ({ id: label, label, live: n === 0 }));
    render(<AccountStack accounts={accounts} />);
    const stack = screen.getByTestId("account-stack");
    expect(within(stack).getAllByTestId("account-avatar")).toHaveLength(3);
    expect(stack.textContent).toContain("+2");
    expect(within(stack).getAllByTestId("account-avatar")[0]!.getAttribute("data-live")).toBe("true");
    expect(within(stack).getAllByTestId("account-avatar")[1]!.getAttribute("data-live")).toBeNull();
  });
});

describe("the accounts that worked in a project", () => {
  it("come from its sessions, the conversations outside AI Juggler and its history; live first, then the newest", () => {
    const p1 = project("p1", "acme-web", { accounts_used: ["Old Pro", "Work Max"] });
    const accounts = projectAccounts(
      p1,
      [session("s1", { status: "finished", last_activity_ms: NOW - 10_000 }), session("s2", { project_id: "p2", account_id: "x", account_label: "Elsewhere" })],
      [conversation("c1", { live: true, last_activity_ms: NOW - 99_000 })],
      [{ id: "a-old", label: "Old Pro" }],
    );
    expect(accounts).toEqual([
      { id: "a-personal", label: "Personal", live: true },
      { id: "a-work", label: "Work Max", live: false },
      { id: "a-old", label: "Old Pro", live: false },
    ]);
    expect(accountsWords(accounts)).toBe("on Personal (running), Work Max and Old Pro");
  });
});

describe("rows say which account serves them, in words", () => {
  it("a session row: the tool on the account, and its avatar beside the tool's mark", () => {
    render(<RailRow session={session("s1")} selected={false} onSelect={() => {}} density="comfortable" now={NOW} />);
    const row = screen.getByRole("button");
    expect(row.textContent).toContain("Claude Code on Work Max");
    expect(within(row).getByTestId("account-avatar").title).toBe("Work Max");
  });

  it("a session row shows its project's mark only where the list isn't grouped by project", () => {
    withMarks([project("p1", "acme-web", { mark: letters() })]);
    const { rerender } = render(<RailRow session={session("s1")} selected={false} onSelect={() => {}} density="comfortable" now={NOW} />);
    expect(screen.queryByTestId("mark-project")).toBeNull();
    rerender(<RailRow session={session("s1")} showProject selected={false} onSelect={() => {}} density="comfortable" now={NOW} />);
    expect(screen.getByTestId("mark-project").getAttribute("data-kind")).toBe("letters");
  });

  it("a project's group: its mark, the accounts that worked there (live ringed), and their names", () => {
    (window as unknown as { __AIJ_FIXTURE__?: unknown }).__AIJ_FIXTURE__ = { conversations: [] };
    const projects = withMarks([project("p1", "acme-web", { mark: letters() })]);
    render(
      <InstanceBoard
        sessions={[session("s1")]}
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
        onChanged={() => {}}
      />,
    );
    const heading = screen.getByTestId("rail-project");
    expect(within(heading).getByTestId("mark-project").getAttribute("data-kind")).toBe("letters");
    expect(within(heading).getByTestId("account-avatar").getAttribute("data-live")).toBe("true");
    // The accounts describe the heading; its name stays the project's.
    expect(heading.textContent).not.toContain("Work Max");
    expect(heading.getAttribute("aria-describedby")).toBeTruthy();
    expect(document.getElementById(heading.getAttribute("aria-describedby")!)!.textContent).toBe("on Work Max (running)");
  });
});

describe("the mark sheet", () => {
  it("keeps a colour and an emoji the user picks, and offers the project's icon back", async () => {
    const set = vi.spyOn(marks, "projectMarkSet").mockResolvedValue();
    const done = vi.fn();
    render(<MarkSheet project={project("p1", "acme-web", { mark: letters({ icon: "v1", shows: "icon" }) })} onClose={() => {}} onDone={done} />);
    expect(screen.getByTestId("mark-shows-icon")).toBeTruthy();
    fireEvent.click(screen.getByRole("radio", { name: "Pink" }));
    fireEvent.click(screen.getByTestId("mark-shows-emoji"));
    fireEvent.change(screen.getByTestId("mark-emoji"), { target: { value: "🚀" } });
    fireEvent.click(screen.getByTestId("mark-save"));
    await waitFor(() => expect(set).toHaveBeenCalledWith("p1", { swatch: 8, emoji: "🚀" }));
    expect(done).toHaveBeenCalled();
  });

  it("letters set aside the icon; an unchanged sheet keeps nothing", async () => {
    const set = vi.spyOn(marks, "projectMarkSet").mockResolvedValue();
    render(<MarkSheet project={project("p1", "acme-web", { mark: letters({ icon: "v1", shows: "icon" }) })} onClose={() => {}} onDone={() => {}} />);
    fireEvent.click(screen.getByTestId("mark-shows-letters"));
    fireEvent.click(screen.getByTestId("mark-save"));
    await waitFor(() => expect(set).toHaveBeenCalledWith("p1", { letters: "AW" }));
    cleanup();
    render(<MarkSheet project={project("p2", "infra", { mark: letters({ letters: "In" }) })} onClose={() => {}} onDone={() => {}} />);
    expect(screen.queryByTestId("mark-shows-icon")).toBeNull();
    fireEvent.click(screen.getByTestId("mark-save"));
    await waitFor(() => expect(set).toHaveBeenLastCalledWith("p2", {}));
  });

  it("'Use its own' forgets the choice; a refusal is said in the sheet", async () => {
    const set = vi.spyOn(marks, "projectMarkSet").mockRejectedValueOnce({ code: "refused", retryable: false, user_message: "Use one emoji" }).mockResolvedValue();
    render(<MarkSheet project={project("p1", "acme-web", { mark: letters({ choice: { swatch: 3 } }) })} onClose={() => {}} onDone={() => {}} />);
    fireEvent.click(screen.getByTestId("mark-save"));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Use one emoji");
    fireEvent.click(screen.getByTestId("mark-reset"));
    await waitFor(() => expect(set).toHaveBeenLastCalledWith("p1", {}));
  });
});
