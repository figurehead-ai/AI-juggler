/**
 * AJ-E3713 vitest — Home's feed as posts, as a reader meets them: the
 * author's own picture, the story with its things as chips, threads under
 * a post, the gesture bar (More / Less like this kept through the leans'
 * store and read back by ranking, "Why am I seeing this?" with Undo,
 * Follow and Remind me made the Watches way, Ask Bot only when Bot is
 * ready), each filter, and the 0.5.1 Nightly's bug: the ChatGPT · Codex
 * row's bold title wrapped inside itself and centred mid-row.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeFeed } from "./HomeFeed";
import { feedItem } from "../preview/feed";
import { feedLeans, forgetFeedLeans } from "../lib/feed-leans";
import { forgetFeedState, keptFilters } from "../lib/feed-state";
import { forgetSharedReads } from "../lib/shared-read";
import { made, WATCHES_CATALOGUE } from "../preview/watches";
import type { FeedItemInfo } from "../lib/ipc/feed";

const NOW = Date.now();
const MIN = 60_000;
const CLAUDE = { kind: "app", name: null, provider: "claude-code", folder: "acc-work", folder_label: "work-max", desktop: true } as const;
const CHATGPT = { kind: "app", name: null, provider: "codex", folder: "acc-codex", folder_label: "Codex", desktop: true } as const;

const seed = (items: FeedItemInfo[]) => Reflect.set(window, "__AIJ_FEED__", items);
const sw = (id: string, at: number, project = "p-web") =>
  feedItem({ id, at_ms: at, kind: "switch", category: "switches", level: 1, coalesce: `w:${id}`, subject: `Switch ${id}`, facts: { project: "web-app", project_id: project, from_label: "Work Max", from_provider: "claude-code", to_label: "Personal", to_provider: "codex", mechanism: "copy", fidelity: "full" }, target: { project_id: project } });
const NO_FACTS_OF_DONE = { project: "web-app", project_id: "p-web" };
const done = (id: string, at: number, over: Omit<Partial<FeedItemInfo>, "facts"> & { facts?: Partial<FeedItemInfo["facts"]> } = {}) =>
  feedItem({ id, at_ms: at, kind: "finished", category: "finished", level: 1, coalesce: `s:${id}`, subject: `Session ${id}`, author: CLAUDE, facts: { project: "web-app", project_id: "p-web" }, target: { session_id: id, project_id: "p-web" }, ...over });

function feed(open = vi.fn(() => true)) {
  return render(
    <HomeFeed stacked={false} now={NOW} nameOf={() => null} quiet onNewSession={vi.fn()} onOpenApp={vi.fn()} toReview={0} onReviewAll={vi.fn()} scroller={() => null}
      facets={{ projects: [], accounts: [], subscriptions: [], pools: null }} actions={{ open, review: vi.fn(), railOf: () => null }} running={2} />,
  );
}

beforeEach(() => {
  forgetFeedState();
  forgetFeedLeans();
  forgetSharedReads();
  window.__AIJ_WATCH_VIEW__ = { watches: [], catalogue: { ...WATCHES_CATALOGUE }, made: (draft, id) => made(draft, id, { createdAgo: 0 }), calls: [] };
});
afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_FEED__");
  Reflect.deleteProperty(window, "__AIJ_WATCH_VIEW__");
  Reflect.deleteProperty(window, "__AIJ_SCHEDULES__");
});

describe("AJ-E3713 R6a — the diff stat on a post", () => {
  const changes = (turn: string, added: number | null, removed: number | null, names = ["src/calc.py", "tests/test_calc.py"]) => ({ turn, files: names.length, names, added, removed });

  it("draws the files, +added −removed and the bar, as the mock; names on hover", async () => {
    seed([done("a", NOW - 9 * MIN, { facts: { ...NO_FACTS_OF_DONE, changes: changes("t1", 412, 96) } })]);
    feed();
    const stat = await screen.findByTestId("feed-changes");
    expect(stat.textContent).toBe("2 files+412 lines added−96 lines removed");
    expect(stat.getAttribute("title")).toBe("src/calc.py, tests/test_calc.py");
    expect(stat.querySelector(".bg-success-strong")?.getAttribute("style")).toContain("width: 81%");
    expect(stat.querySelector(".bg-danger-strong")?.getAttribute("style")).toContain("width: 19%");
  });

  it("a tool whose calls don't count lines shows the files only, with their names", async () => {
    seed([done("a", NOW - 9 * MIN, { facts: { ...NO_FACTS_OF_DONE, changes: changes("t1", null, null, ["lab/data.ipynb"]) } })]);
    feed();
    const stat = await screen.findByTestId("feed-changes");
    expect(stat.dataset.filesOnly).toBe("yes");
    expect(stat.textContent).toBe("1 filelab/data.ipynb");
    expect(stat.textContent).not.toMatch(/[+−]/);
  });

  it("a rollup of finished sessions sums their turns", async () => {
    const rolled = (id: string, at: number, turn: string) => done(id, at, { rollup: "v:finished:acc-work:p-web:L1", rollup_window_ms: 7_200_000, facts: { ...NO_FACTS_OF_DONE, changes: changes(turn, 10, 5, [`src/${id}.ts`]) } });
    seed([rolled("a", NOW - 2 * MIN, "ta"), rolled("b", NOW - 3 * MIN, "tb"), rolled("c", NOW - 4 * MIN, "tc")]);
    feed();
    const card = (await screen.findAllByTestId("feed-card")).find((one) => one.dataset.rolled);
    expect(within(card!).getByTestId("feed-changes").textContent).toBe("3 files+30 lines added−15 lines removed");
  });
});

describe("AJ-E3713 R2 — posts", () => {
  it("an App's post wears its own icon (read at run time), AI Juggler's its mark; a calm line, a story with chips", async () => {
    seed([done("a", NOW - 9 * MIN), sw("w1", NOW - 18 * MIN)]);
    feed();
    const [app, own] = await screen.findAllByTestId("feed-card");
    expect(within(app!).getByTestId("app-icon")).toBeTruthy();
    expect(within(app!).getByText("Claude app").tagName).toBe("B");
    expect(app!.textContent).toContain("· work-max");
    expect(within(app!).getByTestId("feed-sentence").textContent).toBe("Claude app finished Session a in web-app.");
    expect(within(own!).getByTestId("post-mark")).toBeTruthy();
    expect(within(own!).getByText("AI Juggler")).toBeTruthy();
    expect(within(own!).getByTestId("feed-switch").textContent).toContain("→");
  });

  it("threads a session's follow-ups under its post instead of new rows", async () => {
    seed([
      done("a", NOW - 2 * MIN, { id: "fin" }),
      feedItem({ id: "ans", at_ms: NOW - 9 * MIN, kind: "answered", category: "needs", level: 0, coalesce: "s:a", subject: "Session a", facts: { need: "asks" }, target: { session_id: "a" } }),
    ]);
    feed();
    const cards = await screen.findAllByTestId("feed-card");
    expect(cards).toHaveLength(1);
    expect(within(cards[0]!).getByTestId("feed-sentence").textContent).toBe("You answered Session a — it's moving again.");
    expect(within(cards[0]!).getByTestId("feed-followup").textContent).toBe("↳ 7 min later · Session a finished.");
  });

  it("0.5.1 Nightly: the ChatGPT · Codex row's title is one left-aligned box that never wraps inside itself", async () => {
    const long = "Outline the launch plan for the spring release and the docs that go with it";
    seed([feedItem({ id: "c:1", at_ms: NOW - MIN, kind: "worked", category: "other", level: 0, subject: long, author: CHATGPT, target: { conversation_id: "conv-1", account_id: "acc-codex" } })]);
    feed();
    const chip = await screen.findByTestId("feed-subject");
    const classes = chip.className.split(/\s+/);
    for (const name of ["whitespace-nowrap", "text-left", "truncate", "inline-block"]) expect(classes).toContain(name);
    expect(chip.getAttribute("title")).toBe(long);
    // The author's bold name keeps its own box, beside its profile.
    const card = screen.getByTestId("feed-card");
    expect(within(card).getByText("ChatGPT").className).toContain("shrink-0");
  });
});

describe("AJ-E3713 R4 — a rollup's story names who did it", () => {
  it("\"Claude app finished 2 sessions in web-app\", never a sentence starting with its verb", async () => {
    seed([0, 1].map((n) => done(`r${n}`, NOW - (n + 1) * MIN, { rollup: "v:finished:acc-work:p-web:L1", rollup_window_ms: 7_200_000, scope: "f:acc-work" })));
    feed();
    const card = await screen.findByTestId("feed-card");
    expect(within(card).getByTestId("feed-sentence").textContent).toBe("Claude app finished 2 sessions in web-app");
  });
});

describe("AJ-E3713 R3 — More / Less like this", () => {
  it("Less like this is kept, says why with Undo, and the next posts like it fold", async () => {
    seed([sw("w1", NOW - MIN), done("a", NOW - 5 * MIN), sw("w2", NOW - 10 * MIN)]);
    feed();
    const first = (await screen.findAllByTestId("feed-card"))[0]!;
    await act(async () => {
      fireEvent.click(within(first).getByTestId("feed-less-like"));
    });
    // Kept with its scope: the kind of event, the App, the project.
    expect(await feedLeans()).toMatchObject([{ lean: "less", group: "switch", folder: null, project: "p-web", about: "web-app" }]);
    const why = await within(first).findByTestId("feed-why");
    expect(why.textContent).toContain("You'll see fewer switch posts. You chose Less like this for switch posts in web-app.");
    expect(within(first).getByTestId("feed-less-like").getAttribute("aria-pressed")).toBe("true");
    // The post leaned stays; the other switch post in web-app folds.
    expect(screen.getAllByTestId("feed-card").map((card) => card.getAttribute("data-feed-ids"))).toEqual(["w1", "a"]);
    expect(screen.getByTestId("feed-folded").textContent).toBe("1 post shown less · Show");
    fireEvent.click(screen.getByTestId("feed-folded"));
    expect(screen.getAllByTestId("feed-card")).toHaveLength(3);
    // Undo forgets it: nothing folds any more.
    await act(async () => {
      fireEvent.click(within(first).getByTestId("feed-why-undo"));
    });
    expect(await feedLeans()).toEqual([]);
    await waitFor(() => expect(screen.queryByTestId("feed-why")).toBeNull());
  });

  it("More like this lifts posts like it to the top of their day; its lit arrow opens the why", async () => {
    seed([sw("w1", NOW - MIN), done("a", NOW - 5 * MIN)]);
    feed();
    const later = (await screen.findAllByTestId("feed-card"))[1]!;
    await act(async () => {
      fireEvent.click(within(later).getByTestId("feed-more-like"));
    });
    expect(await within(later).findByTestId("feed-why")).toBeTruthy();
    fireEvent.click(within(later).getByTestId("feed-more-like"));
    expect(within(later).queryByTestId("feed-why")).toBeNull();
  });
});

describe("AJ-E3713 R3 — Follow, Remind me, Ask Bot, Open", () => {
  it("Follow makes a watch on the post's session for what it can report, then says Following", async () => {
    seed([done("a", NOW - MIN)]);
    feed();
    const button = await screen.findByTestId("feed-follow");
    await act(async () => {
      fireEvent.click(button);
    });
    const calls = window.__AIJ_WATCH_VIEW__?.calls ?? [];
    expect(calls).toHaveLength(1);
    expect(calls[0]![1]).toMatchObject({ targets: [{ kind: "session", id: "a" }], conditions: [{ code: "finishes" }, { code: "asks_you" }, { code: "fails" }, { code: "hits_limit" }], lifetime: { kind: "until_ends" } });
    await waitFor(() => expect(screen.getByTestId("feed-follow").textContent).toBe("Following"));
  });

  it("AJ-E2027: Remind me shows only once the scheduler answers (D-127), and sets a reminder about the post's thing at the time picked", async () => {
    seed([done("a", NOW - MIN)]);
    window.__AIJ_SCHEDULES__ = { off: true, calls: [] };
    const { unmount } = feed();
    await screen.findByTestId("feed-card");
    await act(async () => undefined);
    expect(screen.queryByTestId("feed-remind")).toBeNull();
    unmount();
    forgetSharedReads();
    window.__AIJ_SCHEDULES__ = { calls: [] };
    feed();
    fireEvent.click(await screen.findByTestId("feed-remind"));
    const before = Date.now();
    await act(async () => {
      fireEvent.click(screen.getAllByTestId("feed-remind-choice")[1]!);
    });
    const calls = window.__AIJ_SCHEDULES__.calls ?? [];
    expect(calls).toHaveLength(1);
    expect(calls[0]).toEqual(["schedule_remind", expect.objectContaining({ about: { kind: "session", id: "a" }, title: expect.stringMatching(/^Look at “.+”$/) })]);
    const request: unknown = calls[0]![1];
    const at = typeof request === "object" && request !== null && "at_ms" in request ? request.at_ms : null;
    expect(typeof at === "number" && at - before >= 60 * 60_000 - 1_000 && at - before <= 60 * 60_000 + 5_000).toBe(true);
    // No watch is made for it.
    expect(window.__AIJ_WATCH_VIEW__?.calls ?? []).toHaveLength(0);
  });

  it("Open opens the post's thing; Ask Bot is hidden while Bot's AI isn't ready (D-144)", async () => {
    seed([done("a", NOW - MIN)]);
    const open = vi.fn(() => true);
    feed(open);
    fireEvent.click(await screen.findByTestId("feed-open"));
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ id: "a" }));
    expect(screen.queryByTestId("feed-ask-bot")).toBeNull();
  });
});

describe("AJ-E3713 R5 — the filters", () => {
  const items = () => [
    done("a", NOW - MIN),
    feedItem({ id: "f", at_ms: NOW - 2 * MIN, kind: "failed", category: "needs", level: 3, coalesce: "s:f", subject: "Broke", author: CLAUDE, target: { session_id: "f" } }),
    feedItem({ id: "t", at_ms: NOW - 3 * MIN, kind: "task_done", category: "pools", level: 0, subject: "A task", facts: { pool: "Release", pool_id: "pool-1" }, target: { task_id: "t-1", pool_id: "pool-1" } }),
    feedItem({ id: "m", at_ms: NOW - 4 * MIN, kind: "message", category: "pools", level: 1, subject: "Thread", facts: { pool: "Release", pool_id: "pool-1", change: "handoff" }, target: { pool_id: "pool-1" } }),
    sw("w", NOW - 5 * MIN),
    feedItem({ id: "x", at_ms: NOW - 6 * MIN, kind: "this_mac", category: "mac", level: 1, subject: "", facts: { line: "This Mac is busy." }, target: { page: "this_mac" } }),
  ];
  const ids = () => screen.queryAllByTestId("feed-card").map((card) => card.getAttribute("data-feed-ids"));

  it("each tab keeps its posts", async () => {
    seed(items());
    feed();
    await waitFor(() => expect(ids()).toEqual(["a", "f", "t", "m", "w", "x"]));
    const tabs: [string, string[]][] = [["Needs you", ["f"]], ["Sessions", ["a", "f"]], ["Tasks", ["t"]], ["Pools", ["t", "m"]], ["Switches & limits", ["w"]], ["This Mac", ["x"]], ["All", ["a", "f", "t", "m", "w", "x"]]];
    for (const [tab, want] of tabs) {
      fireEvent.click(screen.getByRole("button", { name: tab }));
      await waitFor(() => expect(ids()).toEqual(want));
    }
  });

  it("each state chip keeps its posts, says its count today, and is remembered", async () => {
    seed(items());
    feed();
    await waitFor(() => expect(ids()).toHaveLength(6));
    expect(screen.getByTestId("feed-state-running").textContent).toBe("Running 2");
    expect(screen.getByTestId("feed-state-finished").textContent).toBe("Finished 2");
    expect(screen.getByTestId("feed-state-failed").textContent).toBe("Failed 1");
    fireEvent.click(screen.getByTestId("feed-state-failed"));
    await waitFor(() => expect(ids()).toEqual(["f"]));
    expect(keptFilters()).toMatchObject({ state: "failed" });
    fireEvent.click(screen.getByTestId("feed-state-finished"));
    await waitFor(() => expect(ids()).toEqual(["a", "t"]));
    fireEvent.click(screen.getByTestId("feed-state-finished"));
    await waitFor(() => expect(ids()).toHaveLength(6));
  });
});
