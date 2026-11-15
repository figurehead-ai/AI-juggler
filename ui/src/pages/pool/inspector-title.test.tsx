/**
 * AJ-E3012 §9: the inspector always leads with what is selected — its own
 * title, its kind and App as a subtitle, its status as a pill under the
 * title, never the heading — for a session, an App, an Account, a Tool, a
 * Mac, everyone in scope, a Watched session, a line, the lead's place and
 * the pool itself. One selection treatment: the ring and a raised
 * surface, never dashes; the selected card says so to a screen reader.
 */
import { cleanup, fireEvent, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { inspectorHead } from "./inspector-head";
import { nodeBorder } from "./NodeParts";
import { SELECTED } from "./selection";
import { openPage, resetCanvasTests, seedTeam } from "./canvas.test-kit";

beforeEach(resetCanvasTests);
afterEach(cleanup);

describe("the inspector's title, by what is selected", () => {
  it.each([
    [{ kind: "session", name: "Jive CLI", app: "Claude Code · Work", role: "member", state: "Working", level: 0 } as const, "Jive CLI", "Session · Claude Code · Work · Member", "Working"],
    [{ kind: "rule", label: "Claude Code CLI · Work Max", memberKind: "app", role: "reviewer", open: 1 } as const, "Claude Code CLI · Work Max", "App · Reviewer", "1 open now"],
    [{ kind: "rule", label: "Work Max", memberKind: "account", role: "member", open: 0 } as const, "Work Max", "Account · Member", "None open now"],
    [{ kind: "rule", label: "Codex", memberKind: "tool", role: "observer", open: 2 } as const, "Codex", "Tool · Observer", "2 open now"],
    [{ kind: "rule", label: "This Mac", memberKind: "mac", role: "member", open: 3 } as const, "This Mac", "Mac · Member", "3 open now"],
    [{ kind: "rule", label: "Everyone in web", memberKind: "everyone", role: "member", open: 4 } as const, "Everyone in web", "Everyone in scope · Member", "4 open now"],
    [{ kind: "watched", title: "Refactor the settings", app: "claude-code", role: "member" } as const, "Refactor the settings", "Session · claude-code · Watched · Member", "Can't reply here yet"],
    [{ kind: "edge", from: "Planner", to: "Tests", count: "3 messages this week", waits: false } as const, "Planner → Tests", "Line · what passes between these two", "3 messages this week"],
    [{ kind: "lead-next", app: "Claude Code · Work" } as const, "Claude Code · Work's next session", "Lead · it leads once it starts here", "Not started yet"],
    [{ kind: "pool", name: "web", own: "web", where: "web", sessions: 3, sealed: false, needs: 0 } as const, "web", "Pool · web's own pool: every session in web, as one team", "3 sessions"],
    [{ kind: "pool", name: "Checkout team", own: null, where: "web and docs", sessions: 1, sealed: true, needs: 0 } as const, "Checkout team", "Pool · in web and docs", "Sealed"],
  ])("%o", (of, title, subtitle, pill) => {
    const head = inspectorHead(of);
    expect(head.title).toBe(title);
    expect(head.subtitle).toBe(subtitle);
    expect(head.pill?.words).toBe(pill);
  });

  it("the pill carries the level: a need is amber (L2), a stop red (L3), the rest calm", () => {
    expect(inspectorHead({ kind: "session", name: "a", app: "b", role: "member", state: "Asks you", level: 2 }).pill?.level).toBe(2);
    expect(inspectorHead({ kind: "pool", name: "p", own: null, where: "web", sessions: 2, sealed: false, needs: 2 }).pill).toEqual({ words: "2 need you", level: 2 });
    expect(inspectorHead({ kind: "edge", from: "a", to: "b", count: "1 message this week", waits: true }).pill?.level).toBe(2);
  });
});

describe("on the canvas (AJ-E3012)", () => {
  it("selecting a member titles the inspector by its name, with the accent on the title's edge; the pool titles it otherwise", async () => {
    seedTeam();
    const view = await openPage();
    // Nothing selected: the pool, by its name, as a heading.
    expect(view.getAllByRole("heading", { name: "Team" }).map((one) => one.id)).toContain("pool-inspector-title");
    expect(view.getByTestId("inspector-subtitle").textContent).toMatch(/^Pool · /);
    const node = view.getAllByTestId("canvas-node")[0]!;
    const name = node.querySelector<HTMLButtonElement>("[data-node-name]")!;
    fireEvent.click(name);
    await waitFor(() => expect(view.getByTestId("inspector-title").textContent).toBe(name.textContent));
    expect(view.getByTestId("inspector-subtitle").textContent).toMatch(/^Session · .+ · Lead$/);
    expect(within(view.getByTestId("inspector-head")).getByTestId("inspector-pill")).toBeTruthy();
    expect(view.getByTestId("inspector-head").className).toContain("var(--color-accent)");
    // The card says it's the selected one, with the one ring.
    const selected = view.getAllByTestId("canvas-node")[0]!;
    expect(selected.getAttribute("aria-current")).toBe("true");
    expect(name.getAttribute("aria-pressed")).toBe("true");
    expect(selected.className).toContain("ring-2 ring-accent ring-offset-2");
    expect(selected.className).not.toContain("dashed");
    fireEvent.click(view.getByTestId("inspector-back"));
    await waitFor(() => expect(view.getByTestId("inspector-title").textContent).toBe("Team"));
  });

  it("one selection treatment: a solid accent ring with an offset and a raised surface, never dashes; waiting and leading aren't rings", () => {
    expect(nodeBorder(true, false, false)).toContain(SELECTED);
    expect(SELECTED).toBe("bg-selected shadow-card ring-2 ring-accent ring-offset-2 ring-offset-canvas");
    for (const [waits, lead] of [[true, false], [false, true], [false, false]] as const) {
      const rest = nodeBorder(false, waits, lead);
      expect(rest).not.toMatch(/ring|dashed|warning/);
    }
  });
});
