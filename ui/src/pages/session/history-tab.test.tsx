/**
 * wf09, AJ-E1533, AJ-E2108 Req.4 (the 2026-10-27 audit: the tab was
 * never set and never drawn): a session's History tab shows when it has
 * something recorded, lists its steps oldest first in words, links its
 * project's Switch Graph when it switched, and offers Replay once it
 * has ended.
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SessionHistoryTab } from "./HistoryTab";
import { useSessionHasHistory } from "./useSessionHasHistory";
import { historyWords } from "../../lib/session-history-words";
import { pageTabs } from "../registry";
import { forgetPaged } from "../../lib/paged";
import type { SessionEventInfo } from "../../lib/ipc/sessions";
import type { InstanceInfo } from "../../lib/bridge";

const EVENTS: SessionEventInfo[] = [
  { kind: "status", what: "starting", switch: null, at_ms: 1_000 },
  { kind: "status", what: "quota_blocked", switch: null, at_ms: 2_000 },
  { kind: "switch", what: "", switch: { direction: "out", other_account: "Personal", other_provider: "claude-code", state: "confirmed", fidelity: "F3" }, at_ms: 3_000 },
  { kind: "lifecycle", what: "archived", switch: null, at_ms: 4_000 },
];

type Seams = { __AIJ_SESSION_HISTORY__?: Record<string, SessionEventInfo[]> };
const seams = () => window as unknown as Seams;

afterEach(() => {
  cleanup();
  delete seams().__AIJ_SESSION_HISTORY__;
  forgetPaged();
  localStorage.clear();
});

const session = (over: Partial<InstanceInfo> = {}) =>
  ({ id: "s-1", provider: "claude-code", account_label: "Work", status: "finished", project_id: "proj-1", ...over }) as InstanceInfo;

describe("a session's history in words", () => {
  it("says what happened, never a status id", () => {
    const origin = { provider: "claude-code", account: "Work" };
    const words = EVENTS.map((event, index) => historyWords(event, index === 0, origin));
    expect(words[0]).toMatch(/^Started on .+ · Work$/);
    expect(words[1]).toBe("Hit its usage limit");
    expect(words[2]).toMatch(/^Switched to Personal \(/);
    expect(words[3]).toBe("Archived");
    expect(words.join(" ")).not.toMatch(/quota_blocked|claude-code|confirmed/);
  });
});

describe("the History tab", () => {
  it("is one of the session's tabs once it has history", () => {
    expect(pageTabs("session").map((tab) => tab.id)).not.toContain("history");
    expect(pageTabs("session", { hasHistory: true }).map((tab) => tab.id)).toContain("history");
    expect(pageTabs("session", { hasHistory: true, desktop: true }).map((tab) => tab.id)).toContain("history");
  });

  it("knows when there is history to show", async () => {
    seams().__AIJ_SESSION_HISTORY__ = { "s-1": EVENTS };
    function Probe({ id }: { id: string }) {
      return <span data-testid="has">{String(useSessionHasHistory(id))}</span>;
    }
    const view = render(<Probe id="s-1" />);
    await waitFor(() => expect(view.getByTestId("has").textContent).toBe("true"));
    cleanup();
    const none = render(<Probe id="s-none" />);
    await waitFor(() => expect(none.getByTestId("has").textContent).toBe("false"));
  });

  it("lists the steps oldest first, links the Switch Graph and offers Replay once ended", async () => {
    seams().__AIJ_SESSION_HISTORY__ = { "s-1": EVENTS };
    const onOpenProject = vi.fn();
    const view = render(<SessionHistoryTab session={session()} onOpenProject={onOpenProject} />);
    await waitFor(() => expect(view.getAllByTestId("session-history-row").length).toBe(4));
    const rows = view.getAllByTestId("session-history-row").map((row) => row.getAttribute("data-kind"));
    expect(rows).toEqual(["status", "status", "switch", "lifecycle"]);
    fireEvent.click(view.getByTestId("session-history-graph"));
    expect(onOpenProject).toHaveBeenCalledWith("proj-1");
    // Its project opens on Overview, where the Switch Graph is.
    expect(localStorage.getItem("aij.page.tab.project")).toBe("overview");
    expect(view.getByTestId("session-history-replay").textContent).toContain("Replay (read-only)");
  });

  it("a live session says Replay comes once it ends, and no graph link without a switch", async () => {
    seams().__AIJ_SESSION_HISTORY__ = { "s-1": [EVENTS[0]!] };
    const view = render(<SessionHistoryTab session={session({ status: "running" })} onOpenProject={() => {}} />);
    await waitFor(() => expect(view.getAllByTestId("session-history-row").length).toBe(1));
    expect(view.queryByTestId("session-history-replay")).toBeNull();
    expect(view.getByTestId("session-history-replay-later")).toBeTruthy();
    expect(view.queryByTestId("session-history-graph")).toBeNull();
  });
});
