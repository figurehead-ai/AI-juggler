/**
 * AJ-E2104 Req.3 — one state vocabulary everywhere a row shows a state:
 * the Project lens's rows, the icon column, other Macs' rows and the
 * legend say what the place rows say (Asks you, Probably asks you,
 * Working, Idle, Done, review it, Ended), with why it needs you as the
 * reason ("Limit reached", "Probably stuck").
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RailRow } from "./WorkflowRail";
import { IndicatorLegend } from "./indicator";
import { ariaSummary } from "./row-details";
import { sessionState, statusState } from "../lib/rail-items";
import type { InstanceInfo } from "../lib/bridge";

const NOW = 1_728_800_000_000;
const OLD = ["Running", "Starting", "Waiting for you", "Looks stuck", "Hit its limit", "Failed", "Handed over", "Needs you", "Stuck?", "Quota blocked"];

function session(status: InstanceInfo["status"]): InstanceInfo {
  const attention = status === "waiting_user" || status === "stuck" || status === "quota_blocked" || status === "failed" ? status : null;
  return {
    id: `s-${status}`, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "a1", account_label: "Work Max", model: "opus",
    launch_model: "opus", running_model: null, model_source: "launch", status, quota: null, tokens: null, cpu_pct: null, rss_bytes: null,
    child_count: null, subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: attention, attention: null,
    started_at_ms: NOW - 60_000, last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/w/app", kind: "terminal", isolation_label: null,
    project_id: null, conversation_id: null,
  } as InstanceInfo;
}

const EXPECTED: [InstanceInfo["status"], string][] = [
  ["starting", "Working"],
  ["running", "Working"],
  ["waiting_user", "Asks you"],
  ["stuck", "Probably stuck"],
  ["quota_blocked", "Limit reached"],
  ["failed", "Stopped with an error"],
  ["finished", "Ended"],
  ["stopped", "Ended"],
  ["handed_over", "Ended"],
];

afterEach(cleanup);

describe("one state vocabulary (AJ-E2104 Req.3)", () => {
  it("a session's state, or why it needs you, in the place rows' words", () => {
    for (const [status, word] of EXPECTED) expect(sessionState(session(status)).word, status).toBe(word);
    expect(sessionState(session("waiting_user"))).toMatchObject({ state: "asks_you", needs: true, tone: "text-warning" });
    expect(sessionState(session("running"))).toMatchObject({ state: "working", needs: false, tone: "text-live" });
  });

  it("a paired Mac's row (a status alone) says it the same way", () => {
    for (const [status, word] of EXPECTED) expect(statusState(status).word, status).toBe(word);
    expect(statusState("something new").word).toBe("Idle");
  });

  it("the Project lens's row says it in words, in its name, and never in the old words", () => {
    for (const [status, word] of EXPECTED) {
      render(<RailRow session={session(status)} selected={false} onSelect={() => {}} density="comfortable" now={NOW} />);
      const row = screen.getByRole("button");
      expect(screen.getByTestId("rail-state-word").textContent, status).toBe(word);
      expect(row.textContent).toContain(`, ${word}, `);
      for (const old of OLD) expect(row.textContent, `${status}: ${old}`).not.toContain(old);
      cleanup();
    }
  });

  it("the icon column's summary and the legend use the same words", () => {
    expect(ariaSummary(session("waiting_user"), NOW)).toContain("Asks you");
    expect(ariaSummary(session("stuck"), NOW)).toContain("Probably stuck");
    render(<IndicatorLegend />);
    const states = screen.getByLabelText("States");
    expect([...states.querySelectorAll("dt")].map((dt) => dt.textContent)).toEqual(["Asks you", "Probably asks you", "Working", "Idle", "Done, review it", "Ended"]);
  });
});
