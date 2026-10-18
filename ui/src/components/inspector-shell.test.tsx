import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { INSPECTOR_TABS, RightInspector, type InspectorSelection } from "./RightInspector";
import type {
  AccountInfo,
  InstanceInfo,
  LaunchRecipeInfo,
  ProjectInfo,
} from "../lib/bridge";

/**
 * AJ-E406 evidence: the RightInspector shell across all five selection
 * types — six tabs always present (D-06), header contract, per-type tab
 * persistence, keyboard switching, honest-empty tabs, source footers,
 * and the never-unmounting terminal.
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
  history_sessions: 0,
  missing: false,
};

const account: AccountInfo = {
  id: "claude-work",
  provider: "claude-code",
  label: "Work Max",
  plan: "max-20x",
  login_state: "logged_in", cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
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

describe("AJ-E406 the shell across the five selection types (D-06)", () => {
  const kinds: { kind: InspectorSelection; name: string; type: string }[] = [
    { kind: { kind: "instance", id: "sess-1" }, name: "Work Max", type: "Instance" },
    { kind: { kind: "session", id: "conv-1" }, name: "Conversation", type: "Session" },
    { kind: { kind: "provider", id: "claude-code" }, name: "claude-code", type: "Provider" },
    { kind: { kind: "account", id: "claude-work" }, name: "Work Max", type: "Account" },
    { kind: { kind: "project", id: "proj-1" }, name: "alpha", type: "Project" },
  ];

  for (const entry of kinds) {
    it(`renders all six tabs and the header for ${entry.type}`, () => {
      const { container } = shell(entry.kind);
      const tabs = [...container.querySelectorAll("[role='tab']")];
      expect(tabs.map((tab) => tab.textContent)).toEqual([
        "Overview",
        "Usage",
        "History",
        "Quality / Performance",
        "Processes",
        "Actions",
      ]);
      expect(container.querySelector(`[title='${entry.type}']`)).not.toBeNull();
      expect(container.querySelector("h2")?.textContent).toContain(entry.name);
      // D-06: every tab is present — none hidden or disabled.
      for (const tab of tabs) {
        expect((tab as HTMLButtonElement).disabled).toBe(false);
      }
    });
  }

  it("the Instance header carries the I-04 state chip", () => {
    const { container } = shell({ kind: "instance", id: "sess-1" });
    expect(container.textContent).toContain("Running");
  });
});

describe("AJ-E406 tab persistence per selection type (Req.3)", () => {
  it("restores a type's last tab across reselection", () => {
    const first = shell({ kind: "instance", id: "sess-1" });
    fireEvent.click(first.container.querySelector("#inspector-tab-usage")!);
    expect(localStorage.getItem("aij.inspector.tab.instance")).toBe("usage");
    cleanup();

    const other = shell({ kind: "project", id: "proj-1" });
    expect(other.container.querySelector("#inspector-tab-overview")!.getAttribute("aria-selected")).toBe("true");
    cleanup();

    const back = shell({ kind: "instance", id: "sess-1" });
    expect(back.container.querySelector("#inspector-tab-usage")!.getAttribute("aria-selected")).toBe("true");
  });
});

describe("AJ-E406 keyboard + honest empties + footers", () => {
  it("Cmd/Ctrl+1..6 switches tabs inside the inspector (Req.6)", () => {
    const { container } = shell({ kind: "account", id: "claude-work" });
    fireEvent.keyDown(window, { key: "3", metaKey: true });
    expect(container.querySelector("#inspector-tab-history")!.getAttribute("aria-selected")).toBe("true");
    fireEvent.keyDown(window, { key: "6", ctrlKey: true });
    expect(container.querySelector("#inspector-tab-actions")!.getAttribute("aria-selected")).toBe("true");
    // Focus moved to the selected tab button.
    expect(document.activeElement?.id).toBe("inspector-tab-actions");
  });

  it("a tab without data renders its honest empty state, never blank (D-06)", () => {
    const { container } = shell({ kind: "account", id: "claude-work" });
    fireEvent.click(container.querySelector("#inspector-tab-usage")!);
    expect(container.textContent).toContain("Unavailable");
    expect(container.textContent).toContain("usage engine");
  });

  it("every tab footer names the field's real source (Req.5)", () => {
    const { container } = shell({ kind: "instance", id: "sess-1" });
    const footer = container.querySelector("section:not([hidden]) footer");
    expect(footer?.textContent).toContain("source: supervisor");
    fireEvent.click(container.querySelector("#inspector-tab-usage")!);
    const usageFooter = container.querySelector("section:not([hidden]) footer");
    expect(usageFooter?.textContent).toContain("source: usage-engine");
  });

  it("switching tabs NEVER unmounts the embedded terminal (gate)", () => {
    const { container } = shell({ kind: "instance", id: "sess-1" });
    expect(container.querySelector('[data-testid="instance-terminal"]')).not.toBeNull();
    fireEvent.click(container.querySelector("#inspector-tab-history")!);
    // Still in the DOM (hidden panel) — no unmount, no focus steal.
    expect(container.querySelector('[data-testid="instance-terminal"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="instance-terminal"]')!.closest("section")!.hasAttribute("hidden")).toBe(true);
  });

  it("empty selection renders the fleet-at-a-glance summary, never a blank panel (Req.4)", () => {
    const { container } = shell({ kind: "none" });
    expect(container.querySelector('[data-testid="fleet-at-a-glance"]')).not.toBeNull();
    expect(container.textContent).toContain("Nothing selected");
    expect(container.textContent).toContain("By state");
    expect(container.textContent).toContain("Lowest quota");
  });

  it("remote-machine selections show the machine chip (Req.7)", () => {
    const { container } = shell({ kind: "instance", id: "sess-1" }, { machineId: "studio-desk" });
    expect(container.textContent).toContain("studio-desk");
  });

  it("the tab list is fixed order and labeled for a11y (WCAG 2.4.6)", () => {
    const { container } = shell({ kind: "project", id: "proj-1" });
    const tablist = container.querySelector("[role='tablist']");
    expect(tablist?.getAttribute("aria-label")).toBe("Project inspector tabs");
    for (const tab of INSPECTOR_TABS) {
      const panel = container.querySelector(`#inspector-panel-${tab.id}`);
      expect(panel?.getAttribute("aria-labelledby")).toBe(`inspector-tab-${tab.id}`);
    }
  });
});
