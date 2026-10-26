import { cleanup, fireEvent, render, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector, installWords, useToolInstall } from "./RightInspector";
import { since } from "../lib/time-words";
import { quotaDisplay, windowDisplay } from "../lib/wire";
import { providerDisplayName } from "../lib/providers";
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
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "z-glm", provider: "zcode", label: "GLM Pro", plan: "glm-pro", login_state: "logged_out", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
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
  { id: "proj-1", root: "/w/ai-juggler", name: "ai-juggler", last_activity_ms: null, active_instance: null, accounts_used: ["Work Max"], instance_count: 1, history_sessions: 2, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-2", root: "/w/gone", name: "gone", last_activity_ms: null, active_instance: null, accounts_used: ["Personal"], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: true },
];

const RECIPES: LaunchRecipeInfo[] = [
  { id: "p-work", name: "Deep work", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: "opus", default_cwd: null, restart_on_crash: null },
  { id: "p-personal", name: "Weekend", account_id: "claude-personal", account_label: "Personal", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null },
];

function shell(kind: "provider" | "account" | "project", id: string) {
  return render(
    <RightInspector
      selection={{ kind, id }}
      data={{
        instance: null,
        project: kind === "project" ? PROJECTS.find((p) => p.id === id) ?? null : null,
        account: kind === "account" ? ACCOUNTS.find((a) => a.id === id) ?? null : null,
        provider: kind === "provider" ? { id, name: providerDisplayName(id) } : null,
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
  fireEvent.click(container.querySelector(`#page-tab-${tab}`)!);

describe("AJ-E408 Provider (§5.3, Req.1-2)", () => {
  // What Runway reads: each account's measured 5-hour window.
  beforeEach(() => {
    (window as unknown as Record<string, unknown>).__AIJ_QUOTA_DISPLAY__ = [
      quotaDisplay("claude-personal", { five_hour: windowDisplay("tracking", { percentage: 95 }) }),
      quotaDisplay("claude-work", { five_hour: windowDisplay("tracking", { percentage: 55 }) }),
    ];
  });
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__AIJ_QUOTA_DISPLAY__;
  });

  it("Overview: counts, worst-first aggregate, what's installed (no placeholders)", async () => {
    const { container } = shell("provider", "claude-code");
    expect(container.querySelector('[data-testid="provider-overview"]')).not.toBeNull();
    expect(container.textContent).toContain("2 accounts");
    expect(container.textContent).toContain("1 running");
    // D-25: the installed version and tested range, read from this Mac —
    // not "version: No data".
    await waitFor(() => expect(container.querySelector('[data-testid="provider-install"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="provider-install"]')!.textContent).toMatch(/^Version \S+/);
    expect(container.textContent).not.toContain("No data");
    // The least left first, by Runway's own line: Personal (95% used →
    // 5% left) ABOVE Work Max (55% → 45%).
    const aggregate = container.querySelector('[data-testid="provider-aggregate"]')!;
    await waitFor(() => expect(aggregate.textContent).toContain("5% left"));
    const order = [...aggregate.querySelectorAll("span")].map((n) => n.textContent ?? "");
    const personalIdx = order.findIndex((text) => text === "Personal");
    const workIdx = order.findIndex((text) => text === "Work Max");
    expect(personalIdx).toBeGreaterThan(-1);
    expect(personalIdx).toBeLessThan(workIdx);
  });

  it("each account's line is Runway's own (one source per fact, AJ-E1533)", async () => {
    const { container } = shell("provider", "claude-code");
    const aggregate = container.querySelector('[data-testid="provider-aggregate"]')!;
    await waitFor(() => expect(aggregate.querySelectorAll('[data-testid="account-runway"]').length).toBe(2));
    expect(aggregate.textContent).toContain("45% left");
  });

  it("Add account is a header button; actions not built yet aren't shown (D-25)", () => {
    const { getByTestId, container } = shell("provider", "claude-code");
    expect(getByTestId("header-add-account").textContent).toBe("Add account…");
    expect(container.querySelector("#page-tab-actions")).toBeNull();
    expect(container.textContent).not.toMatch(/Check for an update|Open tool settings/);
  });

  it("a failed install check says so instead of showing nothing", async () => {
    (window as unknown as Record<string, unknown>).__AIJ_PROVIDER_INSTALLS_ERROR__ = new Error("probe failed");
    try {
      const { container } = shell("provider", "claude-code");
      await waitFor(() => expect(container.querySelector('[data-testid="provider-install"]')?.textContent).toBe("Couldn't check what's installed"));
    } finally {
      delete (window as unknown as Record<string, unknown>).__AIJ_PROVIDER_INSTALLS_ERROR__;
    }
  });

  it("another tool's version never shows while the new tool is checked", async () => {
    const hook = renderHook(({ id }: { id: string }) => useToolInstall(id), { initialProps: { id: "claude-code" } });
    await waitFor(() => expect(hook.result.current).not.toBeNull());
    expect((hook.result.current as { provider: string }).provider).toBe("claude-code");
    hook.rerender({ id: "codex" });
    // Straight after the switch: nothing, not Claude Code's version.
    expect(hook.result.current).toBeNull();
    await waitFor(() => expect(hook.result.current).not.toBeNull());
    const now = hook.result.current;
    expect(now === "failed" || (now as { provider: string }).provider === "codex").toBe(true);
    hook.unmount();
  });

  it("how long ago: seconds, minutes, hours, then days past 48 h", () => {
    const now = 1_000_000_000_000;
    expect(since(now - 30_000, now)).toBe("30s");
    expect(since(now - 12 * 60_000, now)).toBe("12m");
    expect(since(now - 30 * 3_600_000, now)).toBe("30h");
    expect(since(now - 47 * 3_600_000, now)).toBe("47h");
    expect(since(now - 48 * 3_600_000, now)).toBe("2d");
    expect(since(now - 168 * 3_600_000, now)).toBe("7d");
  });

  it("says what's installed in plain words", () => {
    const base = { provider: "claude-code", display_name: "Claude Code", install_url: "", binary: null, notes: [], kind: "terminal", single_instance_note: null } as const;
    expect(installWords({ ...base, status: "installed", version: "2.1.296", verified_against: ["2.1.x"] } as never)).toBe("Version 2.1.296 · tested with 2.1.x");
    expect(installWords({ ...base, status: "installed", version: null, verified_against: [] } as never)).toBe("Installed");
    expect(installWords({ ...base, status: "unverified", version: "3.0.0", verified_against: ["2.1.x"] } as never)).toBe(
      "Version 3.0.0 · newer than AI Juggler has been tested with",
    );
    // A tool no version of which was tried yet isn't "newer" than one.
    expect(installWords({ ...base, status: "unverified", version: "1.19.3", verified_against: [] } as never)).toBe(
      "Version 1.19.3 · not tested with AI Juggler yet",
    );
    expect(installWords({ ...base, status: "not_installed", version: null, verified_against: [] } as never)).toBe("Not installed on this Mac");
    expect(installWords({ ...base, status: "error", version: null, verified_against: [] } as never)).toBe("Couldn't check what's installed");
  });

  it("no tab of the tool, account or project views shows a task code or an internal id (D-25)", () => {
    for (const [kind, id] of [["provider", "claude-code"], ["account", "claude-work"], ["project", "proj-1"], ["project", "proj-2"]] as const) {
      const { container } = shell(kind, id);
      const tabs = [...container.querySelectorAll<HTMLElement>("[role='tab']")].map((tab) => tab.id.replace("page-tab-", ""));
      for (const tab of tabs.length > 0 ? tabs : ["body"]) {
        if (tab !== "body") open(container, tab);
        // innerText-like: hidden screen-reader text counts too.
        const text = container.textContent ?? "";
        expect(text, `${kind} › ${tab}`).not.toMatch(/\bAJ-E\d+|\bE\d+\+|\(E\d+\)|\bD-\d{2}\b/);
        expect(text, `${kind} › ${tab}`).not.toMatch(/claude-code|\b[Ii]nstances?\b|lands with|no data|now ago/i);
      }
      cleanup();
    }
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
    // 0.1.5: a gauge per window, what's left in words and in its ring.
    await waitFor(() =>
      expect(container.querySelectorAll("[data-testid='window-gauge']").length).toBe(2),
    );
    expect(rings.textContent).toContain("45%");
    expect(rings.textContent).toContain("Not measured yet");
    const fiveHourAria = rings.querySelector('[data-testid="window-gauge"]')!.getAttribute("aria-label")!;
    expect(fiveHourAria).toContain("45% left");
    expect(fiveHourAria).toContain("in 1 h");
    expect(container.querySelector('[data-testid="account-reset"]')?.textContent).toContain("in 1 h");
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
      "every stop is written to the log",
    );
  });

  it("logged-out accounts collapse Overview to the sign-in card", () => {
    const { container } = shell("account", "z-glm");
    expect(container.querySelector('[data-testid="account-signin-card"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="account-rings"]')).toBeNull();
    expect(container.textContent).toContain("signed out");
  });

  it("Sessions lists the account's sessions with how long ago; its actions are the header's (D-25)", () => {
    const { container, getAllByTestId } = shell("account", "claude-work");
    open(container, "sessions");
    expect(getAllByTestId("page-session-row").length).toBeGreaterThan(0);
    expect(container.textContent).not.toContain("now ago");
    expect(container.querySelector("#page-tab-actions")).toBeNull();
    expect(container.textContent).not.toContain("New profile");
  });
});

describe("AJ-E408 Project (§5.5, Req.5-6)", () => {
  it("the header says where it is and its branch; Overview, its sessions ever and accounts", async () => {
    const { container, getByTestId } = shell("project", "proj-1");
    expect(getByTestId("project-root").getAttribute("title")).toBe("/w/ai-juggler");
    expect(getByTestId("page-facts").textContent).toContain("main");
    open(container, "overview");
    const summary = getByTestId("project-summary");
    expect(summary.textContent).toContain("Work Max");
    await waitFor(() => expect(getByTestId("project-sessions-ever").textContent).toMatch(/^\d+ with usage in all$/));
  });

  it("a missing directory renders the danger banner with Relocate… (AJ-E1502)", () => {
    const { container } = shell("project", "proj-2");
    const banner = container.querySelector('[data-testid="project-missing-banner"]');
    expect(banner).not.toBeNull();
    expect(banner!.getAttribute("role")).toBe("alert");
    expect(banner!.textContent).toContain("missing");
    const relocate = [...banner!.querySelectorAll("button")].find((button) => button.textContent === "Relocate…");
    expect(relocate).toBeDefined();
    expect((relocate as HTMLButtonElement).disabled).toBe(false);
  });

  it("Overview's Accounts: ranked by headroom with the `Usual` star", () => {
    const { container } = shell("project", "proj-1");
    open(container, "overview");
    const buttons = [...container.querySelectorAll('[data-testid="project-accounts"] button')].filter((button) => !button.closest('[data-testid="project-default-account"]'));
    expect(buttons.length).toBeGreaterThanOrEqual(2);
    // Work Max (45% left) ranks above Personal (5% left).
    expect(buttons[0]!.textContent).toContain("Work Max");
    expect(buttons[0]!.textContent).toContain("Usual");
    expect(buttons[1]!.textContent).toContain("Personal");
  });

  it("a project with no sessions renders the Start-first-session CTA", () => {
    const empty: ProjectInfo = { ...PROJECTS[0]!, id: "proj-empty", accounts_used: [], instance_count: 0, history_sessions: 0 };
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

  it("a project known only from history says what was done there (0.0.11 walk, Q2)", () => {
    const history: ProjectInfo = {
      ...PROJECTS[0]!, id: "proj-history", accounts_used: ["Work Max"], instance_count: 0,
      history_sessions: 12, history_last_used_ms: NOW - 26 * 3_600_000,
    };
    const { container, getByTestId } = render(
      <RightInspector
        selection={{ kind: "project", id: "proj-history" }}
        data={{ instance: null, project: history, account: null, provider: null, machineId: "this-mac" }}
        sessions={[]}
        projects={[history]}
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
    expect(container.textContent).toContain("Nothing started from AI Juggler here yet");
    expect(container.textContent).not.toContain("No sessions in this project yet");
    // The rail row's own line: when, how many conversations, which accounts.
    expect(getByTestId("project-history-line").textContent).toBe("1 d ago · 12 conversations · Work Max");
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
