/**
 * AJ-E3008 R1 (D-62 decision 18), AJ-E3011 §1: the empty canvas says the
 * first move in one calm card; the setup's order is the inspector's
 * checklist, which ticks each step from the pool and the steps seen, never
 * vanishes when members are added, and goes only when every step is done.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { CanvasTeach, hasWhoIsIn } from "./CanvasTeach";
import { PoolChecklist } from "./setup/PoolChecklist";
import { markSeen, seenSteps, setupReady, setupRows } from "../../lib/pool-setup";

afterEach(cleanup);
beforeEach(() => localStorage.clear());

const primaries = () => screen.getAllByRole("button").filter((button) => button.className.includes("bg-accent") || button.dataset.variant === "primary");

describe("the empty canvas (AJ-E3008 R1, AJ-E3011)", () => {
  it("says the first move, one primary, and offers the guided setup for a pool of yours", () => {
    const calls = { add: vi.fn(), setup: vi.fn() };
    render(<CanvasTeach scope="payments-api" own={false} onAdd={calls.add} onSetup={calls.setup} inset={false} />);
    expect(screen.getByTestId("canvas-empty").textContent).toContain("No one in this pool yet");
    expect(screen.getByTestId("canvas-empty").textContent).toContain("payments-api");
    const add = screen.getByTestId("canvas-teach-add");
    expect(primaries()).toEqual([add]);
    fireEvent.click(add);
    expect(calls.add).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByTestId("canvas-teach-template"));
    expect(calls.setup).toHaveBeenCalledTimes(1);
  });

  it("a project's own pool has no guided setup (its scope is its project)", () => {
    render(<CanvasTeach scope="web" own onAdd={() => undefined} onSetup={() => undefined} inset={false} />);
    expect(screen.queryByTestId("canvas-teach-template")).toBeNull();
  });

  it("walk K1: a pool of 500 sessions, none drawn, has who's in it", () => {
    // A project's pool: its sessions join through its project, no member of their own.
    expect(hasWhoIsIn(500, false)).toBe(true);
    expect(hasWhoIsIn(0, true)).toBe(true); // an App, Account, Tool or Mac brings them as they start
    expect(hasWhoIsIn(0, false)).toBe(false);
  });
});

describe("the setup checklist (AJ-E3011 §1)", () => {
  const facts = { own: false, every: false, whoIn: false, led: false, leadNext: false };

  it("keeps the setup's order and ticks where, who's in and who leads from the pool", () => {
    const rows = setupRows({ ...facts, whoIn: true, leadNext: true }, new Set());
    expect(rows.map((row) => row.step)).toEqual(["where", "who", "lead", "everyone", "talk", "own"]);
    // Round 4 (#2): a step left at its default counts as done.
    expect(rows.map((row) => row.done)).toEqual([true, true, true, true, true, true]);
    expect(rows.map((row) => row.byDefault)).toEqual([false, false, false, true, true, true]);
    // A pool on every project: where is its default until it's seen.
    expect(setupRows({ ...facts, every: true }, new Set())[0]).toMatchObject({ done: true, byDefault: true });
    expect(setupRows({ ...facts, every: true }, new Set(["where"]))[0]).toMatchObject({ done: true, byDefault: false });
  });

  it("counts defaults as done (round 4 #2); ready once who's in is set and the team tools are on, with what happens next", () => {
    const open = vi.fn();
    const { rerender } = render(<PoolChecklist rows={setupRows(facts, new Set())} onOpen={open} />);
    // Only who's in has no default.
    expect(screen.getByTestId("pool-checklist-count").textContent).toBe("5 of 6 done");
    expect(screen.getAllByTestId("pool-checklist-default")).toHaveLength(4);
    const who = screen.getAllByTestId("pool-checklist-step")[1]!;
    expect(who.getAttribute("aria-current")).toBe("step");
    fireEvent.click(who);
    expect(open).toHaveBeenCalledWith("who");
    // Members added, the team tools still off: all ticked, and it says what's left.
    const rows = setupRows({ ...facts, whoIn: true }, new Set());
    rerender(<PoolChecklist rows={rows} onOpen={open} ready={setupReady(rows, false)} />);
    expect(screen.getByTestId("pool-checklist-count").textContent).toBe("6 of 6 done");
    expect(screen.getByTestId("pool-checklist-tools-left").textContent).toContain("Ready once the team tools are on");
    // The team tools on: ready, and the next line.
    rerender(<PoolChecklist rows={rows} onOpen={open} ready={setupReady(rows, true)} next="Open Claude Code · Work in web: it leads; their messages show here" />);
    expect(screen.queryByTestId("pool-checklist")).toBeNull();
    expect(screen.getByTestId("pool-ready").textContent).toContain("This pool is ready");
    expect(screen.getByTestId("pool-ready-next").textContent).toBe("What happens next: Open Claude Code · Work in web: it leads; their messages show here.");
  });

  it("shrinks to one line while something else is selected, which opens the next step", () => {
    const open = vi.fn();
    render(<PoolChecklist rows={setupRows(facts, new Set())} onOpen={open} compact />);
    const line = screen.getByTestId("pool-checklist-line");
    expect(line.textContent).toContain("5 of 6 done · next: Who's in");
    fireEvent.click(line);
    expect(open).toHaveBeenCalledWith("who");
  });

  it("remembers the steps seen per pool on this Mac", () => {
    expect(seenSteps("pool-a").size).toBe(0);
    markSeen("pool-a", ["talk", "own"]);
    expect([...seenSteps("pool-a")].sort()).toEqual(["own", "talk"]);
    expect(seenSteps("pool-b").size).toBe(0);
    localStorage.setItem("aij.pool-setup.pool-c", "not json");
    expect(seenSteps("pool-c").size).toBe(0);
  });
});
