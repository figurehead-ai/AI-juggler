import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import { RenewalAnnouncer } from "./indicator";
import { renewalLabel, renewalRowVisible } from "../lib/renewal";
import { sortSessions } from "../lib/railview";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, ProjectInfo } from "../lib/bridge";

/**
 * AJ-E411 evidence (D-12): the renewal-date states at every threshold
 * boundary, the user-entered-only editor, the row chip rule, the
 * once-per-day polite announcement, and the Needs-me tie-break that
 * never outranks a live blocked agent.
 */

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => <div data-terminal-id={id} />,
}));

const setRenewalMock = vi.fn(async (_id: string, _date: string | null) => {
  throw new Error("not wired in this test");
});

const NOW = new Date("2026-10-14T12:00:00").getTime();

function daysOut(days: number): string {
  const target = new Date(NOW + days * 86_400_000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${target.getFullYear()}-${pad(target.getMonth() + 1)}-${pad(target.getDate())}`;
}

function account(overrides: Partial<AccountInfo> = {}): AccountInfo {
  return {
    id: "a1",
    provider: "claude-code",
    label: "Work Max",
    plan: null,
    login_state: "logged_in",
    cleanup_opt_in: false,
    preview_opt_in: false,
    renewal_date: null,
    ...overrides,
  };
}

function instance(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    model: null,
    launch_model: null,
    running_model: null,
    model_source: null === null ? "unknown" : "launch",
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

function accountShell(row: AccountInfo) {
  const view = render(
    <RightInspector
      selection={{ kind: "account", id: row.id }}
      data={{ instance: null, project: null, account: row, provider: null, machineId: "this-mac" }}
      sessions={[]}
      accounts={[row]}
      {...base}
    />,
  );
  return view;
}

afterEach(cleanup);
beforeEach(() => {
  setRenewalMock.mockClear();
  localStorage.clear();
});

describe("AJ-E411 threshold labels (TS mirror of renewal.rs)", () => {
  it("renders every boundary tier with the exact copy", () => {
    expect(renewalLabel(null, NOW)).toBeNull(); // unknown, never guessed
    expect(renewalLabel(daysOut(15), NOW)).toEqual({ tier: "date", text: `Renews ${daysOut(15)}` });
    expect(renewalLabel(daysOut(14), NOW)?.text).toBe("Renews in 14 days");
    expect(renewalLabel(daysOut(8), NOW)?.tier).toBe("countdown");
    expect(renewalLabel(daysOut(7), NOW)).toEqual({ tier: "warning", text: "Renews in 7 days" });
    expect(renewalLabel(daysOut(1), NOW)?.text).toBe("Renews in 1 days");
    expect(renewalLabel(daysOut(0), NOW)).toEqual({ tier: "danger", text: "Renewal date passed" });
    expect(renewalLabel(daysOut(-1), NOW)?.text).toBe("Renewal date passed");
    // Row visibility: only at <= 7 days (Req.2/Req.5).
    expect(renewalRowVisible(renewalLabel(daysOut(8), NOW))).toBe(false);
    expect(renewalRowVisible(renewalLabel(daysOut(7), NOW))).toBe(true);
    expect(renewalRowVisible(renewalLabel(daysOut(0), NOW))).toBe(true);
  });
});

describe("AJ-E411 Account inspector states + editor", () => {
  it("renders Expiry: unknown when no date exists", () => {
    const { container } = accountShell(account());
    expect(container.querySelector('[data-testid="account-renewal"]')!.textContent).toContain(
      "Expiry: unknown",
    );
  });

  it("renders the date-only, countdown, warning, and danger states as text chips", () => {
    const cases: Array<[string, string, string]> = [
      [daysOut(20), `Renews ${daysOut(20)}`, "plain"],
      [daysOut(10), "Renews in 10 days", "plain"],
      [daysOut(6), "Renews in 6 days", "warning"],
      [daysOut(0), "Renewal date passed", "danger"],
    ];
    for (const [date, text, kind] of cases) {
      cleanup();
      const { container } = accountShell(account({ renewal_date: date }));
      const node = container.querySelector('[data-testid="account-renewal"]')!;
      expect(node.textContent).toContain(text);
      const chip = node.querySelector("[class*='warning'], [class*='danger']");
      if (kind === "plain") {
        expect(chip).toBeNull(); // never colour-only, and no chip before the threshold
      } else {
        expect(chip).not.toBeNull();
      }
    }
  });

  it("hosts the editor: labelled date input, Clear, and the D-12 helper text", () => {
    const { container } = accountShell(account({ renewal_date: daysOut(6) }));
    const editor = container.querySelector('[data-testid="renewal-editor"]')!;
    const input = editor.querySelector("input[type='date']") as HTMLInputElement;
    expect(input).not.toBeNull();
    expect(input.getAttribute("id")).toBe("renewal-a1");
    expect(editor.querySelector(`label[for="renewal-a1"]`)!.textContent).toBe(
      "Renewal date (optional)",
    );
    expect(editor.textContent).toContain("Entered by you - Juggler does not read billing");
    expect(input.value).toBe(daysOut(6));
    expect(editor.querySelector("button")!.textContent).toContain("Clear");
  });

  it("Actions offers Edit renewal date... which jumps to the Overview editor", () => {
    const row = account();
    const { container } = render(
      <RightInspector
        selection={{ kind: "account", id: row.id }}
        data={{ instance: null, project: null, account: row, provider: null, machineId: "this-mac" }}
        sessions={[]}
        accounts={[row]}
        {...base}
      />,
    );
    fireEvent.click(container.querySelector("#inspector-tab-actions")!);
    const entry = [...container.querySelectorAll("button")].find(
      (button) => button.textContent === "Edit renewal date...",
    )!;
    expect(entry).toBeDefined();
    const listener = vi.fn();
    window.addEventListener("aij:edit-renewal", listener);
    fireEvent.click(entry);
    expect(listener).toHaveBeenCalled();
    window.removeEventListener("aij:edit-renewal", listener);
  });
});

describe("AJ-E411 once-per-day polite announcement", () => {
  it("announces a crossing account once per local day, not on every tick", () => {
    const expiring = account({ renewal_date: daysOut(3) });
    const { rerender, container } = render(<RenewalAnnouncer accounts={[expiring]} now={NOW} />);
    expect(container.querySelector('[data-testid="renewal-announcer"]')!.textContent).toContain(
      "Work Max renewal: Renews in 3 days",
    );
    // Same-day re-render: no repeat.
    rerender(<RenewalAnnouncer accounts={[expiring]} now={NOW + 60_000} />);
    expect(container.querySelector('[data-testid="renewal-announcer"]')!.textContent).toBe("");
    // Far-future account: never announced.
    cleanup();
    const { container: quiet } = render(
      <RenewalAnnouncer accounts={[account({ renewal_date: daysOut(30) })]} now={NOW} />,
    );
    expect(quiet.querySelector('[data-testid="renewal-announcer"]')!.textContent).toBe("");
  });
});

describe("AJ-E411 Needs-me tie-break (Req.5)", () => {
  const rows: InstanceInfo[] = [
    instance("plain-running"),
    instance("expiring-account", { account_id: "a-expiring" }),
  ];
  const accounts: AccountInfo[] = [
    account({ id: "a1" }),
    account({ id: "a-expiring", renewal_date: daysOut(5) }),
  ];

  it("an expiring account rises among equal-severity peers", () => {
    const ordered = sortSessions(rows, "needs", {}, NOW, [], accounts);
    expect(ordered[0].id).toBe("expiring-account");
  });

  it("expiry never outranks a live blocked agent", () => {
    const failed = instance("failed-agent", { status: "failed", needs_attention: "failed" });
    const ordered = sortSessions([failed, ...rows], "needs", {}, NOW, [], accounts);
    expect(ordered[0].id).toBe("failed-agent"); // severity first, always
  });
});
