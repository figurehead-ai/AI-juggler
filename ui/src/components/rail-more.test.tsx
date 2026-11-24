/**
 * AJ-E1554 — each rail group says quietly what more there is, and each
 * count is one click from exactly what it counts: an App header's counts
 * (R2, R6), a session's subagents (R3, in place-board.test), a group's
 * footer (R4), folded Inactive apps' summary (R5); and a rail row's slots
 * hold still under the pointer (R1: nothing shown on hover takes room of
 * its own or covers the title).
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { GroupFooter, HeadCounts, RailFooter, idleSummary } from "./rail-more";
import { PlaceHeader } from "./PlaceHeader";
import { SessionRow } from "./row/SessionRow";
import { RailSignIns } from "./RailSignIns";
import { RailOfferRow } from "./PlaceOffers";
import { SectionHead } from "./PlaceBoardParts";
import { countWords, headCounts, idOf, nextOf } from "../lib/rail-counts";
import { presetListState, takeListPreset } from "../lib/list-presets";
import { rememberedTabId } from "../pages/registry";
import { sessionItem, type RailItem } from "../lib/rail-items";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const NOW = new Date(2026, 10, 23, 15, 0, 0).getTime();
afterEach(() => {
  cleanup();
  localStorage.clear();
});

function item(id: string, over: Partial<RailItem> = {}): RailItem {
  const facts: Record<string, unknown> = {
    id, provider: "claude-code", account_id: "work", account_label: "Work Max", status: "running", needs_attention: null, attention: null, subtask_count: null,
    started_at_ms: NOW - 60_000, last_activity_ms: NOW - 30_000, machine_id: "this-mac", kind: "terminal", project_id: null, conversation_id: `c-${id}`, git_branch: null,
  };
  // Only the facts an item reads (the rest of a session's are unread here).
  const row: InstanceInfo = JSON.parse(JSON.stringify(facts));
  return { ...sessionItem(row, () => null, false), ...over };
}

let working: RailItem, working2: RailItem, asks: RailItem, review: RailItem, idle: RailItem, group: RailItem[];
beforeAll(() => {
  // After the test setup loads the tools' facts (an item's place names its tool).
  working = item("w1", { state: "working", subagents: 4 });
  working2 = item("w2", { state: "working" });
  asks = item("a1", { state: "asks_you", need: "asks" });
  review = item("r1", { state: "done_review" });
  idle = item("i1", { state: "idle", subagents: 2 });
  group = [working, working2, asks, review, idle];
});

describe("AJ-E1554 R2/R6 an App header's counts", () => {
  it("counts from the rows it holds, non-zero only, in order: working, needs you, to review, subagents", () => {
    const counts = headCounts(group, [asks]);
    expect(counts.map((count) => [count.kind, count.n])).toEqual([["working", 2], ["needs", 1], ["review", 1], ["subagents", 6]]);
    // Each count is the number of what it opens (agents: the sessions that run them).
    for (const count of counts) expect(count.kind === "subagents" ? count.items.reduce((sum, one) => sum + one.subagents, 0) : count.items.length).toBe(count.n);
    // Zero values are hidden: nothing working, nothing counted.
    expect(headCounts([idle].map((one) => ({ ...one, subagents: 0 })), [])).toEqual([]);
    // What needs you is counted once, as needing you, never also as working.
    expect(headCounts([{ ...asks, state: "working" }], [asks]).map((count) => count.kind)).toEqual(["needs"]);
  });

  it("each is a button, named for what it counts, that opens the next of exactly those, by id", () => {
    const onSelect = vi.fn();
    const { rerender } = render(<HeadCounts items={group} needItems={[asks]} level={2} probably={false} selectedId={null} onSelect={onSelect} />);
    const work = screen.getByTestId("place-count-working");
    expect(work.getAttribute("aria-label")).toBe("2 working here: each click opens the next");
    expect(work.getAttribute("title")).toBe(work.getAttribute("aria-label"));
    expect(work.className).toContain("tabular-nums");
    // PAT-COLOR: quiet, a tone on hover, never a line.
    expect(work.className).toContain("text-muted");
    expect(work.className).not.toMatch(/\bborder\b/);
    fireEvent.click(work);
    expect(onSelect).toHaveBeenLastCalledWith("w1");
    rerender(<HeadCounts items={group} needItems={[asks]} level={2} probably={false} selectedId="w1" onSelect={onSelect} />);
    fireEvent.click(screen.getByTestId("place-count-working"));
    expect(onSelect).toHaveBeenLastCalledWith("w2");
    // Needs you: amber (its level's mark), and its one session.
    expect(screen.getByTestId("place-needs").getAttribute("data-level")).toBe("2");
    fireEvent.click(screen.getByTestId("place-count-needs"));
    expect(onSelect).toHaveBeenLastCalledWith("a1");
    fireEvent.click(screen.getByTestId("place-count-review"));
    expect(onSelect).toHaveBeenLastCalledWith("r1");
    // Subagents: a branch and the number; it opens the session on its Subagents tab.
    expect(screen.getByTestId("place-count-subagents").textContent).toBe("6");
    expect(screen.getByTestId("place-count-subagents").querySelector("svg")).not.toBeNull();
    fireEvent.click(screen.getByTestId("place-count-subagents"));
    // w1 is open (selected above): the next session that runs some.
    expect(onSelect).toHaveBeenLastCalledWith("i1");
    expect(rememberedTabId("session")).toBe("subagents");
  });

  it("the next is the one after what's open, then round to the first", () => {
    expect(nextOf([working, working2], null)).toBe(working);
    expect(nextOf([working, working2], "w1")).toBe(working2);
    expect(nextOf([working, working2], "w2")).toBe(working);
    expect(nextOf([], "w1")).toBeNull();
    expect(idOf(working)).toBe("w1");
    expect(countWords({ kind: "subagents", items: [working, idle], n: 6 })).toBe("6 subagents running in 2 sessions: opens each one's Subagents tab");
  });

  it("the header draws its counts in place of the bare count", () => {
    render(<PlaceHeader place={null} label="Work" count={5} needs={1} folded={null} onToggle={() => {}} cluster={<HeadCounts items={group} needItems={[asks]} level={2} probably={false} selectedId={null} onSelect={() => {}} />} />);
    expect(screen.queryByTitle("5 open here")).toBeNull();
    expect(screen.getByTestId("place-counts")).toBeTruthy();
    // Outside its page button: a count is its own way in.
    expect(screen.getByTestId("place-page").contains(screen.getByTestId("place-counts"))).toBe(false);
  });
});

describe("AJ-E1554 R4 a group's footer", () => {
  it("says what more there is: \"+12 more · 3 recent\", each part its way in", () => {
    const onMore = vi.fn(), onRecent = vi.fn();
    render(<GroupFooter label="Work" hidden={12} words="more" total={17} recent={{ count: 3, open: false }} onMore={onMore} onRecent={onRecent} />);
    expect(screen.getByTestId("rail-group-footer").textContent).toBe("+12 more·3 recent");
    const more = screen.getByTestId("rail-more-quiet");
    // It agrees with the list it opens: its page's Open now lists all 17.
    expect(more.getAttribute("title")).toBe("Opens Work's page on Open now: all 17 open");
    fireEvent.click(more);
    expect(onMore).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByTestId("place-recent"));
    expect(onRecent).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("place-recent").getAttribute("aria-expanded")).toBe("false");
  });

  it("a group with nothing more has no footer; recent alone reads \"+3 recent\"", () => {
    const { container } = render(<GroupFooter label="Work" hidden={0} words="more" total={2} recent={null} onMore={() => {}} onRecent={() => {}} />);
    expect(container.textContent).toBe("");
    cleanup();
    render(<GroupFooter label="Work" hidden={0} words="more" total={2} recent={{ count: 3, open: false }} onMore={() => {}} onRecent={() => {}} />);
    expect(screen.getByTestId("place-recent").textContent).toBe("+3 recent");
    expect(screen.queryByTestId("rail-more-quiet")).toBeNull();
  });

  it("\"+n more\" opens its App's page on Sessions, its list on Open now: the list it counts", () => {
    const toggle = vi.fn();
    const line = { kind: "more" as const, group: { key: "app|claude-app|Work", label: "Work", place: null, accountId: null, poolId: null, items: group }, quiet: 3, words: "more" as const, recent: null };
    const opened: string[] = [];
    const on = (event: Event) => {
      if (event instanceof CustomEvent) opened.push(JSON.stringify(event.detail));
    };
    window.addEventListener("aij:open-app", on);
    try {
      render(<RailFooter line={line} app="app-work" toggle={toggle} />);
      fireEvent.click(screen.getByTestId("rail-more-quiet"));
    } finally {
      window.removeEventListener("aij:open-app", on);
    }
    expect(opened).toEqual([JSON.stringify({ id: "app-work" })]);
    expect(rememberedTabId("app")).toBe("sessions");
    expect(takeListPreset("app-sessions:app-work")).toBe("open");
    expect(toggle).not.toHaveBeenCalled();
    // No page of its own (another lens): in place, exactly those.
    cleanup();
    render(<RailFooter line={line} app={null} toggle={toggle} />);
    fireEvent.click(screen.getByTestId("rail-more-quiet"));
    expect(toggle).toHaveBeenCalledWith("app|claude-app|Work#all");
  });

  it("its page's list opens on the state the count was (once)", () => {
    presetListState("app-sessions:work", "open");
    expect(takeListPreset("app-sessions:other")).toBeNull();
    expect(takeListPreset("app-sessions:work")).toBe("open");
    expect(takeListPreset("app-sessions:work")).toBeNull();
  });
});

describe("AJ-E1554 R5 folded Inactive apps", () => {
  it("say what they hold", () => {
    expect(idleSummary(10, 48)).toBe("10 apps · 48 sessions");
    expect(idleSummary(1, 1)).toBe("1 app · 1 session");
    expect(idleSummary(2, 0)).toBe("2 apps");
  });

  it("hold one line: the label gives way, never the summary or a count (0.5.1 F22)", () => {
    render(<SectionHead id="idle" count={5} open={false} onToggle={() => {}} summary={idleSummary(5, 16)} />);
    const words = screen.getByTestId("section-label-words");
    expect(words.className.split(" ")).toEqual(expect.arrayContaining(["min-w-0", "truncate"]));
    expect(words.getAttribute("title")).toBe(words.textContent);
    const summary = screen.getByTestId("section-summary");
    expect(summary.textContent).toBe("5 apps · 16 sessions");
    expect(summary.className.split(" ")).toEqual(expect.arrayContaining(["shrink-0", "whitespace-nowrap"]));
    expect(summary.className).not.toMatch(/\btruncate\b/);
    cleanup();
    render(<SectionHead id="needs" count={3} level={2} open onToggle={() => {}} />);
    expect(screen.getByTestId("section-count").className.split(" ")).toEqual(expect.arrayContaining(["shrink-0", "whitespace-nowrap"]));
  });
});

describe("AJ-E1554 R1 a rail row's slots hold still under the pointer", () => {
  const swaps = /\b(hidden|w-0|w-auto|group-hover:(flex|block|w-\S+|hidden))\b/;
  it("a session row: its dot and grip share the gutter; its state's word and tools share its end; hover changes opacity only", () => {
    const arrange = { group: "g", enabled: true, grip: () => {}, drop: null, dragging: false };
    render(<SessionRow item={{ ...working, subagents: 0 }} density="cozy" now={NOW} onSelect={() => {}} within={{ app: true }} tools arrange={arrange} />);
    const gutter = screen.getByTestId("row-gutter"), end = screen.getByTestId("row-end");
    expect(gutter.querySelector("[data-state-dot]")).not.toBeNull();
    expect(gutter.querySelector("[data-testid='order-grip']")).not.toBeNull();
    expect(end.querySelector("[data-testid='state-word']")).not.toBeNull();
    expect(end.querySelector("[data-testid='row-tools']")).not.toBeNull();
    // One grid cell per slot: both layers always drawn, the slot as wide at rest as on hover.
    for (const slot of [gutter, end]) {
      expect(slot.className).toContain("[&>*]:col-start-1");
      expect(slot.className).toContain("[&>*]:row-start-1");
      for (const layer of slot.children) expect(layer.className).not.toMatch(swaps);
    }
    // The title is apart from both: nothing drawn over it.
    const title = screen.getByTestId("row-title");
    expect(gutter.contains(title) || end.contains(title)).toBe(false);
    // The ⋯ is in its end's tools now, never over the dot.
    expect(end.querySelector("[aria-haspopup='menu']")).not.toBeNull();
    // Hover changes nothing in the DOM (only CSS opacity under :hover).
    const before = document.body.innerHTML;
    fireEvent.mouseEnter(title.closest("[data-rail-row]")!);
    expect(document.body.innerHTML).toBe(before);
  });

  it("an App header: its grip over its icon, its ↗ in a slot of its own, both by opacity", () => {
    const arrange = { group: "g", enabled: true, grip: () => {}, drop: null, dragging: false };
    render(<PlaceHeader place={null} label="Work" count={2} needs={0} folded={null} onToggle={() => {}} onOpen={() => {}} arrange={arrange} />);
    expect(screen.getByTestId("place-grip").className).toContain("absolute");
    expect(screen.getByTestId("place-open").className).not.toMatch(swaps);
    expect(screen.getByTestId("place-open").className).toContain("group-hover/head:opacity-100");
    expect(screen.getByTestId("place-open").className).not.toMatch(/\bborder\b/);
  });

  it("a sign-in row and a running, not added one: fixed slots, nothing on hover, no bright line", () => {
    render(<RailSignIns terminals={[{ id: "t1", title: "Sign in to Work", ended: false }]} selectedId="t1" onSelect={() => {}} />);
    const signIn = screen.getByTestId("rail-sign-in");
    expect(signIn.className).not.toMatch(/group-hover:|\bborder\b/);
    // Selected by a fill, never the accent bar it had.
    expect(signIn.querySelector(".bg-accent")).toBeNull();
    cleanup();
    render(<RailOfferRow offer={{ id: "app|claude-app|Lab", label: "Lab", running: true }} onAdd={() => {}} />);
    const add = screen.getByTestId("rail-offer-add");
    expect(add.className).not.toMatch(/\bborder\b/);
    expect(add.className).toContain("hover:bg-control-hi");
  });
});
