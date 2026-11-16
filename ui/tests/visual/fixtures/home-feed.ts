/**
 * D-118 (AJ-E3701/E3702) — Home's feed in the preview: items seeded as
 * `window.__AIJ_FEED__` (the preview reads them as the backend's pages)
 * and the "last look" seeded as `__AIJ_SUMMARY__`. Shared by the Home
 * feed gallery, the paging spec and the capture families.
 */
import { feedItem } from "../../../src/preview/feed";
import type { FeedItemInfo } from "../../../src/types/generated/FeedItemInfo";


type Over = Omit<Parameters<typeof feedItem>[0], "id" | "at_ms" | "kind" | "category">;

const HOUR = 3_600_000;
const MIN = 60_000;
const WORK = { kind: "app", name: null, provider: "claude-code", folder: "claude-work", folder_label: "Work Max", desktop: false } as const;
const PERSONAL = { kind: "app", name: null, provider: "claude-code", folder: "claude-personal", folder_label: "Personal", desktop: true } as const;
const CODEX = { kind: "app", name: null, provider: "codex", folder: "codex", folder_label: "Codex", desktop: false } as const;

/** The last look, `hours` ago (never the default window), as the seam
 * reads it; the return (the read as Home opened, `generated_ms`) `back`
 * hours ago: what came after it is above the away divider, what came
 * between the two is the digest's only (R5). */
export function away(now: number, hours: number, back = 0) {
  return { since_ms: now - hours * HOUR, generated_ms: now - back * HOUR, default_window: false, rows: [], counts: {}, leftovers: 0 };
}

/** Busy's away: gone from 9 h ago until 4½ h ago, so every card of the
 * busy day is since the return and the night's two are the digest's. */
export const BUSY_AWAY = (now: number) => away(now, 9, 4.5);

/** A few endings newer than the last look (9 h) and one older: the digest's input. */
export function digestFeed(now: number): FeedItemInfo[] {
  const ended = (id: string, ago: number, kind: string, subject: string, level: number) =>
    feedItem({
      id, at_ms: now - ago, kind, category: kind === "failed" ? "needs" : "finished", level, coalesce: id, subject, author: WORK,
      facts: { project: "ai-juggler", cwd: "/Users/you/Dev/ai-juggler", duration_ms: 25 * MIN },
      target: { session_id: id.slice(2) },
    });
  return [
    ended("l:s1", 1 * HOUR, "finished", "Refactor the router", 0),
    ended("l:s2", 2 * HOUR, "failed", "Migrate the schema", 3),
    ended("l:s3", 3 * HOUR, "finished", "Write the docs", 0),
    ended("l:s4", 20 * HOUR, "finished", "An older session", 0),
  ];
}

/** Every kind the feed draws, newest first, a rollup of four finished
 * sessions, and two from the night before (the away digest's). */
export function busyFeed(now: number): FeedItemInfo[] {
  const item = (id: string, ago: number, kind: string, category: string, over: Over) =>
    feedItem({ id, at_ms: now - ago, kind, category, coalesce: id, subject: "Refactor the router", author: WORK, ...over });
  const rolled = [0, 1, 2, 3].map((n) =>
    feedItem({
      id: `l:r${n}`, at_ms: now - (3 * HOUR + n * 20 * MIN), kind: "finished", category: "finished", coalesce: `s:r${n}`, rollup: "v:finished:claude-work:proj-ai-juggler:L0",
      rollup_window_ms: 2 * HOUR, scope: "f:claude-work", subject: `Fix flaky test ${n + 1}`, author: WORK,
      facts: { project: "ai-juggler", project_id: "proj-ai-juggler", duration_ms: 12 * MIN }, target: { session_id: `r${n}` },
    }),
  );
  return [
    item("l:1", 2 * MIN, "finished", "finished", { facts: { project: "ai-juggler", duration_ms: 18 * MIN } }),
    item("l:2", 9 * MIN, "failed", "needs", { level: 3, subject: "Migrate the schema", facts: { project: "payments-api", detail: "exit code 1", duration_ms: 4 * MIN } }),
    // A crash and the restart after it: one session, one card.
    item("n:4", 13 * MIN, "restarted", "needs", { level: 3, coalesce: "s:idx", subject: "Index the docs", author: { ...PERSONAL, kind: "juggler" }, facts: { attempt: 1, tries: 3 } }),
    item("l:3", 14 * MIN, "crashed", "needs", { level: 3, coalesce: "s:idx", subject: "Index the docs", author: PERSONAL, facts: { detail: "signal 9" } }),
    item("l:5", 30 * MIN, "answered", "needs", { subject: "Write the docs", facts: { need: "asks" } }),
    item("l:6", 40 * MIN, "switch", "switches", { level: 1, subject: "Answer the release question", author: { ...WORK, kind: "juggler" }, facts: { from_label: "Work Max", from_provider: "claude-code", to_label: "Codex", to_provider: "codex", mechanism: "copy", fidelity: "full" } }),
    item("l:7", 50 * MIN, "switch_failed", "switches", { level: 2, subject: "Plan the trip notes", author: { ...WORK, kind: "juggler" }, facts: { from_label: "Work Max", to_label: "Personal", detail: "Personal is signed out" } }),
    item("l:8", 1 * HOUR, "quota", "switches", { level: 2, subject: "Work Max", author: { ...WORK, kind: "account", name: "Work Max" }, facts: { window: "5-hour", meter: 8, resets_at_ms: now + 2 * HOUR, sessions: 2 } }),
    item("l:9", 70 * MIN, "reset", "switches", { subject: "Codex", author: { ...CODEX, kind: "account", name: "Codex" }, facts: { window: "weekly", meter: 100 } }),
    item("l:10", 80 * MIN, "task_done", "pools", { subject: "Add the paging test", author: { kind: "agent", name: "Tess", provider: "claude-code", folder: null, folder_label: null, desktop: false }, facts: { pool: "Release crew", pool_id: "pool-1", change: "In progress → Done" } }),
    item("l:11", 90 * MIN, "task_review", "pools", { level: 1, subject: "Review the diff", author: { kind: "agent", name: "Rae", provider: "codex", folder: null, folder_label: null, desktop: false }, facts: { pool: "Release crew", pool_id: "pool-1", change: "In progress → In review" } }),
    item("l:12", 100 * MIN, "message", "pools", { subject: "Which branch?", author: { kind: "agent", name: "Tess", provider: "claude-code", folder: null, folder_label: null, desktop: false }, facts: { pool: "Release crew", pool_id: "pool-1", thread: "Which branch?", thread_count: 3, quote: "I'd cut it from release/0.5 so the paging test lands with the feed." } }),
    item("c:13", 110 * MIN, "worked", "other", { subject: "Plan the trip notes app", author: CODEX, facts: { project: "trip-notes" } }),
    ...rolled,
    // The night (BUSY_AWAY's digest): a failure and a finish while away.
    item("l:14", 6 * HOUR, "failed", "needs", { level: 3, subject: "Rebuild the search index", facts: { project: "ai-juggler", detail: "exit code 2", duration_ms: 40 * MIN } }),
    item("l:15", 7 * HOUR, "finished", "finished", { subject: "Bump the dependencies", facts: { project: "ai-juggler", duration_ms: 22 * MIN } }),
  ];
}

/** `count` single cards (no rollup), newest first: the paging test's flood. */
export function manyFeed(now: number, count: number): FeedItemInfo[] {
  return Array.from({ length: count }, (_, n) =>
    feedItem({
      id: `l:p${n}`, at_ms: now - n * 7 * MIN, kind: "finished", category: "finished", coalesce: `s:p${n}`, subject: `Session ${n + 1}`, author: WORK,
      facts: { project: "ai-juggler", duration_ms: 10 * MIN }, target: { session_id: `p${n}` },
    }),
  );
}
