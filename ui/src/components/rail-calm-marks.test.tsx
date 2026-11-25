/**
 * AJ-E1557 — the rail is calm: no bullet before a session (an idle or
 * ended row leads with nothing), a state mark only where there is a state
 * worth a glance (working, needs you, done to review, an error), every
 * title in the same place either way, and a subagents count marked by a
 * cluster of helpers, not a git branch (the 0.5.1 Nightly's two reads).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import { SessionRow } from "./row/SessionRow";
import { RowSubagents } from "./row/RowParts";
import { HeadCounts } from "./rail-more";
import { IconGitBranch, IconHelpers } from "./icons";
import { hasStateMark, itemLevel } from "../lib/attention";
import { sessionItem, type RailItem } from "../lib/rail-items";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const NOW = new Date(2026, 10, 25, 15, 0, 0).getTime();
afterEach(cleanup);

function item(id: string, over: Partial<RailItem> = {}): RailItem {
  const facts: Record<string, unknown> = {
    id, provider: "claude-code", account_id: "work", account_label: "Work Max", status: "running", needs_attention: null, attention: null, subtask_count: null,
    started_at_ms: NOW - 60_000, last_activity_ms: NOW - 30_000, machine_id: "this-mac", kind: "terminal", project_id: null, conversation_id: `c-${id}`, git_branch: null,
  };
  const row: InstanceInfo = JSON.parse(JSON.stringify(facts));
  return { ...sessionItem(row, () => null, false), ...over };
}

const CASES: Array<[string, Partial<RailItem>, boolean]> = [
  ["idle", { state: "idle", need: null }, false],
  ["ended", { state: "ended", need: null }, false],
  ["working", { state: "working", need: null }, true],
  ["needs you", { state: "asks_you", need: "asks" }, true],
  ["done to review", { state: "done_review", need: "review" }, true],
  ["an error", { state: "ended", need: "failed" }, true],
];

describe("AJ-E1557 R1 a state mark only where there is a state", () => {
  for (const [name, over, marked] of CASES) {
    for (const density of ["cozy", "compact"] as const) {
      it(`${name} (${density}): ${marked ? "its state's mark" : "no leading mark"}, the title where every row's is`, () => {
        const one = item(name.replace(/\s/g, "-"), over);
        expect(hasStateMark(one, itemLevel(one))).toBe(marked);
        render(<SessionRow item={one} density={density} now={NOW} onSelect={() => {}} within={{ app: true }} />);
        const gutter = screen.getByTestId("row-gutter");
        expect(gutter.querySelector("[data-state-dot]") !== null, `${name}: a mark`).toBe(marked);
        // The gutter stays, as wide as ever, with or without its mark:
        // no title moves when a state appears or goes.
        expect(gutter.className.split(" ")).toContain("w-4");
        expect(gutter.nextElementSibling?.contains(screen.getByTestId("row-title"))).toBe(true);
        // The state is never lost: its word ends the row and leads its name.
        expect(screen.getByTestId("state-word").textContent).not.toBe("");
      });
    }
  }
});

describe("AJ-E1557 R2 the subagents mark is a cluster of helpers", () => {
  const helpers = renderToStaticMarkup(<IconHelpers width={12} height={12} />);
  const branch = renderToStaticMarkup(<IconGitBranch width={12} height={12} />);
  const inner = (markup: string) => {
    const box = document.createElement("div");
    box.innerHTML = markup;
    return box.firstElementChild!.innerHTML;
  };

  it("a session row's count: the helpers glyph at 12 px, never the git branch", () => {
    const one = item("agents", { state: "working", subagents: 3 });
    render(<RowSubagents item={one} words="3 subagents running" onSelect={() => {}} />);
    const svg = screen.getByTestId("row-subagents").querySelector("svg")!;
    expect(svg.innerHTML).toBe(inner(helpers));
    expect(svg.innerHTML).not.toBe(inner(branch));
    expect(svg.getAttribute("width")).toBe("12");
  });

  it("an App header's subagents count: the same glyph", () => {
    const group = [item("w", { state: "working", subagents: 2 })];
    render(<HeadCounts items={group} needItems={[]} level={0} probably={false} selectedId={null} onSelect={() => {}} />);
    const svg = screen.getByTestId("place-count-subagents").querySelector("svg")!;
    expect(svg.innerHTML).toBe(inner(helpers));
    expect(svg.getAttribute("width")).toBe("12");
  });
});
