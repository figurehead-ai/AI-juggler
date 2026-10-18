import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { SessionCard, SessionRow } from "./WorkflowRail";
import type { InstanceInfo } from "../lib/bridge";

/**
 * AJ-E402 evidence: the indicator row/card across the 14 named fixtures
 * — 9 states, no-quota, no-pricing, non-git, 0 children, unknown wire
 * status — asserting the exact honesty copy. Never `$0.00`, never a
 * fabricated quota/model/child count.
 */

afterEach(cleanup);

const NOW = Date.now();

function mk(overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-test",
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
    quota: { used_percentage: 42.5, resets_at_ms: NOW + 3_600_000, confidence: "computed" },
    tokens: { input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 },
    cpu_pct: 3.5,
    rss_bytes: 123_456,
    child_count: 2,
    subtask_count: null,
    git_branch: "feat/read-model",
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 42 * 60_000,
    last_activity_ms: NOW - 10_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-1",
    conversation_id: null,
    ...overrides,
  };
}

const rowText = (session: InstanceInfo, variant: "row" | "card" = "card"): string => {
  const { container } = render(
    variant === "row" ? (
      <SessionRow session={session} selected={false} onSelect={() => {}} />
    ) : (
      <SessionCard session={session} />
    ),
  );
  return container.textContent ?? "";
};

describe("AJ-E402 SessionRow — the 9 SessionStatus states (icon + text + colour)", () => {
  const states: [string, string][] = [
    ["starting", "Starting"],
    ["running", "Running"],
    ["waiting_user", "Needs you"],
    ["stuck", "Stuck?"],
    ["quota_blocked", "Quota blocked"],
    ["finished", "Finished"],
    ["stopped", "Stopped"],
    ["failed", "Failed"],
    ["handed_over", "Handed over"],
  ];

  for (const [status, label] of states) {
    it(`renders the ${status} state as text`, () => {
      const attention =
        status === "waiting_user" || status === "stuck" || status === "quota_blocked" || status === "failed";
      const text = rowText(
        mk({ status, needs_attention: attention ? (status as InstanceInfo["needs_attention"]) : null }),
      );
      expect(text).toContain(label);
    });
  }
});

describe("AJ-E402 honesty copy — absence is exact, never fabricated", () => {
  it("quota-less row renders `Quota: no data`, not a zero bar", () => {
    const text = rowText(mk({ quota: null }));
    expect(text).toContain("Quota: no data");
    expect(text).not.toContain("% left");
  });

  it("no pricing source renders tokens only — never `$0.00`", () => {
    const text = rowText(mk({}));
    expect(text).toContain("19.1k tok");
    expect(text).not.toContain("$");
  });

  it("never-observed tokens render no token text at all", () => {
    const text = rowText(mk({ tokens: null }));
    expect(text).not.toContain("tok");
  });

  it("non-git cwd omits the branch field entirely", () => {
    const text = rowText(mk({ git_branch: null }));
    expect(text).not.toContain("feat/read-model");
    expect(screen.queryByTitle("branch feat/read-model")).toBeNull();
  });

  it("0 children hides the badge (no `⑂ 0` noise)", () => {
    const text = rowText(mk({ child_count: 0 }));
    expect(text).not.toContain("children");
  });

  it("unknown wire status renders `Unknown`, never a neutral dot", () => {
    const text = rowText(
      mk({ status: "solar-flare", needs_attention: null }),
      "row",
    );
    expect(text).toContain("Unknown");
  });

  it("model-less row renders `default model`, never a guessed name", () => {
    const text = rowText(mk({ model: null }));
    expect(text).toContain("default model");
  });

  it("never-active row renders `no output yet`", () => {
    const text = rowText(mk({ last_activity_ms: null }));
    expect(text).toContain("no output yet");
  });

  it("just-started row renders `starting…`", () => {
    const text = rowText(mk({ started_at_ms: NOW - 1_000 }));
    expect(text).toContain("starting…");
  });

  it("unsampled resources render the dash, not 0%", () => {
    const text = rowText(mk({ cpu_pct: null, rss_bytes: null }));
    expect(text).toContain("—");
    expect(text).not.toContain("0% ·");
  });

  it("waiting row carries a wait duration from last activity", () => {
    const text = rowText(
      mk({ status: "waiting_user", needs_attention: "waiting_user", last_activity_ms: NOW - 4 * 60_000 }),
    );
    expect(text).toContain("wait 4m");
  });

  it("quota-blocked row carries the reset countdown when known", () => {
    const text = rowText(
      mk({ status: "quota_blocked", needs_attention: "quota_blocked" }),
    );
    expect(text).toContain("resets");
  });
});

describe("AJ-E402 card/row parity and screen-reader access", () => {
  it("the card renders the same field text as the row (no unexplainable numbers)", () => {
    const session = mk();
    const card = rowText(session, "card");
    // The narrow rail row carries the account as initials disc + title;
    // accessible parity is the row's aria-label (I-02 without the room).
    const { container } = render(
      <SessionRow session={session} selected={false} onSelect={() => {}} />,
    );
    const row = container.querySelector("button")?.getAttribute("aria-label") ?? "";
    for (const field of [
      "Claude",
      "Work Max",
      "opus",
      "Running",
      "% left",
      "19.1k tok",
      "feat/read-model",
    ]) {
      expect(card).toContain(field);
      expect(row).toContain(field);
    }
  });

  it("every number is reachable as text via the row's aria-label", () => {
    const session = mk();
    render(<SessionRow session={session} selected={false} onSelect={() => {}} />);
    const label = screen.getByRole("button", { name: /Work Max/ }).getAttribute("aria-label") ?? "";
    for (const part of [
      "Claude provider",
      "Work Max",
      "opus",
      "Running",
      "quota 58% left, computed",
      "19.1k tok",
      "cpu 4%, ram 121 KB",
      "2 children",
      "branch feat/read-model",
    ]) {
      expect(label).toContain(part);
    }
  });

  it("aria-label degrades honestly with no data", () => {
    render(
      <SessionRow
        session={mk({ quota: null, tokens: null, model: null, cpu_pct: null, rss_bytes: null })}
        selected={false}
        onSelect={() => {}}
      />,
    );
    const label = screen.getByRole("button", { name: /Work Max/ }).getAttribute("aria-label") ?? "";
    expect(label).toContain("default model");
    expect(label).toContain("Quota: no data");
  });

  it("renders the generic Tool mark for an unknown provider", () => {
    const text = rowText(mk({ provider: "some-new-harness" }));
    expect(text).toContain("Tool");
  });
});

describe("AJ-E405 structured reasons + live announcements", () => {
  it("the badge tooltip and aria-label carry the measured rule inputs verbatim", () => {
    const session = mk({
      status: "waiting_user",
      needs_attention: "waiting_user",
      attention: {
        rule: "waiting_prompt",
        inputs: ["prompt alive", "no output 4m"],
        since_ms: NOW - 4 * 60_000,
      },
    });
    const { container } = render(
      <SessionRow session={session} selected={false} onSelect={() => {}} />,
    );
    const badge = container.querySelector("[title='waiting_prompt · prompt alive · no output 4m']");
    expect(badge).not.toBeNull();
    const label = container.querySelector("button")!.getAttribute("aria-label") ?? "";
    expect(label).toContain("waiting_prompt · prompt alive · no output 4m");
  });

  it("the polite live region announces attention transitions (WCAG 4.1.3)", async () => {
    const { AttentionAnnouncer } = await import("./indicator");
    const healthy = mk({ id: "a1" });
    const { rerender } = render(<AttentionAnnouncer sessions={[healthy]} />);
    const waiting = mk({
      id: "a1",
      status: "waiting_user",
      needs_attention: "waiting_user",
      attention: { rule: "waiting_prompt", inputs: ["prompt alive"], since_ms: NOW },
    });
    rerender(<AttentionAnnouncer sessions={[waiting]} />);
    const region = document.querySelector('[data-testid="attention-announcer"]')!;
    expect(region.getAttribute("aria-live")).toBe("polite");
    expect(region.textContent).toContain("Work Max needs you — waiting_prompt");
    rerender(<AttentionAnnouncer sessions={[healthy]} />);
    expect(document.querySelector('[data-testid="attention-announcer"]')!.textContent).toContain(
      "recovered",
    );
  });
});
