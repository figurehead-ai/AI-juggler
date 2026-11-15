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
import { markSeen, seenSteps, setupRows } from "../../lib/pool-setup";

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
    expect(rows.map((row) => row.done)).toEqual([true, true, true, false, false, false]);
    // A pool on every project chose where only once it was seen.
    expect(setupRows({ ...facts, every: true }, new Set())[0]!.done).toBe(false);
    expect(setupRows({ ...facts, every: true }, new Set(["where"]))[0]!.done).toBe(true);
  });

  it("never vanishes when members are added; goes once every step is done", () => {
    const open = vi.fn();
    const { rerender } = render(<PoolChecklist rows={setupRows(facts, new Set())} onOpen={open} />);
    expect(screen.getByTestId("pool-checklist-count").textContent).toBe("1 of 6 done");
    // Members added: still there, Who's in ticked.
    rerender(<PoolChecklist rows={setupRows({ ...facts, whoIn: true }, new Set())} onOpen={open} />);
    expect(screen.getByTestId("pool-checklist")).toBeTruthy();
    expect(screen.getAllByTestId("pool-checklist-step").map((step) => step.dataset.done)).toEqual(["true", "true", "false", "false", "false", "false"]);
    // The next step is the current one; each opens its control.
    const lead = screen.getAllByTestId("pool-checklist-step")[2]!;
    expect(lead.getAttribute("aria-current")).toBe("step");
    fireEvent.click(lead);
    expect(open).toHaveBeenCalledWith("lead");
    rerender(<PoolChecklist rows={setupRows({ ...facts, whoIn: true, led: true }, new Set(["everyone", "talk", "own"]))} onOpen={open} />);
    expect(screen.queryByTestId("pool-checklist")).toBeNull();
  });

  it("shrinks to one line while something else is selected, which opens the next step", () => {
    const open = vi.fn();
    render(<PoolChecklist rows={setupRows({ ...facts, whoIn: true }, new Set())} onOpen={open} compact />);
    const line = screen.getByTestId("pool-checklist-line");
    expect(line.textContent).toContain("2 of 6 done · next: Who leads");
    fireEvent.click(line);
    expect(open).toHaveBeenCalledWith("lead");
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
