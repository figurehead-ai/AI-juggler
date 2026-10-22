import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector, type InspectorSelection } from "./RightInspector";
import type {
  AccountInfo,
  InstanceInfo,
  LaunchRecipeInfo,
  ProjectInfo,
} from "../lib/bridge";

/**
 * AJ-E406, then AJ-E1533 (one page model): every type's page — its
 * identity header, its own tabs with its body first and none empty,
 * per-type tab memory, keyboard switching, source footers, and the
 * never-unmounting terminal.
 */

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => (
    <div data-terminal-id={id} data-testid="instance-terminal" />
  ),
}));

afterEach(cleanup);

const NOW = 1_728_800_000_000;

const instance: InstanceInfo = {
  id: "sess-1",
  launch_recipe_id: "p-1",
  adopted: false,
  provider: "claude-code",
  account_id: "claude-work",
  account_label: "Work Max",
  model: "opus",
  launch_model: "opus",
  running_model: null,
  model_source: "opus" === null ? "unknown" : "launch",
  status: "running",
  quota: { used_percentage: 40, resets_at_ms: NOW + 3_600_000, confidence: "computed" },
  tokens: null,
  cpu_pct: 12,
  rss_bytes: 1_000_000_000,
  child_count: null,
  subtask_count: null,
  git_branch: "main",
  git_dirty_count: null,
  needs_attention: null,
  attention: null,
  started_at_ms: NOW - 60_000,
  last_activity_ms: NOW - 5_000,
  machine_id: "this-mac",
  cwd: "/Users/dev/ai-juggler",
  kind: "terminal",
  isolation_label: null,
  project_id: "proj-1",
  conversation_id: null,
};

const project: ProjectInfo = {
  id: "proj-1",
  root: "/w/alpha",
  name: "alpha",
  last_activity_ms: null,
  active_instance: null,
  accounts_used: [],
  instance_count: 1,
  history_sessions: 0, history_last_used_ms: null, repo: null,
  missing: false,
};

const account: AccountInfo = {
  id: "claude-work",
  provider: "claude-code",
  label: "Work Max",
  plan: "max-20x",
  login_state: "logged_in", cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false, in_place: false,
};

const recipe: LaunchRecipeInfo = {
  id: "p-2",
  name: "Quick fixes",
  account_id: "claude-personal",
  account_label: "Personal",
  provider: "claude-code",
  model: null,
  default_cwd: null,
};

const props = {
  sessions: [instance],
  projects: [project],
  accounts: [account],
  profiles: [recipe],
  terminalTitle: "Work Max · instance",
  freshness: { "sess-1": { resources: NOW - 1_000, quota: NOW - 1_000 } },
  now: NOW,
  onOpenProject: () => {},
  onOpenSession: () => {},
  onLaunch: () => {},
  onStopped: () => {},
  onHandover: () => {},
  onNew: () => {},
};

function shell(selection: InspectorSelection, data = {}) {
  return render(
    <RightInspector
      selection={selection}
      data={{
        instance: selection.kind === "instance" ? instance : null,
        project: selection.kind === "project" ? project : null,
        account: selection.kind === "account" ? account : null,
        provider: selection.kind === "provider" ? { id: "claude-code", name: "claude-code" } : null,
        machineId: "this-mac",
        ...data,
      }}
      {...props}
    />,
  );
}

beforeEach(() => {
  localStorage.clear();
});

describe("AJ-E1533 one page model: each type's header and its own tabs", () => {
  const kinds: { kind: InspectorSelection; name: string; type: string; tabs: string[] }[] = [
    // D-25: a session is titled by its label, else its place (branch, else folder).
    { kind: { kind: "instance", id: "sess-1" }, name: "main", type: "Session", tabs: ["Terminal", "Usage", "Processes"] },
    // A tool's page is its body alone: no tab strip.
    { kind: { kind: "provider", id: "claude-code" }, name: "claude-code", type: "Tool", tabs: [] },
    { kind: { kind: "account", id: "claude-work" }, name: "Work Max", type: "Account", tabs: ["Overview", "Sessions"] },
    // Nothing measured yet: no Usage tab (no empty tabs).
    { kind: { kind: "project", id: "proj-1" }, name: "alpha", type: "Project", tabs: ["Sessions", "Overview"] },
  ];

  for (const entry of kinds) {
    it(`a ${entry.type}'s page: its type, its name, and only the tabs with something to show`, () => {
      const { container, getByTestId } = shell(entry.kind);
      expect(getByTestId("page-type").textContent).toBe(entry.type);
      expect(getByTestId("page-name").textContent).toBe(entry.name);
      expect([...container.querySelectorAll("[role='tab']")].map((tab) => tab.textContent)).toEqual(entry.tabs);
    });
  }

  it("the session header carries the I-04 state chip, and its project as the crumb", () => {
    const openProject = vi.fn();
    const { container, getByTestId } = render(
      <RightInspector selection={{ kind: "instance", id: "sess-1" }} data={{ instance, machineId: "this-mac" }} {...props} onOpenProject={openProject} />,
    );
    expect(container.textContent).toContain("Running");
    fireEvent.click(getByTestId("crumb-project"));
    expect(openProject).toHaveBeenCalledWith("proj-1");
  });

  it("a project's Usage tab appears once something is measured, by account", () => {
    const measured = { ...instance, tokens: { input: 1200, output: 300, cache_read: 5000, cache_write: 0, reasoning: 0 } };
    const { container, getByTestId } = render(
      <RightInspector selection={{ kind: "project", id: "proj-1" }} data={{ project, machineId: "this-mac" }} {...props} sessions={[measured]} />,
    );
    fireEvent.click(container.querySelector("#page-tab-usage")!);
    expect(getByTestId("project-usage-total").textContent).toBe("2k");
  });

  it("a project's header reads its root's git state: branch, ahead/behind, not committed", async () => {
    (window as unknown as { __AIJ_PROJECT_GIT__?: unknown }).__AIJ_PROJECT_GIT__ = {
      "proj-1": { branch: "trunk", changed: 2, untracked: 1, ahead: 2, behind: 0, worktree: false, created_by_juggler: false, path: "/w/alpha" },
    };
    const { findByTestId, getByTestId, queryByTestId } = shell({ kind: "project", id: "proj-1" });
    expect((await findByTestId("project-uncommitted")).textContent).toBe("· 3 files not committed");
    expect(getByTestId("project-branch").textContent).toBe("· trunk");
    expect(getByTestId("project-upstream").textContent).toBe("· 2 ahead");
    delete (window as unknown as { __AIJ_PROJECT_GIT__?: unknown }).__AIJ_PROJECT_GIT__;
    cleanup();
    // Not a git checkout (or not answered yet): its sessions' branches, nothing more.
    const plain = shell({ kind: "project", id: "proj-1" });
    expect(plain.getByTestId("project-branch").textContent).toBe("· main");
    expect(queryByTestId("project-uncommitted")).toBeNull();
  });

  it("a project's body lists its sessions, live first", () => {
    const { getAllByTestId, getByTestId } = shell({ kind: "project", id: "proj-1" });
    expect(getAllByTestId("page-session-row")).toHaveLength(1);
    expect(getByTestId("page-sessions-count").textContent).toBe("1 session · 1 running");
  });
});

describe("AJ-E1533 tab memory per type (AJ-E1534)", () => {
  it("restores a type's last tab, never carrying it to another type", () => {
    const first = shell({ kind: "instance", id: "sess-1" });
    fireEvent.click(first.container.querySelector("#page-tab-usage")!);
    expect(localStorage.getItem("aij.page.tab.session")).toBe("usage");
    cleanup();

    const other = shell({ kind: "project", id: "proj-1" });
    expect(other.container.querySelector("#page-tab-sessions")!.getAttribute("aria-selected")).toBe("true");
    cleanup();

    const back = shell({ kind: "instance", id: "sess-1" });
    expect(back.container.querySelector("#page-tab-usage")!.getAttribute("aria-selected")).toBe("true");
  });
});

describe("AJ-E1533 keyboard, footers, the terminal", () => {
  // AJ-E1504: ⌘1–4 open the main screens, so the tabs are ⌥⌘1…n
  // (⌥ changes what `key` reads; the digit comes from `code`).
  it("Option+Cmd/Ctrl+digit picks a tab; Cmd+digit alone doesn't", () => {
    const { container } = shell({ kind: "instance", id: "sess-1" });
    fireEvent.keyDown(window, { key: "3", code: "Digit3", metaKey: true });
    expect(container.querySelector("#page-tab-processes")!.getAttribute("aria-selected")).toBe("false");
    fireEvent.keyDown(window, { key: "£", code: "Digit3", metaKey: true, altKey: true });
    expect(container.querySelector("#page-tab-processes")!.getAttribute("aria-selected")).toBe("true");
    expect(document.activeElement?.id).toBe("page-tab-processes");
  });

  it("every tab's footer names where its numbers come from", () => {
    const { container } = shell({ kind: "instance", id: "sess-1" });
    expect(container.querySelector("section:not([hidden]) footer")?.textContent).toContain("from this Mac's process monitor");
    fireEvent.click(container.querySelector("#page-tab-usage")!);
    expect(container.querySelector("section:not([hidden]) footer")?.textContent).toContain("from usage tracking");
  });

  it("switching tabs NEVER unmounts the embedded terminal (gate)", () => {
    const { container } = shell({ kind: "instance", id: "sess-1" });
    expect(container.querySelector('[data-testid="instance-terminal"]')).not.toBeNull();
    fireEvent.click(container.querySelector("#page-tab-processes")!);
    expect(container.querySelector('[data-testid="instance-terminal"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="instance-terminal"]')!.closest("section")!.hasAttribute("hidden")).toBe(true);
  });

  it("the session header says on which account and since when, and offers Switch account…", () => {
    const { getByTestId } = shell({ kind: "instance", id: "sess-1" });
    expect(getByTestId("session-subline").textContent).toContain("Work Max");
    expect(getByTestId("session-subline").textContent).toMatch(/started .+ ago/);
    expect(getByTestId("header-switch").textContent).toBe("Switch account…");
  });

  it("the switch list is one step away: closed, opened by Switch account…; open when the limit hit", async () => {
    const { getByTestId } = shell({ kind: "instance", id: "sess-1" });
    const section = getByTestId("handover-section") as HTMLDetailsElement;
    expect(section.open).toBe(false);
    await act(async () => {
      fireEvent.click(getByTestId("header-switch"));
    });
    expect((getByTestId("handover-section") as HTMLDetailsElement).open).toBe(true);
    cleanup();
    const blocked = shell({ kind: "instance", id: "sess-1" }, { instance: { ...instance, status: "quota_blocked" } });
    expect((blocked.getByTestId("handover-section") as HTMLDetailsElement).open).toBe(true);
  });

  it("empty selection renders nothing here — the work area is Home (D-25)", () => {
    const { container } = shell({ kind: "none" });
    expect(container.querySelector('[data-testid="right-inspector"]')).toBeNull();
    expect(container.textContent).toBe("");
  });

  it("remote-machine selections show the machine chip (Req.7)", () => {
    const { container } = shell({ kind: "instance", id: "sess-1" }, { machineId: "studio-desk" });
    expect(container.textContent).toContain("studio-desk");
  });

  it("the tab list is labelled, and each panel by its tab (WCAG 2.4.6)", () => {
    const { container } = shell({ kind: "project", id: "proj-1" });
    expect(container.querySelector("[role='tablist']")?.getAttribute("aria-label")).toBe("Project tabs");
    for (const id of ["sessions", "overview"]) {
      expect(container.querySelector(`#page-panel-${id}`)?.getAttribute("aria-labelledby")).toBe(`page-tab-${id}`);
    }
  });
});
