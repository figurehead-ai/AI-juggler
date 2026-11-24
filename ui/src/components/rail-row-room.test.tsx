/**
 * 0.5.1 F55 — a rail row's words in a short row: the state word ends on
 * "…" (truncate), never cut mid-letter, and a title keeps a readable
 * minimum: the "app" chip and compact's age give way by the row's own
 * width (a container query), not the title.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RailRow } from "./WorkflowRail";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const NOW = 1_728_800_000_000;

function desktop(status: InstanceInfo["status"]): InstanceInfo {
  return {
    id: "s1", launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "a1", account_label: "Work Max", model: "opus",
    launch_model: "opus", running_model: null, model_source: "launch", status, quota: null, tokens: null, cpu_pct: null, rss_bytes: null,
    child_count: null, subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: status === "failed" ? "failed" : null, attention: null,
    started_at_ms: NOW - 60_000, last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/w/app", kind: "desktop", isolation_label: null,
    project_id: null, conversation_id: null, title: "Rename the photo exports",
  };
}

afterEach(cleanup);

describe("a rail row in a short row (F55)", () => {
  it("the state word truncates with an ellipsis instead of being cut", () => {
    render(<RailRow session={desktop("failed")} selected={false} onSelect={() => {}} density="comfortable" now={NOW} trailing />);
    const word = screen.getByTestId("rail-state-word");
    expect(word.textContent).toBe("Stopped with an error");
    expect(word.className).toContain("truncate");
    expect(word.className).toContain("min-w-0");
    expect(word.className).not.toContain("shrink-0");
  });

  it("the words measure their own room, and the chip and the age give way first", () => {
    const { container } = render(<RailRow session={desktop("failed")} selected={false} onSelect={() => {}} density="compact" now={NOW} trailing />);
    expect(container.querySelector(".\\@container\\/row")).not.toBeNull();
    const chip = screen.getByTestId("rail-app-chip");
    expect(chip.className).toContain("@max-[11.5rem]/row:hidden");
    const age = container.querySelector<HTMLElement>(".ml-auto.tabular-nums");
    expect(age?.textContent).toMatch(/ago|now|\d/);
    expect(age?.className).toContain("@max-[8.75rem]/row:hidden");
    // The title never carries a give-way class: it is what stays.
    const title = screen.getByText("Rename the photo exports");
    expect(title.className).not.toMatch(/hidden/);
  });

  it("comfortable keeps the chip until the title's minimum plus the chip", () => {
    render(<RailRow session={desktop("running")} selected={false} onSelect={() => {}} density="comfortable" now={NOW} />);
    expect(screen.getByTestId("rail-app-chip").className).toContain("@max-[7.5rem]/row:hidden");
  });
});
