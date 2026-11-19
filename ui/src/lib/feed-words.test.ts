/**
 * D-118 (AJ-E3702 R10) — every sentence of the feed, one fixture per item
 * type the read model makes from today's journals (`events.md` §1–6, F1):
 * subject first, what changed, the result; no emoji, no exclamation mark,
 * no first person; times as R10 says them.
 */
import { describe, expect, it } from "vitest";
import { awayWords } from "../components/HomeFeedAway";
import { authorWords, dayLabel, digestWords, durationWords, exactWhen, factsLine, feedWhen, placeWords, rollupSentence, said, sentence } from "./feed-words";
import { feedItem } from "../preview/feed";
import type { FeedItemInfo } from "./ipc/feed";

const NOW = new Date(2026, 10, 14, 10, 0, 0).getTime();
const app = { kind: "app", name: null, provider: "claude-code", folder: "claude-work", folder_label: "Work Max", desktop: false } as const;
const item = (over: Partial<FeedItemInfo> & Pick<FeedItemInfo, "kind">, facts: Partial<FeedItemInfo["facts"]> = {}): FeedItemInfo =>
  feedItem({ id: `x:${over.kind}`, at_ms: NOW - 60_000, category: "other", subject: "Fix the flaky login test", author: app, ...over, facts: { ...feedItem({ id: "", at_ms: 0, kind: "", category: "" }).facts, ...facts } });

/** One per F1 item type, with the sentence it must read. */
const CASES: [FeedItemInfo, string][] = [
  [item({ kind: "finished" }, { duration_ms: 42 * 60_000 }), "finished Fix the flaky login test in 42 min"],
  [item({ kind: "failed" }, { detail: "exit code 1", duration_ms: 42 * 60_000 }), "Fix the flaky login test stopped with an error (exit code 1) after 42 min"],
  [item({ kind: "crashed" }, { detail: "signal 9" }), "Fix the flaky login test crashed (signal 9)"],
  [item({ kind: "ended" }), "Fix the flaky login test ended when AI Juggler quit"],
  [item({ kind: "restarted", author: { ...app, kind: "juggler" } }, { attempt: 1, tries: 3 }), "restarted Fix the flaky login test after it crashed (try 1 of 3)"],
  [item({ kind: "stopped", author: { ...app, kind: "you" } }), "stopped Fix the flaky login test"],
  [item({ kind: "reviewed", author: { ...app, kind: "you" } }), "marked Fix the flaky login test reviewed"],
  [item({ kind: "archived", author: { ...app, kind: "you" } }), "archived Fix the flaky login test"],
  [item({ kind: "answered" }, { need: "asks" }), "Fix the flaky login test got its answer"],
  [item({ kind: "answered" }, { need: "stuck" }), "Fix the flaky login test moved on"],
  [item({ kind: "answered" }, { need: "heavy" }), "Fix the flaky login test no longer needs you"],
  [item({ kind: "answered", subject: "" }, { need: "signin", line: "Work Max needs its sign-in" }), "Resolved: Work Max needs its sign-in"],
  [item({ kind: "switch", subject: "Answer the release question" }, { from_provider: "claude-code", from_label: "Work Max", to_provider: "codex", to_label: "Personal" }), "moved Answer the release question from Claude Code CLI · Work Max to Codex CLI · Personal"],
  [item({ kind: "switch_failed", subject: "Answer the release question" }, { to_provider: "codex", to_label: "Personal" }), "couldn't move Answer the release question to Codex CLI · Personal. The original is untouched."],
  [item({ kind: "quota", author: { ...app, kind: "account", name: "Work Max" } }, { meter: 12, window: "5-hour", sessions: 2 }), "is at 12% of its 5-hour window. 2 sessions use it."],
  [item({ kind: "credits", author: { ...app, kind: "account", name: "Work Max" } }, { window: "5-hour", credits: "$4.10 left" }), "used its 5-hour window. Credits carry on ($4.10 left)."],
  [item({ kind: "reset", author: { ...app, kind: "account", name: "Personal" } }, { window: "5-hour", sessions: 1 }), "has room again: its 5-hour window reset. 1 session can switch back."],
  [item({ kind: "renewal" }, { line: "Personal renews in 5 days" }), "Personal renews in 5 days"],
  [item({ kind: "this_mac" }, { line: "This Mac was under memory pressure for 12 min." }), "This Mac was under memory pressure for 12 min."],
  [item({ kind: "app_started" }, { line: "AI Juggler started Claude app · Personal again" }), "AI Juggler started Claude app · Personal again"],
  [item({ kind: "keep_alive_paused" }, { line: "Keep alive paused for Claude app · Lab" }), "Keep alive paused for Claude app · Lab"],
  [item({ kind: "update" }, { line: "AI Juggler 0.5.1 is ready" }), "AI Juggler 0.5.1 is ready"],
  [item({ kind: "found" }, { line: "Cursor is running but isn't added" }), "Cursor is running but isn't added"],
  [item({ kind: "task_done", subject: "Wire the export button" }, { change: "In progress → Done" }), "finished the task Wire the export button"],
  [item({ kind: "task_review", subject: "Export flow" }), "Export flow is waiting for review"],
  [item({ kind: "task_blocked", subject: "Export flow" }), "Export flow is blocked"],
  [item({ kind: "message", subject: "Export flow" }, { change: "review_request" }), "asked for a review in Export flow"],
  [item({ kind: "message", subject: "Export flow" }, { change: "handoff" }), "handed work over in Export flow"],
  [item({ kind: "message", subject: "Export flow" }, { change: "question" }), "asked a question in Export flow"],
  [item({ kind: "message", subject: "Export flow" }, { change: "status" }), "posted where it is in Export flow"],
  [item({ kind: "worked", subject: "Plan the trip notes app" }), "worked on Plan the trip notes app"],
  // AJ-E3817: a watch the user set, in the app's words from the typed watch.
  [item({ kind: "watch", subject: "api" }, { told: "finished" }), "api finished · your watch"],
];

describe("D-118 R10 — the feed's sentences", () => {
  it.each(CASES.map(([one, words]) => [one.kind, one, words] as const))("%s", (_kind, one, words) => {
    expect(said(sentence(one))).toBe(words);
  });

  it("speaks in the voice: no emoji, no exclamation mark, no first person, the object's name its strong part", () => {
    for (const [one] of CASES) {
      const words = said(sentence(one));
      expect(words, one.kind).not.toMatch(/\p{Extended_Pictographic}/u);
      expect(words, one.kind).not.toContain("!");
      expect(words, one.kind).not.toMatch(/\b(I|I'm|we|We|my|our)\b/);
      const name = sentence(one).find((part) => part.strong);
      if (one.subject && !["quota", "credits", "reset", "renewal", "this_mac", "app_started", "keep_alive_paused", "update", "found"].includes(one.kind)) {
        expect(name?.text, one.kind).toBe(one.subject);
      }
    }
  });

  it("names the author as whoever acted: the App, an agent, an Account, you, or AI Juggler for its own acts", () => {
    expect(authorWords(item({ kind: "finished" }))).toBe("Claude Code CLI · Work Max");
    expect(authorWords(item({ kind: "task_done", author: { ...app, kind: "agent", name: "builder-2" } }))).toBe("builder-2");
    expect(authorWords(item({ kind: "quota", author: { ...app, kind: "account", name: "Work Max" } }))).toBe("Work Max");
    expect(authorWords(item({ kind: "stopped", author: { ...app, kind: "you" } }))).toBe("You");
    expect(authorWords(item({ kind: "switch", author: { ...app, kind: "juggler" } }))).toBe("AI Juggler");
    expect(placeWords(item({ kind: "message" }, { pool: "Release", thread: "Export flow" }))).toBe("Release · Export flow");
    expect(placeWords(item({ kind: "quota", author: { ...app, kind: "account", name: "Work Max" } }))).toBe("Account");
  });

  it("says a crash's restart and what a switch carried in the facts line", () => {
    const restart = item({ kind: "restarted" }, { attempt: 1, tries: 3 });
    expect(factsLine(item({ kind: "failed" }), restart)).toBe("AI Juggler restarted it (try 1 of 3)");
    expect(factsLine(item({ kind: "switch" }, { fidelity: "history, 2 tasks", mechanism: "pack" }))).toBe("carried: history, 2 tasks · by pack");
    expect(factsLine(item({ kind: "finished" }))).toBeNull();
  });

  it("says times as R10 does: relative under an hour, the clock today, Yesterday, then the date", () => {
    expect(feedWhen(NOW - 20_000, NOW)).toBe("just now");
    expect(feedWhen(NOW - 12 * 60_000, NOW)).toBe("12 min");
    expect(feedWhen(NOW - 2 * 3_600_000, NOW)).toMatch(/^8:00/);
    expect(feedWhen(NOW - 20 * 3_600_000, NOW)).toMatch(/^Yesterday 2:00/);
    expect(feedWhen(NOW - 5 * 86_400_000, NOW)).toMatch(/Nov 9/);
    expect(exactWhen(NOW)).toContain("2026");
    expect(dayLabel(NOW - 60_000, NOW)).toBe("Today");
    expect(dayLabel(NOW - 20 * 3_600_000, NOW)).toBe("Yesterday");
    expect(durationWords(30_000)).toBe("under a minute");
    expect(durationWords(125 * 60_000)).toBe("2 h 5 min");
  });

  it("rolls a key up into one sentence with its count and place", () => {
    const head = item({ kind: "finished" }, { project: "trip-notes" });
    expect(said(rollupSentence("finished", 4, head))).toBe("finished 4 sessions in trip-notes");
    expect(said(rollupSentence("task_done", 3, head))).toBe("finished 3 tasks");
    expect(said(rollupSentence("message", 4, item({ kind: "message", subject: "Export flow" })))).toBe("4 messages in Export flow");
  });

  it("digests what happened by kind, broken first", () => {
    expect(digestWords(["finished", "finished", "failed", "switch", "reset", "worked"])).toBe("1 stopped with an error, 2 finished, 1 session worked on in other apps, 1 switch, 1 more update");
    expect(digestWords([])).toBe("Nothing changed");
    expect(digestWords(["import"])).toBe("1 more update");
  });
});

describe("D-118 R5 — the away divider's words, on a pinned clock (the morning summary's 'starts plain' rule)", () => {
  // Pinned: the words never depend on today's date (CI 43777785232: the
  // summary card's test aged past a week and its format gained the date).
  const at = new Date(2026, 9, 26, 9, 12, 0).getTime();
  it("starts plain, a clock time, when the last look was the same day", () => {
    expect(awayWords(new Date(2026, 9, 26, 1, 56, 0).getTime(), at)).toMatch(/^While you were away · since 1:56\sAM$/);
  });
  it("adds the day only when the last look was another day", () => {
    expect(awayWords(new Date(2026, 9, 25, 21, 12, 0).getTime(), at)).toMatch(/^While you were away · since Yesterday 9:12\sPM$/);
    expect(awayWords(new Date(2026, 9, 20, 21, 12, 0).getTime(), at)).toBe("While you were away · since Oct 20");
  });
});
