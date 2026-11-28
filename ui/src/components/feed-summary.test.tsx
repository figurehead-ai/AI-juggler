/**
 * AJ-E3831 (D-154) vitest — a finished turn's AI summary on its feed
 * post: only while its Beta runs ("AI summaries in the feed"), only for a
 * turn the job covers; tagged AI with whose it is and Beta, the agent's
 * own words one tap away, "This is wrong" kept on this Mac; a refused
 * draft shows the agent's words in the line's place; off, nothing shows.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { HomeFeed } from "./HomeFeed";
import { feedItem } from "../preview/feed";
import { forgetFeedState } from "../lib/feed-state";
import { forgetFeedLeans } from "../lib/feed-leans";
import { forgetSharedReads } from "../lib/shared-read";
import type { AiJobInfo } from "../lib/ipc/ai";
import type { FeedItemInfo } from "../lib/ipc/feed";
import type { FeedLineInfo } from "../lib/ipc/feed-line";

const NOW = Date.now();
const CLI = { kind: "app", name: null, provider: "claude-code", folder: "acc-work", folder_label: "Work Max", desktop: false } as const;

const job = (over: Partial<AiJobInfo>): AiJobInfo => ({
  job: "feed", state: "ready", beta: true, beta_on: true, model_id: "apple-on-device", model_name: "Apple's on-device model", why: null, pick: null, covers: ["claude-code", "codex"], ...over,
});
const done = (id: string, over: Partial<FeedItemInfo> = {}) =>
  feedItem({ id, at_ms: NOW - 240_000, kind: "finished", category: "finished", level: 1, coalesce: `s:${id}`, subject: `Session ${id}`, author: CLI, facts: { project: "shop-api" }, target: { session_id: id }, ...over });
const SHOWN: FeedLineInfo = {
  state: "shown", turn: "t1", line: "Partly done: added the priority option · open: calendar sync · 6 tests pass", excerpt: "Two of the three are done, and the 6 tests pass.", model: "Apple's on-device model",
};

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
});
afterEach(() => {
  cleanup();
  for (const key of ["__AIJ_FEED__", "__AIJ_FEED_LINES__", "__AIJ_FEED_LINE_WRONG__", "__AIJ_AI_JOBS__"]) Reflect.deleteProperty(window, key);
});

describe("AJ-E3831 — the feed's AI summary on a finished turn's post", () => {
  it("shows the line tagged AI and Beta, the agent's words one tap away, and keeps 'This is wrong'", async () => {
    Reflect.set(window, "__AIJ_FEED__", [done("s1")]);
    window.__AIJ_AI_JOBS__ = [job({})];
    window.__AIJ_FEED_LINES__ = { s1: SHOWN };
    feed();
    expect((await screen.findByTestId("feed-summary-line")).textContent).toBe(SHOWN.line);
    const summary = screen.getByTestId("feed-summary");
    expect(summary.textContent).toContain("Summary by Apple's on-device model");
    expect(summary.textContent).toContain("Beta");
    expect(screen.queryByTestId("feed-summary-words")).toBeNull();
    fireEvent.click(screen.getByTestId("feed-summary-words-toggle"));
    expect(screen.getByTestId("feed-summary-words").textContent).toContain(SHOWN.excerpt!);
    fireEvent.click(screen.getByTestId("feed-summary-wrong"));
    await waitFor(() => expect(window.__AIJ_FEED_LINE_WRONG__).toEqual(["t1"]));
    expect(screen.getByTestId("feed-summary-wrong-kept")).toBeTruthy();
  });

  it("shows the agent's own words when every draft was refused", async () => {
    Reflect.set(window, "__AIJ_FEED__", [done("s1")]);
    window.__AIJ_AI_JOBS__ = [job({})];
    window.__AIJ_FEED_LINES__ = { s1: { ...SHOWN, state: "plain", line: null, model: null } };
    feed();
    expect((await screen.findByTestId("feed-summary-plain")).textContent).toContain(SHOWN.excerpt!);
    expect(screen.queryByTestId("feed-summary-line")).toBeNull();
  });

  it("shows nothing with its Beta off, or for a turn it doesn't cover", async () => {
    Reflect.set(window, "__AIJ_FEED__", [done("s1"), done("s2", { author: { ...CLI, provider: "kiro" } })]);
    window.__AIJ_AI_JOBS__ = [job({ state: "beta_off", beta_on: false })];
    window.__AIJ_FEED_LINES__ = { s1: SHOWN, s2: SHOWN };
    feed();
    await screen.findAllByTestId("feed-card");
    expect(screen.queryByTestId("feed-summary")).toBeNull();
    cleanup();
    forgetSharedReads();
    window.__AIJ_AI_JOBS__ = [job({})];
    Reflect.set(window, "__AIJ_FEED__", [done("s2", { author: { ...CLI, provider: "kiro" } })]);
    feed();
    await screen.findAllByTestId("feed-card");
    expect(screen.queryByTestId("feed-summary")).toBeNull();
    expect(screen.queryByTestId("feed-summary-probe")).toBeNull();
  });
});
