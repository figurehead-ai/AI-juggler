/**
 * 0.3.2 — the maintainer's 0.3.0 walk: on Processes the app row
 * ("Claude") and the session row of a session's own tab could not be
 * folded. Every row with something under it folds on the click, at once,
 * without waiting for the backend; a session's choice is also handed to
 * the view so the next read keeps it.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProcessTree, unfolded } from "./ProcessTree";
import type { ProcessGraphRowInfo } from "../../lib/ipc/processes";

afterEach(cleanup);

function row(key: string, kind: string, depth: number, has_children: boolean): ProcessGraphRowInfo {
  return {
    key,
    kind,
    depth,
    parent_key: null,
    has_children,
    state: "working",
    origin: "ours",
    owner_words: "",
    cpu_pct: 1,
    rss_bytes: 1_000_000,
    ports: [],
    app: kind === "app" ? ({ kind: "tool", name: "Claude", tool: "claude-code" } as never) : null,
    process:
      kind === "process"
        ? { pid: depth * 100, start_time: 1, name: `proc-${key}`, role: "tool", role_words: "a tool", role_sure: true, label: null, subagent_id: null, subagent_title: null, spark: [], peak_rss: 1, age_ms: 1, cpu_time_ms: 1, waits_for_pid: null, waits_for_name: null, stop: "ask", stop_reason: null, mark: null, mark_until_ms: null, heavy: false, failed: null, internal: false }
        : null,
    rollup: { processes: 2, rss_bytes: 1_000_000, cpu_pct: 1, ports: [], attention: 0, to_clean: 0, worst: "working" },
    session_key: kind === "app" ? null : "s:1",
    session: kind === "session" ? ({ session_id: "1", origin: "ours", tool: "claude-code", account_id: null, project_id: null, title: "api", folder_name: "api", state: "working", alive: true, root_pid: 200, folded: false, shared_by: 0, started_ms: null } as never) : null,
  } as ProcessGraphRowInfo;
}

const ROWS = [row("a:claude", "app", 0, true), row("s:1", "session", 1, true), row("p:200", "process", 2, true), row("p:300", "process", 3, false)];

function tree(onToggle = vi.fn()) {
  render(<ProcessTree rows={ROWS} total={ROWS.length} label="Processes" isOpen={(r) => !r.session?.folded} onToggle={onToggle} actions={() => null} hasMore={false} onNearEnd={() => {}} />);
  return onToggle;
}

const keys = () => screen.queryAllByTestId(/^process-row-/).map((el) => el.getAttribute("data-testid")!.replace("process-row-", ""));
const fold = (key: string) => fireEvent.click(within(screen.getByTestId(`process-row-${key}`)).getByRole("button", { name: /^(Fold|Unfold)$/ }));

describe("0.3.2 folding on Processes", () => {
  it("unfolded() hides everything deeper under a folded row, of any kind", () => {
    expect(unfolded(ROWS, new Set(["a:claude"])).map((r) => r.key)).toEqual(["a:claude"]);
    expect(unfolded(ROWS, new Set(["s:1"])).map((r) => r.key)).toEqual(["a:claude", "s:1"]);
    expect(unfolded(ROWS, new Set(["p:200"])).map((r) => r.key)).toEqual(["a:claude", "s:1", "p:200"]);
  });

  it("the app row folds and unfolds on the click", () => {
    tree();
    expect(keys()).toEqual(["a:claude", "s:1", "p:200", "p:300"]);
    fold("a:claude");
    expect(keys()).toEqual(["a:claude"]);
    fold("a:claude");
    expect(keys()).toEqual(["a:claude", "s:1", "p:200", "p:300"]);
  });

  it("the session row folds at once and tells the view, even where the backend sends it open", () => {
    const onToggle = tree();
    fold("s:1");
    expect(keys()).toEqual(["a:claude", "s:1"]);
    expect(onToggle).toHaveBeenCalledWith(expect.objectContaining({ key: "s:1" }), false);
    fold("s:1");
    expect(keys()).toEqual(["a:claude", "s:1", "p:200", "p:300"]);
    expect(onToggle).toHaveBeenLastCalledWith(expect.objectContaining({ key: "s:1" }), true);
  });
});
