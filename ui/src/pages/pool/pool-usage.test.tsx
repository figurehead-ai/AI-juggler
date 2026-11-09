/**
 * AJ-E3009 R4 (D-62): what a pool's members used this week, a view only:
 * its own members' sessions, the most first, nothing to press, and nothing
 * ranked or woken by room (D-31).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as history from "../../lib/ipc/usageHistory";
import { PoolUsage } from "./PoolUsage";
import type { Page } from "../../lib/paged";
import type { UsageSessionInfo } from "../../types/generated/UsageSessionInfo";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const row = (session_id: string | null, io: number): UsageSessionInfo => ({ profile: "p", id: session_id ?? "none", session_id, tool: "claude-code", title: null, project_id: "p1", io, cache: 0, usd: null, first_ms: 0, last_ms: 0, messages: 1 });
const page = (items: UsageSessionInfo[]): Page<UsageSessionInfo> => ({ items, next_cursor: null, total: items.length });

describe("a pool's usage this week (AJ-E3009 R4)", () => {
  it("counts only its members' sessions, the most first, and offers nothing to press", async () => {
    const read = vi.spyOn(history, "usageHistorySessions").mockResolvedValue(page([row("a", 100), row("b", 300), row("stranger", 999), row(null, 50)]));
    render(<PoolUsage members={[{ id: "a", name: "Writer" }, { id: "b", name: "Reviewer" }]} project="p1" />);
    const usage = await screen.findByTestId("pool-usage");
    expect(read).toHaveBeenCalledWith(expect.objectContaining({ project: "p1", top: 0 }), expect.anything());
    const names = [...usage.querySelectorAll("li")].map((li) => li.textContent);
    expect(names[0]).toContain("Reviewer");
    expect(names[1]).toContain("Writer");
    expect(names).toHaveLength(2);
    expect(usage.textContent).toContain("in and out this week, by its members");
    expect(usage.textContent).toContain("A view only");
    expect(usage.querySelectorAll("button, a, [role=button]")).toHaveLength(0);
  });

  it("says so when usage can't be read, and reads nothing without members", async () => {
    vi.spyOn(history, "usageHistorySessions").mockRejectedValue(new Error("no"));
    render(<PoolUsage members={[{ id: "a", name: "Writer" }]} project={null} />);
    expect(await screen.findByText("Usage isn't readable now.")).toBeTruthy();
    cleanup();
    const read = vi.spyOn(history, "usageHistorySessions");
    read.mockClear();
    render(<PoolUsage members={[]} project={null} />);
    expect(await screen.findByTestId("pool-usage")).toBeTruthy();
    expect(read).not.toHaveBeenCalled();
  });
});
