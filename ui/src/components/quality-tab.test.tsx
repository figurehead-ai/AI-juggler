import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, ProjectInfo } from "../lib/bridge";

/**
 * AJ-E409 evidence: the Quality/Performance tab — every signal labelled
 * with its tier, exact honesty copy, no composite score anywhere, and
 * the View-as-table text alternative.
 */

const NOW = 1_728_800_000_000;

// Hoisted: vi.mock factories run before module scope initializes, so
// the fixtures the factory closes over must live inside vi.hoisted.
const { qualityMock } = vi.hoisted(() => {
  const NOW = 1_728_800_000_000;
  const qualityFixtures: Record<string, import("../lib/bridge").QualitySignalsInfo> = {
    full: {
      tokens_bars: [420, 510, 380, 600, 540],
      avg_tokens_per_min: 490,
      error_label: "2 errors / 41 turns",
      errors_not_measurable: false,
      context_used: null,
      context_window: null,
      cumulative: { input: 12_345, output: 6_789, cache_read: 0, cache_write: 0, reasoning: 0 },
      session_cost_usd: 0.69,
      priced_model: "opus",
      quota_forecast: "not burning",
      stuck: { rule: "stuck_no_output", inputs: ["no output 6m", "cpu 0.3%"], since_ms: NOW - 6 * 60_000 },
    },
    idle: {
      tokens_bars: [0, 0, 0, 0, 0],
      avg_tokens_per_min: 0,
      error_label: "Not measurable (interactive)",
      errors_not_measurable: true,
      context_used: null,
      context_window: null,
      cumulative: { input: 0, output: 0, cache_read: 0, cache_write: 0, reasoning: 0 },
      session_cost_usd: null,
      priced_model: null,
      quota_forecast: "not burning",
      stuck: null,
    },
  };
  return {
    qualityMock: vi.fn(
      async (id: string): Promise<import("../lib/bridge").QualitySignalsInfo | null> =>
        qualityFixtures[id] ?? null,
    ),
  };
});
vi.mock("../lib/bridge", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/bridge")>();
  return { ...original, instanceQuality: qualityMock };
});
vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => <div data-terminal-id={id} />,
}));



function instance(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "opus" === null ? "unknown" : "launch",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 3_600_000,
    last_activity_ms: NOW,
    machine_id: "this-mac",
    cwd: "/w/x",
    kind: "terminal",
    isolation_label: null,
    project_id: null,
    conversation_id: null,
    ...overrides,
  };
}

const base = {
  projects: [] as ProjectInfo[],
  accounts: [{ id: "a1", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in" }] as AccountInfo[],
  profiles: [] as LaunchRecipeInfo[],
  terminalTitle: null,
  freshness: {},
  now: NOW,
  onOpenProject: () => {},
  onOpenSession: () => {},
  onLaunch: async () => {},
  onStopped: () => {},
  onHandover: () => {},
  onNew: () => {},
};

function qualityShell(row: InstanceInfo) {
  const view = render(
    <RightInspector
      selection={{ kind: "instance", id: row.id }}
      data={{ instance: row, project: null, account: null, provider: null, machineId: "this-mac" }}
      sessions={[row]}
      {...base}
    />,
  );
  fireEvent.click(view.container.querySelector("#inspector-tab-quality")!);
  return view;
}

afterEach(cleanup);
beforeEach(() => qualityMock.mockClear());

describe("AJ-E409 Quality tab signals (D-07)", () => {
  it("renders every signal with its tier tag and exact copy", async () => {
    const { container } = qualityShell(instance("full"));
    await waitFor(() => expect(container.querySelector('[data-testid="quality-tokens"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="quality-tokens"]')!.textContent).toContain("490 avg (5 min)");
    expect(container.querySelector('[data-testid="quality-tokens"]')!.textContent).toContain("Measured");
    expect(container.querySelector('[data-testid="quality-errors"]')!.textContent).toContain("2 errors / 41 turns");
    expect(container.querySelector('[data-testid="quality-context"]')!.textContent).toContain(
      "Context: not reported by claude-code",
    );
    expect(container.querySelector('[data-testid="quality-context"]')!.textContent).toContain("cumulative, not context");
    expect(container.querySelector('[data-testid="quality-cost"]')!.textContent).toContain("Est · value-equiv.");
    expect(container.querySelector('[data-testid="quality-cost"]')!.textContent).toContain("opus");
    expect(container.querySelector('[data-testid="quality-forecast"]')!.textContent).toContain("not burning");
    expect(container.querySelector('[data-testid="quality-stuck"]')!.textContent).toContain("Stuck?");
    expect(container.querySelector('[data-testid="quality-stuck"]')!.textContent).toContain("no output 6m");
  });

  it("idle 0 tok/min renders as a real zero — distinct from no data", async () => {
    const { container } = qualityShell(instance("idle"));
    await waitFor(() => expect(container.querySelector('[data-testid="quality-tokens"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="quality-tokens"]')!.textContent).toContain("0 avg (5 min)");
    expect(container.querySelector('[data-testid="quality-errors"]')!.textContent).toContain(
      "Not measurable (interactive)",
    );
    expect(container.querySelector('[data-testid="quality-cost"]')!.textContent).toContain("never $0.00");
  });

  it("no transcript at all renders the `no data` state", async () => {
    const { container } = qualityShell(instance("unknown"));
    await waitFor(() => expect(container.querySelector('[data-testid="quality-tab"]')).not.toBeNull());
    expect(container.textContent).toContain("no data — no transcript usage events yet");
  });

  it("View as table swaps the chart for a text alternative (a11y)", async () => {
    const { container } = qualityShell(instance("full"));
    await waitFor(() => expect(container.querySelector('[data-testid="quality-tokens"]')).not.toBeNull());
    // Scoped: the inspector now renders other aria-pressed toggles
    // (AJ-E209 ranked switch rows) above this tab's own toggle.
    const toggle = Array.from(
      container.querySelectorAll<HTMLButtonElement>("button[aria-pressed]"),
    ).find((button) => button.textContent?.includes("View as table"))!;
    expect(toggle.textContent).toContain("View as table");
    fireEvent.click(toggle);
    const table = container.querySelector('[data-testid="quality-tokens"] table');
    expect(table).not.toBeNull();
    expect(table!.textContent).toContain("600");
    expect(toggle.getAttribute("aria-pressed")).toBe("true");
  });

  it("no composite score exists in the UI (grep gate, component level)", async () => {
    const { container } = qualityShell(instance("full"));
    await waitFor(() => expect(container.querySelector('[data-testid="quality-tokens"]')).not.toBeNull());
    const text = (container.textContent ?? "").toLowerCase();
    for (const banned of ["quality score", "health score", "/100", "score:"]) {
      expect(text).not.toContain(banned);
    }
  });
});
