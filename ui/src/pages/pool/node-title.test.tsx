/**
 * D-126 §4a (the rc1 X2 lab: fifty nodes all read "web · started 5:16
 * P…"), the rc2 walk P5 (AJ-E3304 L4: an accessibility press on a session
 * node did nothing) and the rc2 walk's undrawn nodes after Diagram → List
 * → Diagram.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MOCK_SESSIONS } from "../../preview/fixtures";
import type { ContextCard } from "../../types/generated/ContextCard";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { startedWords } from "../../lib/session-titles";
import { CanvasNode } from "./CanvasNode";
import { nodeTitles } from "./node-title";
import type { MemberNode } from "./useCanvas";
import { openPage, resetCanvasTests, seedTeam } from "./canvas.test-kit";

const NOW = Date.UTC(2026, 10, 16, 17, 30);
const AT = NOW - 14 * 60_000;
const base = MOCK_SESSIONS[0]!;
const untitled = (id: string, over: Partial<InstanceInfo> = {}): InstanceInfo => ({ ...base, id, label: undefined, title: undefined, git_branch: null, cwd: "/Users/dev/web", started_at_ms: AT, ...over });
const card = (task = "None"): ContextCard => ({ session_id: "x", name: "x", place: "web · Work Max", tool: "Claude Code", account: "Work Max", state: "running", role: "member", task: { value: task, unavailable: null }, said: { value: null, unavailable: "Unavailable" }, last_activity_ms: null, files: [], files_unavailable: null });
const member = (session: InstanceInfo, task?: string): MemberNode => ({ id: session.id, session, card: card(task), role: "member", name: "web · started 5:16 PM" });

afterEach(cleanup);

describe("a node leads with its title or task, the time second (D-126 §4a)", () => {
  it("fifty untitled sessions in one folder, started the same minute, are told apart in a few characters", () => {
    const fifty = Array.from({ length: 50 }, (_, at) => member(untitled(`s-${String(at).padStart(2, "0")}`)));
    const titles = nodeTitles(fifty, NOW);
    const shown = fifty.map((one) => titles.get(one.id)!.title);
    expect(new Set(shown).size).toBe(50);
    expect(shown[0]).toBe("web (1)");
    expect(shown[49]).toBe("web (50)");
    // The time is second, never in front of the name.
    for (const one of fifty) expect(titles.get(one.id)!.when).toBe(startedWords(AT, NOW));
    expect(Math.max(...shown.map((words) => words.length))).toBeLessThanOrEqual(8);
  });

  it("its own name first, with no time when no one shares it; a task names an untitled one", () => {
    const named = member(untitled("a", { title: "Fix the login redirect" }));
    const tasked = member(untitled("b"), "Add the export button");
    const twins = [member(untitled("c", { label: "Review" })), member(untitled("d", { label: "Review", started_at_ms: AT + 60_000 }))];
    const titles = nodeTitles([named, tasked, ...twins], NOW);
    expect(titles.get("a")).toEqual({ title: "Fix the login redirect", when: null });
    expect(titles.get("b")).toEqual({ title: "Add the export button", when: startedWords(AT, NOW) });
    expect(titles.get("c")!.title).toBe("Review (1)");
    expect(titles.get("d")!.title).toBe("Review (2)");
    expect(titles.get("d")!.when).toBe(startedWords(AT + 60_000, NOW));
  });

  it("the full card shows the title, and the time on its state line", () => {
    const one = { ...member(untitled("s-12")), title: { title: "web (12)", when: "started 5:16 PM" } };
    render(<CanvasNode member={one} at={{ x: 0, y: 0 }} asks={[]} held={0} selected={false} tabStop dragging={false} scale={1} onSelect={() => {}} onMenu={() => {}} onPointerDown={() => {}} onKey={() => {}} />);
    const node = screen.getByTestId("canvas-node");
    expect(node.querySelector("[data-node-name]")!.textContent).toBe("web (12)");
    expect(screen.getByTestId("node-when").textContent).toBe("started 5:16 PM");
    expect(screen.getByTestId("tool-logo").getAttribute("data-kind")).toBe("cli");
  });
});

describe("a compact node at fit (the coordinator's critique: \"Checkout c…\" at 54 %)", () => {
  it("wraps its title over two lines before it cuts it", () => {
    const long = { ...member(untitled("s-9")), title: { title: "Review the checkout steps", when: null } };
    render(<CanvasNode member={long} at={{ x: 0, y: 0 }} asks={[]} held={0} selected={false} tabStop dragging={false} scale={0.54} onSelect={() => {}} onMenu={() => {}} onPointerDown={() => {}} onKey={() => {}} />);
    const text = screen.getByTestId("canvas-node").querySelector<HTMLElement>("[data-node-name-text]")!;
    expect(text.className.split(" ")).toContain("line-clamp-2");
    expect(text.textContent).toBe("Review the checkout steps");
  });
});

describe("every node is a real button (the rc2 walk P5, AJ-E3304 L4)", () => {
  const titled = { ...member(untitled("s-7")), role: "lead", title: { title: "web (7)", when: "started 5:16 PM" } };

  it.each([1, 0.6, 0.35])("at %s the node is a button by role and name, not a group around one", (scale) => {
    const select = vi.fn();
    render(<CanvasNode member={titled} at={{ x: 0, y: 0 }} asks={[]} held={0} selected={false} tabStop dragging={false} scale={scale} onSelect={select} onMenu={() => {}} onPointerDown={() => {}} onKey={() => {}} />);
    expect(screen.queryAllByRole("group")).toEqual([]);
    const node = screen.getByRole("button", { name: "web (7), started 5:16 PM" });
    expect(document.getElementById(node.getAttribute("aria-describedby")!)!.textContent).toMatch(/^Lead\. /);
    // An accessibility press (VoiceOver's, AXPress) is a click with no
    // pointer: it opens the node and moves to the inspector.
    fireEvent.click(node, { detail: 0 });
    expect(select).toHaveBeenCalledWith("s-7", true);
  });

  it("takes keyboard focus and Enter", async () => {
    const select = vi.fn();
    render(<CanvasNode member={titled} at={{ x: 0, y: 0 }} asks={[]} held={0} selected={false} tabStop dragging={false} scale={1} onSelect={select} onMenu={() => {}} onPointerDown={() => {}} onKey={() => {}} />);
    await userEvent.tab();
    const node = screen.getByRole("button", { name: "web (7), started 5:16 PM" });
    expect(document.activeElement).toBe(node);
    await userEvent.keyboard("{Enter}");
    expect(select).toHaveBeenCalledWith("s-7", true);
  });
});

describe("Diagram → List → Diagram draws the nodes at once (the rc2 walk)", () => {
  beforeEach(resetCanvasTests);

  it("its nodes are drawn again with no fade (WebKit holds a hidden window's animations at their first, invisible frame)", async () => {
    seedTeam();
    const view = await openPage();
    fireEvent.click(view.getByTestId("pool-mode-list"));
    await waitFor(() => expect(view.queryByTestId("pool-canvas")).toBeNull());
    fireEvent.click(view.getByTestId("pool-mode-canvas"));
    // Drawn in the same render, with no ResizeObserver or frame to wait for.
    const nodes = view.getAllByTestId("canvas-node");
    expect(nodes.length).toBeGreaterThan(0);
    for (const node of nodes) expect(node.outerHTML).not.toContain("aij-fade-in");
  });

  it("a node fades in only when it changes form (a zoom across 100 %)", () => {
    const one = member(untitled("s-1"));
    const props = { member: one, at: { x: 0, y: 0 }, asks: [], held: 0, selected: false, tabStop: true, dragging: false, onSelect: () => {}, onMenu: () => {}, onPointerDown: () => {}, onKey: () => {} };
    const view = render(<CanvasNode {...props} scale={1} />);
    expect(view.getByTestId("canvas-node").outerHTML).not.toContain("aij-fade-in");
    view.rerender(<CanvasNode {...props} scale={0.6} />);
    expect(view.getByTestId("canvas-node").outerHTML).toContain("aij-fade-in");
  });
});
