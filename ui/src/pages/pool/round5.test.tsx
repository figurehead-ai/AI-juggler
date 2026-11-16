/**
 * AJ-E3011 round 5 (docs/qa/evidence/AJ-E3011/walks/after5-run1.md, with
 * main's correction under D-55): no jump on Add, a project's own pool
 * without an "Everyone" card, the cost line, the inspector wider and
 * resizable, and an opened section's control focused.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { keepInPlace, keep } from "../../lib/in-place";
import { resetPending } from "../../lib/pool-pending";
import { scopeOf, type PoolInfo } from "../../lib/ipc/pools";
import { TEAM_TOOLS } from "../../lib/team-tools";
import { Splitter, edgeWidthForKey, readEdgeWidth } from "../../components/Splitter";
import { INSPECTOR_EDGE } from "./Inspector";
import { Palette } from "./Palette";
import { PoolSetupSections } from "./PoolSetupSections";
import { drawnRules } from "./usePoolSetup";
import { JoinAs } from "./setup/JoinAs";

beforeEach(() => {
  resetPending();
  localStorage.removeItem(INSPECTOR_EDGE.store);
});
afterEach(cleanup);

describe("AJ-E3011 round 5", () => {
  it("#2: an added row stays where it was, marked Added, so the next Add doesn't move", () => {
    const [first, second, third] = MOCK_SESSIONS;
    const props = { poolId: "p", projects: MOCK_PROJECTS, removing: null, open: true, onOpen: () => undefined, onStartDrag: () => undefined, onAdd: () => undefined };
    const view = render(<Palette {...props} sessions={[first!, second!, third!]} />);
    const before = screen.getAllByTestId("canvas-palette-item").map((row) => row.getAttribute("data-session"));
    fireEvent.click(screen.getAllByTestId("canvas-palette-add")[0]!);
    // The pool read back: the first is a member now, gone from the candidates.
    view.rerender(<Palette {...props} sessions={[second!, third!]} />);
    const after = screen.getAllByTestId("canvas-palette-item");
    expect(after.map((row) => row.getAttribute("data-session"))).toEqual(before);
    expect(after[0]!.querySelector('[data-testid="canvas-palette-added"]')?.textContent).toBe("Added");
    expect(after[1]!.querySelector('[data-testid="canvas-palette-add"]')).not.toBeNull();
    // Kept rows go back to their places, in order.
    const kept = keep(keep(new Map(), "a", "a", 0), "c", "c", 2);
    expect(keepInPlace(["b", "d"], kept, (row) => row).map((one) => `${one.row}${one.added ? "+" : ""}`)).toEqual(["a+", "b", "c+", "d"]);
  });

  it("#3: a project's own pool draws no Everyone card; its join role is set in Everyone else", () => {
    const rules = [{ member: "all" }, { member: "app:cli:work" }];
    expect(drawnRules({ kind: "default" }, rules)).toEqual([{ member: "app:cli:work" }]);
    expect(drawnRules({ kind: "user" }, rules)).toEqual(rules);
    render(<JoinAs where="web" value="member" onChange={() => undefined} />);
    expect(screen.getByTestId("join-as").textContent).toContain("New sessions in web join as:");
  });

  it("#4: the cost line", () => {
    expect(TEAM_TOOLS.cost).toBe("It costs nothing extra. Your assistants use their own turns only when they act.");
  });

  it("D-55: the inspector is 380 px at first, 360 to 420, from its left edge, and keeps its width", () => {
    expect(readEdgeWidth(INSPECTOR_EDGE)).toBe(380);
    expect(edgeWidthForKey(INSPECTOR_EDGE, 380, "ArrowLeft", false)).toBe(388);
    expect(edgeWidthForKey(INSPECTOR_EDGE, 380, "ArrowRight", true)).toBe(360);
    expect(edgeWidthForKey(INSPECTOR_EDGE, 410, "ArrowLeft", true)).toBe(420);
    function Host() {
      const [width, setWidth] = useState(readEdgeWidth(INSPECTOR_EDGE));
      return <Splitter edge={INSPECTOR_EDGE} width={width} onWidth={setWidth} controls="pool-inspector" />;
    }
    render(<Host />);
    const edge = screen.getByRole("separator", { name: "Resize the inspector" });
    fireEvent.keyDown(edge, { key: "End" });
    expect(edge.getAttribute("aria-valuenow")).toBe("420");
    fireEvent(window, new Event("pagehide"));
    expect(localStorage.getItem(INSPECTOR_EDGE.store)).toBe("420");
    expect(readEdgeWidth(INSPECTOR_EDGE)).toBe(420);
  });

  it("D-55: a section opened from the checklist or a Change has its control focused", () => {
    const pool: PoolInfo = { id: "pool-web", name: "web team", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0, ...scopeOf("user", null) };
    const rules = { rules: { pool_id: pool.id, members: [{ member: "all", kind: "everyone", role: "member", level: null, joined_at: 1, sessions: 0, session_ids: [] }], talk: [], lead_next: null }, watched: [], reload: () => undefined };
    render(
      <div data-canvas-scroll>
        <PoolSetupSections pool={pool} rules={rules} members={[]} ruleLabel={() => ""} where="web" selection={{ kind: "pool", focus: "everyone" }} onAddMembers={() => undefined} />
      </div>,
    );
    expect(document.activeElement).toBe(screen.getByTestId("pool-setup-everyone").querySelector('[data-testid="pool-join-picker"], button'));
    expect(screen.getByTestId("pool-setup-everyone").contains(document.activeElement)).toBe(true);
  });
});
