/**
 * AJ-E3713 R2, R3, R4, R5, R8 — the story feed's rules, without a screen:
 * the template stories (no AI, still a story; never "Resolved: …"), the
 * story seam for D-149's feed job and its fallback, threads (follow-ups of
 * one session or task under one post), ranking after a "less" (and never
 * burying something urgent), and each tab's and state chip's rule.
 */
import { afterEach, describe, expect, it } from "vitest";
import { feedItem } from "../preview/feed";
import { cardsOf, threadOf } from "./feed-rollup";
import { leanFor, rank, whyWords, type FeedLeanInfo } from "./feed-leans";
import { groupOf, inState, inTab, TABS } from "./feed-chips";
import { attachmentOf, followUp, postAuthor, setStorySource, story, storyText, templateStory } from "./feed-story";
import type { FeedItemInfo } from "./ipc/feed";

const NOW = new Date(2026, 10, 26, 15, 0).getTime();
const MIN = 60_000;
const CLAUDE = { kind: "app", name: null, provider: "claude-code", folder: "acc-work", folder_label: "work-max", desktop: true } as const;

const finished = (over: Partial<FeedItemInfo> = {}) =>
  feedItem({ id: "l:1", at_ms: NOW - 9 * MIN, kind: "finished", category: "finished", level: 1, coalesce: "s:a", subject: "Release checks", author: CLAUDE, facts: { project: "web-app", project_id: "p-web", duration_ms: 42 * MIN }, target: { session_id: "a", project_id: "p-web" }, ...over });
const switched = (id: string, at: number, project = "p-web") =>
  feedItem({ id, at_ms: at, kind: "switch", category: "switches", level: 1, subject: "Port the parser", facts: { project_id: project, from_label: "Work Max", from_provider: "claude-code", to_label: "Personal", to_provider: "codex", mechanism: "copy", fidelity: "full" }, target: { project_id: project } });

afterEach(() => setStorySource(null));

describe("AJ-E3713 R4 — the template stories", () => {
  it("tells who did what from the facts, the thing a chip", () => {
    const parts = story(finished());
    expect(storyText(parts)).toBe("Claude app finished Release checks in web-app in 42 min.");
    expect(parts.filter((part) => part.chip === "subject").map((part) => part.text)).toEqual(["Release checks"]);
  });

  it("says an answered need as what came of it, never \"Resolved: …\"", () => {
    const answered = feedItem({ id: "n:1", at_ms: NOW, kind: "answered", category: "needs", subject: "Cart endpoints", facts: { need: "asks" } });
    expect(storyText(story(answered))).toBe("You answered Cart endpoints — it's moving again.");
    const nameless = { ...answered, subject: "", facts: { ...answered.facts, need: "heavy", line: "This Mac is busy" } };
    expect(storyText(story(nameless))).toBe("Something that needed you is settled: This Mac is busy.");
    for (const kind of ["answered", "finished", "failed", "switch", "quota", "message", "task_done", "watch", "this_mac"]) {
      expect(storyText(templateStory({ ...answered, kind }))).not.toMatch(/^Resolved:/);
    }
  });

  it("never says a switch moved the conversation (it is copied, F9); it carries on in the new App", () => {
    const text = storyText(story(switched("w:1", NOW)));
    expect(text).toBe("Port the parser carries on in Codex CLI · Personal: AI Juggler copied it over from Claude Code CLI · Work Max. It took the whole conversation along.");
    expect(text).not.toMatch(/\bmoved\b/);
    expect(attachmentOf(switched("w:1", NOW))).toMatchObject({ kind: "switch", from: { label: "Claude Code CLI · Work Max" }, to: { label: "Codex CLI · Personal" } });
  });

  it("tells a pool's work from the pool, and quotes the agent's own words", () => {
    const message = feedItem({ id: "m:1", at_ms: NOW, kind: "message", category: "pools", subject: "Totals in cents", author: { kind: "agent", name: "Builder", provider: "codex", folder: null, folder_label: null, desktop: false }, facts: { pool: "Checkout redesign", pool_id: "pool-1", change: "question", quote: "Do totals use integer cents?" } });
    const parts = story(message);
    expect(storyText(parts)).toBe("In the Checkout redesign pool, Builder asked a question in Totals in cents.");
    expect(parts.find((part) => part.chip === "pool")?.text).toBe("Checkout redesign");
    expect(attachmentOf(message)).toEqual({ kind: "quote", text: "Do totals use integer cents?", who: "Builder's words" });
    expect(postAuthor(message)).toEqual({ kind: "juggler", name: "AI Juggler", place: null });
  });

  it("says an Account's room as measured, its meter the attachment", () => {
    const quota = feedItem({ id: "n:2", at_ms: NOW, kind: "quota", category: "switches", subject: "Work Max", author: { kind: "account", name: "Work Max", provider: null, folder: "acc-work", folder_label: null, desktop: false }, facts: { window: "5-hour", meter: 39, resets_at_ms: null, sessions: 2 } });
    expect(storyText(story(quota))).toBe("Work Max has 39% of its 5-hour window left. Right now 2 sessions use it.");
    expect(attachmentOf(quota)).toEqual({ kind: "usage", left: 39, window: "5-hour", resetsAt: null });
  });

  it("names the post's author by the App's own word and profile", () => {
    expect(postAuthor(finished())).toEqual({ kind: "app", name: "Claude app", place: "work-max" });
  });
});

describe("AJ-E3713 R4 — D-149's story seam, and the fallback", () => {
  it("uses the job's story when it names the same thing; else, or with no AI, the template's", () => {
    const item = finished();
    expect(storyText(story(item))).toBe(storyText(templateStory(item)));
    setStorySource(() => [{ text: "work-max wrapped up " }, { text: "Release checks", chip: "subject" }, { text: " with every check green." }]);
    expect(storyText(story(item))).toBe("work-max wrapped up Release checks with every check green.");
    // A story that doesn't name the item's thing is dropped (facts only).
    setStorySource(() => [{ text: "Something else happened." }]);
    expect(storyText(story(item))).toBe(storyText(templateStory(item)));
    // The job declines (its verdict didn't pass): the template.
    setStorySource(() => null);
    expect(storyText(story(item))).toBe(storyText(templateStory(item)));
  });
});

describe("AJ-E3713 R2 — threads", () => {
  it("groups a session's follow-ups under its first post, one event told twice once", () => {
    const items = [
      feedItem({ id: "n:fin", at_ms: NOW - 2 * MIN, kind: "finished", category: "finished", level: 1, coalesce: "s:a", subject: "Login refactor", facts: { duration_ms: 20 * MIN } }),
      feedItem({ id: "l:fin", at_ms: NOW - 2 * MIN - 1_000, kind: "finished", category: "finished", level: 1, coalesce: "s:a", subject: "Login refactor" }),
      feedItem({ id: "n:ans", at_ms: NOW - 9 * MIN, kind: "answered", category: "needs", level: 0, coalesce: "s:a", subject: "Login refactor", facts: { need: "asks" } }),
      feedItem({ id: "n:other", at_ms: NOW - 5 * MIN, kind: "finished", category: "finished", level: 1, coalesce: "s:b", subject: "Cart endpoints" }),
    ];
    const cards = cardsOf(items);
    expect(cards).toHaveLength(2);
    const thread = threadOf(cards[0]!.things[0]!);
    expect(thread.post.id).toBe("n:ans");
    expect(thread.follow.map((item) => item.kind)).toEqual(["finished"]);
    expect(storyText(followUp(thread.follow[0]!))).toBe("Login refactor finished.");
  });

  it("threads a task's changes too", () => {
    const task = (id: string, at: number, kind: string) => feedItem({ id, at_ms: at, kind, category: "pools", level: 1, subject: "Wire the export", facts: { pool: "Release", pool_id: "pool-1" }, target: { task_id: "t-1", pool_id: "pool-1" } });
    const cards = cardsOf([task("t:2", NOW, "task_done"), task("t:1", NOW - 30 * MIN, "task_review")]);
    expect(cards).toHaveLength(1);
    expect(threadOf(cards[0]!.things[0]!).follow.map((item) => item.kind)).toEqual(["task_done"]);
  });
});

describe("AJ-E3713 R3 — ranking reads the leans", () => {
  const less: FeedLeanInfo = { key: "feed:switch:x", lean: "less", group: "switch", folder: null, project: "p-web", about: "web-app", at_ms: NOW };
  const more: FeedLeanInfo = { key: "feed:finish:y", lean: "more", group: "finish", folder: "acc-work", project: null, about: "work-max", at_ms: NOW };

  it("after a \"less\", the quiet posts it names fold out of their day; a \"more\" rises first", () => {
    const cards = cardsOf([switched("w:2", NOW - MIN), finished({ id: "l:9", at_ms: NOW - 30 * MIN, coalesce: "s:z", rollup: null }), switched("w:3", NOW - 40 * MIN, "p-other")]);
    const before = rank(cards, []);
    expect(before[0]!.cards.map((card) => card.head.id)).toEqual(["w:2", "l:9", "w:3"]);
    const after = rank(cards, [less, more]);
    expect(after[0]!.cards.map((card) => card.head.id)).toEqual(["l:9", "w:3"]);
    expect(after[0]!.folded.map((card) => card.head.id)).toEqual(["w:2"]);
    // A post just leaned stays where it is, with its Undo.
    const pinned = rank(cards, [less], new Set([cards[0]!.key]));
    expect(pinned[0]!.cards.map((card) => card.head.id)).toEqual(["w:2", "l:9", "w:3"]);
  });

  it("reads the lean of the post a thread shows (its first item), as its gesture bar leans", () => {
    const thread = cardsOf([
      feedItem({ id: "fin", at_ms: NOW - MIN, kind: "finished", category: "finished", level: 1, coalesce: "s:a", subject: "A" }),
      feedItem({ id: "ans", at_ms: NOW - 5 * MIN, kind: "answered", category: "needs", level: 0, coalesce: "s:a", subject: "A", facts: { need: "asks" } }),
    ]);
    const lessNeeds: FeedLeanInfo = { key: "feed:need:x", lean: "less", group: "need", folder: null, project: null, about: null, at_ms: NOW };
    expect(rank(thread, [lessNeeds])[0]!.folded).toHaveLength(1);
    expect(rank(thread, [{ ...lessNeeds, group: "finish" }])[0]!.folded).toHaveLength(0);
  });

  it("never folds or moves something that broke or needs the reader", () => {
    const failed = { ...switched("w:4", NOW - MIN), kind: "switch_failed", level: 2 };
    const ranked = rank(cardsOf([failed]), [{ ...less, group: groupOf("switch_failed") }]);
    expect(ranked[0]!.cards).toHaveLength(1);
    expect(ranked[0]!.folded).toHaveLength(0);
  });

  it("the most specific lean is the one behind a post, and says why", () => {
    const wide: FeedLeanInfo = { ...less, key: "feed:switch:all", project: null, lean: "more", about: null };
    expect(leanFor(switched("w:5", NOW), [wide, less])?.key).toBe(less.key);
    expect(whyWords(less)).toEqual({ head: "You'll see fewer switch posts.", why: "You chose Less like this for switch posts in web-app." });
  });
});

describe("AJ-E3713 R5 — the tabs and state chips", () => {
  const kinds = ["finished", "failed", "answered", "worked", "task_done", "task_blocked", "message", "switch", "quota", "this_mac"];
  const under = (tab: string) => kinds.filter((kind) => inTab({ kind, category: "other" }, tab));

  it("each tab keeps what it says (the read model's rules)", () => {
    expect(TABS.map((tab) => tab.label)).toEqual(["All", "Needs you", "Sessions", "Tasks", "Pools", "Switches & limits", "This Mac"]);
    expect(under("all")).toEqual(kinds);
    expect(kinds.filter((kind) => inTab({ kind, category: kind === "failed" || kind === "answered" ? "needs" : "x" }, "needs"))).toEqual(["failed", "answered"]);
    expect(under("sessions")).toEqual(["finished", "failed", "answered", "worked"]);
    expect(under("tasks")).toEqual(["task_done", "task_blocked"]);
    expect(under("pools")).toEqual(["task_done", "task_blocked", "message"]);
    expect(under("switches")).toEqual(["switch", "quota"]);
    expect(under("mac")).toEqual(["this_mac"]);
  });

  it("each state chip keeps what it says", () => {
    expect(kinds.filter((kind) => inState(kind, "finished"))).toEqual(["finished", "task_done"]);
    expect(kinds.filter((kind) => inState(kind, "failed"))).toEqual(["failed", "task_blocked"]);
  });
});
