import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import type {
  AccountInfo,
  InstanceInfo,
  LaunchRecipeInfo,
  ProjectInfo,
} from "../lib/bridge";

/**
 * AJ-E408 evidence: Provider / Account / Project inspector content on a
 * 2-Provider × 3-Account × 2-Project fixture — worst-first aggregates,
 * `mixed` labelling, % text on quota rings, logged-out collapse,
 * missing-dir banner, `Usual` ranking, and the D-10 label law.
 */

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => (
    <div data-terminal-id={id} data-testid="instance-terminal" />
  ),
}));

afterEach(cleanup);
beforeEach(() => localStorage.clear());

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
    model_source: "opus" === null ? "unknown" : "launch",
    status: "running",
    quota: { used_percentage: 40, resets_at_ms: NOW + 3_600_000, confidence: "computed" },
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
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
    ...overrides,
  };
}

const ACCOUNTS: AccountInfo[] = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
  { id: "z-glm", provider: "zcode", label: "GLM Pro", plan: "glm-pro", login_state: "logged_out", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
];

const SESSIONS: InstanceInfo[] = [
  mk("s-work", { quota: { used_percentage: 55, resets_at_ms: NOW + 3_600_000, confidence: "computed" } }),
  mk("s-personal", {
    id: "s-personal",
    account_id: "claude-personal",
    account_label: "Personal",
    status: "finished",
    quota: { used_percentage: 95, resets_at_ms: NOW + 7_200_000, confidence: "estimated" },
    project_id: "proj-2",
    git_dirty_count: 3,
  }),
];

const PROJECTS: ProjectInfo[] = [
  { id: "proj-1", root: "/w/ai-juggler", name: "ai-juggler", last_activity_ms: null, active_instance: null, accounts_used: ["Work Max"], instance_count: 1, history_sessions: 2, missing: false },
  { id: "proj-2", root: "/w/gone", name: "gone", last_activity_ms: null, active_instance: null, accounts_used: ["Personal"], instance_count: 1, history_sessions: 0, missing: true },
];

const RECIPES: LaunchRecipeInfo[] = [
  { id: "p-work", name: "Deep work", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: "opus", default_cwd: null },
  { id: "p-personal", name: "Weekend", account_id: "claude-personal", account_label: "Personal", provider: "claude-code", model: null, default_cwd: null },
];

function shell(kind: "provider" | "account" | "project", id: string) {
  return render(
    <RightInspector
      selection={{ kind, id }}
      data={{
        instance: null,
        project: kind === "project" ? PROJECTS.find((p) => p.id === id) ?? null : null,
        account: kind === "account" ? ACCOUNTS.find((a) => a.id === id) ?? null : null,
        provider: kind === "provider" ? { id, name: id } : null,
        machineId: "this-mac",
      }}
      sessions={SESSIONS}
      projects={PROJECTS}
      accounts={ACCOUNTS}
      profiles={RECIPES}
      terminalTitle={null}
      freshness={{}}
      now={NOW}
      onOpenProject={() => {}}
      onOpenSession={() => {}}
      onLaunch={async () => {}}
      onStopped={() => {}}
      onHandover={() => {}}
      onNew={() => {}}
    />,
  );
}

const open = (container: HTMLElement, tab: string) =>
  fireEvent.click(container.querySelector(`#inspector-tab-${tab}`)!);

describe("AJ-E408 Provider (§5.3, Req.1-2)", () => {
  it("Overview: counts, worst-first aggregate, honest version/capability badges", () => {
    const { container } = shell("provider", "claude-code");
    expect(container.querySelector('[data-testid="provider-overview"]')).not.toBeNull();
    expect(container.textContent).toContain("2 accounts");
    expect(container.textContent).toContain("1 running");
    expect(container.textContent).toContain("version: No data");
    expect(container.textContent).toContain("capabilities: No data");
    // Worst first: Personal (95% used → 5% left) ABOVE Work Max (55% → 45%).
    const aggregate = container.querySelector('[data-testid="provider-aggregate"]')!;
    const order = [...aggregate.querySelectorAll("span")].map((n) => n.textContent ?? "");
    const personalIdx = order.findIndex((text) => text === "Personal");
    const workIdx = order.findIndex((text) => text === "Work Max");
    expect(personalIdx).toBeGreaterThan(-1);
    expect(personalIdx).toBeLessThan(workIdx);
  });

  it("mixed labelling appears when confidence tiers differ across accounts", () => {
    const { container } = shell("provider", "claude-code");
    expect(container.querySelector('[data-testid="provider-mixed"]')).not.toBeNull();
    expect(container.textContent).toContain("mixed");
  });

  it("Actions: Add account real; update-check disabled with the never-auto-update reason", () => {
    const { container } = shell("provider", "claude-code");
    open(container, "actions");
    expect(container.querySelector('[data-testid="provider-actions"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="action-add-account"] button')!.getAttribute("disabled")).toBeNull();
    const update = container.querySelector('[data-testid="action-check-for-provider-update"]')!;
    expect(update.textContent).toContain("never auto-update");
  });
});

describe("AJ-E408 Account (§5.4, Req.3-4)", () => {
  it("Overview: rings carry % text and aria labels; reset countdown; affinity; renewal honest", async () => {
    // AJ-E610: the account card renders engine-evaluated two-clock
    // rings — seed the display seam (5h: 55% used Measured with a
    // reset; weekly: unknown) and assert the two-clock copy.
    (
      window as unknown as { __AIJ_QUOTA_DISPLAY__?: unknown[] }
    ).__AIJ_QUOTA_DISPLAY__ = [
      {
        account: "claude-work",
        five_hour: {
          state: "tracking",
          percentage: 55,
          badge: "measured",
          stale_note_ms: undefined,
          resets_at_ms: NOW + 3_600_000,
          burned_tokens: 12_345,
        },
        weekly: { state: "unknown", burned_tokens: 0 },
        renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false },
        forecast: { state: "learning", badge: "estimated" },
      },
    ];
    const { container } = shell("account", "claude-work");
    const rings = container.querySelector('[data-testid="account-rings"]')!;
    await waitFor(() =>
      expect(rings.querySelectorAll("[role='img']").length).toBe(2),
    );
    expect(rings.textContent).toContain("55%");
    expect(rings.textContent).toContain("Start a session to measure");
    const fiveHourAria = rings.querySelector('[role="img"]')!.getAttribute("aria-label")!;
    expect(fiveHourAria).toContain("55% used");
    expect(fiveHourAria).toContain("Measured");
    expect(container.querySelector('[data-testid="account-reset"]')?.textContent).toContain("1h0m");
    expect(container.querySelector('[data-testid="account-renewal"]')?.textContent).toContain("Expiry: unknown");
    expect(container.querySelector('[data-testid="account-affinity"]')?.textContent).toContain("ai-juggler");
    // AJ-E410 (D-08): the Tier-1 cleanup opt-in toggle — off by default.
    const toggle = container.querySelector('[data-testid="account-cleanup-toggle"]')! as HTMLInputElement;
    expect(toggle).not.toBeNull();
    expect(toggle.querySelector("input")!.checked).toBe(false);
    expect(container.querySelector('[data-testid="account-overview"]')!.textContent).toContain(
      "Both off by default",
    );
    expect(container.querySelector('[data-testid="account-overview"]')!.textContent).toContain(
      "audit-logged (D-08)",
    );
  });

  it("logged-out accounts collapse Overview to the sign-in card", () => {
    const { container } = shell("account", "z-glm");
    expect(container.querySelector('[data-testid="account-signin-card"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="account-rings"]')).toBeNull();
    expect(container.textContent).toContain("signed out");
  });

  it("History groups sessions Today; Actions honour D-10 (`New launch settings…`)", () => {
    const { container } = shell("account", "claude-work");
    open(container, "history");
    expect(container.querySelector('[data-testid="account-history"]')).not.toBeNull();
    expect(container.textContent).toContain("Today");
    cleanup();
    const actions = shell("account", "claude-work");
    open(actions.container, "actions");
    const newSettings = actions.container.querySelector('[data-testid="action-new-launch-settings"]');
    expect(newSettings?.textContent ?? "").toContain("launch settings");
    expect(actions.container.textContent).not.toContain("New profile");
    // Delete keeps the managed-dir note adjacent (deep 02 PF-08).
    expect(actions.container.querySelector('[data-testid="action-delete-account"]')?.textContent).toContain("directory is kept");
  });
});

describe("AJ-E408 Project (§5.5, Req.5-6)", () => {
  it("Overview: On-it-now card, copyable root, branch summary, dirty total, counts", () => {
    const { container } = shell("project", "proj-1");
    expect(container.querySelector('[data-testid="project-on-it-now"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="project-root"]')?.textContent).toContain("/w/ai-juggler");
    const summary = container.querySelector('[data-testid="project-summary"]')!;
    expect(summary.textContent).toContain("main");
    expect(summary.textContent).toContain("Work Max");
    expect(summary.textContent).toContain("imported sessions: 2");
  });

  it("a missing directory renders the danger banner + Relocate…", () => {
    const { container } = shell("project", "proj-2");
    const banner = container.querySelector('[data-testid="project-missing-banner"]');
    expect(banner).not.toBeNull();
    expect(banner!.getAttribute("role")).toBe("alert");
    expect(banner!.textContent).toContain("missing");
    expect(banner!.textContent).toContain("Relocate…");
  });

  it("Actions: ranked by headroom with the `Usual` star", () => {
    const { container } = shell("project", "proj-1");
    open(container, "actions");
    const buttons = [...container.querySelectorAll('[data-testid="project-actions"] > button')];
    expect(buttons.length).toBeGreaterThanOrEqual(2);
    // Work Max (45% left) ranks above Personal (5% left).
    expect(buttons[0]!.textContent).toContain("Work Max");
    expect(buttons[0]!.textContent).toContain("Usual");
    expect(buttons[1]!.textContent).toContain("Personal");
  });

  it("a project with no sessions renders the Start-first-session CTA", () => {
    const empty: ProjectInfo = { ...PROJECTS[0]!, id: "proj-empty", accounts_used: [], instance_count: 0 };
    const { container } = render(
      <RightInspector
        selection={{ kind: "project", id: "proj-empty" }}
        data={{ instance: null, project: empty, account: null, provider: null, machineId: "this-mac" }}
        sessions={[]}
        projects={[empty]}
        accounts={ACCOUNTS}
        profiles={RECIPES}
        terminalTitle={null}
        freshness={{}}
        now={NOW}
        onOpenProject={() => {}}
        onOpenSession={() => {}}
        onLaunch={async () => {}}
        onStopped={() => {}}
        onHandover={() => {}}
        onNew={() => {}}
      />,
    );
    expect(container.textContent).toContain("No sessions in this project yet");
    expect(container.textContent).toContain("Start first session");
  });
});

describe("AJ-E408 traceability (Req.7)", () => {
  it("every aggregate row names its account (no unexplainable numbers)", () => {
    const { container } = shell("provider", "claude-code");
    const aggregate = container.querySelector('[data-testid="provider-aggregate"]')!;
    const rows = [...aggregate.querySelectorAll(":scope > div")];
    for (const row of rows) {
      expect(row.textContent).toMatch(/Work Max|Personal/);
    }
  });
});
