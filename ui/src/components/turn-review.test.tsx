/**
 * AJ-E3820 vitest — the turn review where a turn's end is shown: folded
 * under a finished turn's feed post (the agent's sentences quoted) and
 * under the session page's last turn (named, the reply being right
 * above); only for an App whose turn ends AI Juggler sees (the AI query's
 * `covers`), never for a desktop app's conversation, never empty; the
 * Beta's lines only once opened and only with the review job ready, each
 * tagged AI, with whose they are, the source and "This is wrong".
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { HomeFeed } from "./HomeFeed";
import { TurnReview } from "./TurnReview";
import { feedItem } from "../preview/feed";
import { CLEAN, FAILED, STREAMED, STREAMED_AI, UNCLEAR, review } from "../preview/turn-review";
import { forgetFeedState } from "../lib/feed-state";
import { forgetFeedLeans } from "../lib/feed-leans";
import { forgetSharedReads } from "../lib/shared-read";
import type { AiJobInfo } from "../lib/ipc/ai";
import type { FeedItemInfo } from "../lib/ipc/feed";

const NOW = Date.now();
const MIN = 60_000;
const CLI = { kind: "app", name: null, provider: "claude-code", folder: "acc-work", folder_label: "Work Max", desktop: false } as const;

const seedFeed = (items: FeedItemInfo[]) => Reflect.set(window, "__AIJ_FEED__", items);
const job = (over: Partial<AiJobInfo>): AiJobInfo => ({
  job: "review", state: "beta_off", beta: false, beta_on: false, model_id: null, model_name: null, why: null, pick: null, covers: ["claude-code", "codex"], ...over,
});

const done = (id: string, over: Omit<Partial<FeedItemInfo>, "facts"> & { facts?: Partial<FeedItemInfo["facts"]> } = {}) =>
  feedItem({ id, at_ms: NOW - 4 * MIN, kind: "finished", category: "finished", level: 1, coalesce: `s:${id}`, subject: `Session ${id}`, author: CLI, facts: { project: "shop-api" }, target: { session_id: id }, ...over });

function feed() {
  return render(
    <HomeFeed stacked={false} now={NOW} nameOf={() => null} quiet onNewSession={() => undefined} onOpenApp={() => undefined} toReview={0} onReviewAll={() => undefined} scroller={() => null}
      facets={{ projects: [], accounts: [], subscriptions: [], pools: null }} actions={{ open: () => true, review: () => undefined, railOf: () => null }} running={0} />,
  );
}

beforeEach(() => {
  forgetFeedState();
  forgetFeedLeans();
  forgetSharedReads();
  window.__AIJ_AI_JOBS__ = [job({})];
});
afterEach(() => {
  cleanup();
  for (const key of ["__AIJ_FEED__", "__AIJ_REVIEWS__", "__AIJ_REVIEWS_AI__", "__AIJ_REVIEW_WRONG__", "__AIJ_AI_JOBS__", "__AIJ_REVIEWS_AI_MS__"]) Reflect.deleteProperty(window, key);
});

describe("AJ-E3820 R3 — on a finished turn's feed post", () => {
  it("folds under the post with its files and risk, and opens in place with the agent's own sentences", async () => {
    seedFeed([done("s-stream")]);
    window.__AIJ_REVIEWS__ = { "s-stream": STREAMED };
    feed();
    const toggle = await screen.findByTestId("turn-review-toggle");
    // The post's count: the two changed and the reverted one (G25).
    expect(toggle.textContent).toBe("Turn review · 3 files · Medium risk");
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByTestId("turn-review-panel")).toBeNull();
    fireEvent.click(toggle);
    const panel = screen.getByTestId("turn-review-panel");
    expect(within(panel).getByTestId("turn-review-risk").textContent).toBe("RiskMedium · tests not run · asks you · part undone");
    expect(within(panel).getAllByTestId("turn-review-file").map((f) => f.textContent)).toEqual([
      "export/dunnock.py+40 lines added −25 lines removed",
      "export/writer.py+12 lines added −3 lines removed",
    ]);
    expect(within(panel).getByTestId("turn-review-reverted").textContent).toContain("export/quoting.py“I also changed the CSV quoting, then reverted that");
    expect(within(panel).getByTestId("turn-review-tests").textContent).toBe("TestsNot run“The integration suite is still unrun.”");
    expect(within(panel).getByTestId("turn-review-question").textContent).toBe("Asks you“Should I keep the old quoting behind a flag?”");
    expect(screen.queryByTestId("turn-review-ai"), "no AI lines: the Beta is off").toBeNull();
  });

  it("says a failure in red and its tests in the agent's words", async () => {
    seedFeed([done("s-parser", { kind: "failed", category: "needs", level: 3 })]);
    window.__AIJ_REVIEWS__ = { "s-parser": FAILED };
    feed();
    fireEvent.click(await screen.findByTestId("turn-review-toggle"));
    expect(screen.getByTestId("turn-review").getAttribute("data-risk")).toBe("high");
    expect(screen.getByTestId("turn-review-tests").textContent).toContain("Fail“Three tests still fail");
    expect(screen.getByTestId("turn-review-failures").textContent).toBe("What failed“The build also hit an error in the linker step.”");
  });

  it("is never offered where it can't be: a desktop app's turn, a tool it doesn't cover, a turn with no review or nothing in it", async () => {
    seedFeed([
      done("s-desktop", { author: { ...CLI, desktop: true } }),
      done("s-grok", { author: { ...CLI, provider: "grok-build" } }),
      done("s-none"),
      done("s-empty"),
      done("s-clean", { at_ms: NOW - 9 * MIN }),
    ]);
    window.__AIJ_REVIEWS__ = { "s-desktop": STREAMED, "s-grok": STREAMED, "s-empty": review({ turn: "t-empty" }), "s-clean": CLEAN };
    feed();
    await screen.findByTestId("turn-review-toggle");
    expect(screen.getAllByTestId("turn-review").length, "only the clean turn's").toBe(1);
    expect(screen.getByTestId("turn-review-toggle").textContent).toBe("Turn review · 1 file · Low risk");
  });

  it("isn't offered before the AI query says which tools it covers", async () => {
    window.__AIJ_AI_JOBS__ = [];
    seedFeed([done("s-stream")]);
    window.__AIJ_REVIEWS__ = { "s-stream": STREAMED };
    feed();
    await screen.findAllByTestId("feed-card");
    expect(screen.queryByTestId("turn-review")).toBeNull();
  });
});

describe("AJ-E3820 R2 — the Beta's lines", () => {
  it("come only once opened, with the review job ready: tagged AI, whose they are, the source and This is wrong", async () => {
    window.__AIJ_AI_JOBS__ = [job({ state: "ready", beta: true, beta_on: true, model_id: "apple-on-device", model_name: "Apple’s on-device model" })];
    seedFeed([done("s-stream")]);
    window.__AIJ_REVIEWS__ = { "s-stream": STREAMED };
    window.__AIJ_REVIEWS_AI__ = { "s-stream": STREAMED_AI };
    feed();
    const toggle = await screen.findByTestId("turn-review-toggle");
    expect(screen.getByTestId("turn-review").getAttribute("data-ai")).toBeNull();
    fireEvent.click(toggle);
    await waitFor(() => expect(screen.getByTestId("turn-review").getAttribute("data-ai")).toBe("yes"));
    expect(screen.getAllByTestId("turn-review-about").map((a) => a.textContent)).toEqual([
      "AIswitched the dunnock export to stream rows",
      "AIthe writer now takes rows one at a time",
    ]);
    const foot = screen.getByTestId("turn-review-ai");
    expect(foot.textContent).toContain("Lines by Apple’s on-device model");
    expect(foot.textContent).toContain("Beta");
    fireEvent.click(within(foot).getByTestId("turn-review-source-toggle"));
    expect(screen.getByTestId("turn-review-source").textContent).toContain("Switched the dunnock export to stream rows instead of building one big list");
    fireEvent.click(within(foot).getByTestId("turn-review-wrong"));
    expect(screen.getByTestId("turn-review-wrong-kept").textContent).toContain("Marked wrong, kept on");
    expect(window.__AIJ_REVIEW_WRONG__).toEqual(["t-stream"]);
    fireEvent.click(within(screen.getByTestId("turn-review-wrong-kept")).getByRole("button", { name: "Undo" }));
    expect(window.__AIJ_REVIEW_WRONG__).toEqual([]);
  });

  it("0.5.2 G28: the code-only review at once, a cue while the lines are written, and a refused draft never shows", async () => {
    window.__AIJ_AI_JOBS__ = [job({ state: "ready", beta: true, beta_on: true, model_id: "apple-on-device", model_name: "Apple’s on-device model" })];
    window.__AIJ_REVIEWS__ = { c1: STREAMED };
    window.__AIJ_REVIEWS_AI__ = { c1: STREAMED_AI };
    window.__AIJ_REVIEWS_AI_MS__ = 60;
    render(<TurnReview ask={{ account_id: "work", conversation_id: "c1", until_ms: NOW }} quote />);
    fireEvent.click(await screen.findByTestId("turn-review-toggle"));
    // The facts are there at once; the cue says the lines are coming.
    expect(screen.getByTestId("turn-review-changed")).toBeTruthy();
    expect(screen.getByTestId("turn-review-writing").textContent).toBe("AIApple’s on-device model is writing lines on this turn…");
    await waitFor(() => expect(screen.getAllByTestId("turn-review-about")).toHaveLength(2));
    expect(screen.queryByTestId("turn-review-writing")).toBeNull();
    cleanup();
    // Every draft refused: the backend answers the code-only review; the
    // cue goes and nothing else changes.
    window.__AIJ_REVIEWS_AI__ = { c1: STREAMED };
    render(<TurnReview ask={{ account_id: "work", conversation_id: "c1", until_ms: NOW }} quote />);
    fireEvent.click(await screen.findByTestId("turn-review-toggle"));
    expect(screen.getByTestId("turn-review-writing")).toBeTruthy();
    await waitFor(() => expect(screen.queryByTestId("turn-review-writing")).toBeNull());
    expect(screen.queryByTestId("turn-review-about")).toBeNull();
    expect(screen.queryByTestId("turn-review-ai")).toBeNull();
  });

  it("are never asked for while the Beta is off", async () => {
    seedFeed([done("s-stream")]);
    window.__AIJ_REVIEWS__ = { "s-stream": STREAMED };
    window.__AIJ_REVIEWS_AI__ = { "s-stream": STREAMED_AI };
    feed();
    fireEvent.click(await screen.findByTestId("turn-review-toggle"));
    await screen.findByTestId("turn-review-panel");
    expect(screen.queryByTestId("turn-review-about")).toBeNull();
    expect(screen.queryByTestId("turn-review-ai")).toBeNull();
  });
});

describe("AJ-E3820 R3 — under the session page's last turn", () => {
  it("names what the reply above says instead of quoting it again", async () => {
    window.__AIJ_REVIEWS__ = { c1: STREAMED };
    render(<TurnReview ask={{ account_id: "work", conversation_id: "c1", until_ms: NOW }} quote={false} />);
    fireEvent.click(await screen.findByTestId("turn-review-toggle"));
    expect(screen.getByTestId("turn-review-tests").textContent).toBe("TestsNot run, as it says");
    expect(screen.getByTestId("turn-review-question").textContent).toBe("Asks youYes · its question is above");
    expect(screen.getByTestId("turn-review-reverted").textContent).toBe("Revertedexport/quoting.py");
  });

  it("0.5.2 G25: a turn whose files were all undone still counts them, as its post does", async () => {
    window.__AIJ_REVIEWS__ = { c3: review({ turn: "t-undo", reverted: ["src/locale.ts"], reverted_words: "I reverted the change to locale.ts.", risk: "medium", risk_why: ["reverted"] }) };
    render(<TurnReview ask={{ account_id: "work", conversation_id: "c3", until_ms: NOW }} quote={false} />);
    expect((await screen.findByTestId("turn-review-toggle")).textContent).toBe("Turn review · 1 file · Medium risk");
  });

  it("says an undo that names no file as edited, files only for a tool that doesn't count lines", async () => {
    window.__AIJ_REVIEWS__ = { c2: UNCLEAR };
    render(<TurnReview ask={{ account_id: "work", conversation_id: "c2", until_ms: NOW }} quote={false} />);
    fireEvent.click(await screen.findByTestId("turn-review-toggle"));
    const changed = screen.getByTestId("turn-review-changed");
    expect(changed.textContent).toContain("Edited");
    expect(changed.textContent).toContain("It says it undid part of this, not which file.");
    expect(screen.getAllByTestId("turn-review-file").map((f) => f.textContent)).toEqual(["sync/retry.ts", "notebooks/backoff.ipynb"]);
  });
});
