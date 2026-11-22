/**
 * AJ-E3712 vitest — the agent digest in Home's feed: its words from the
 * rollup's facts (state words follow the facts; reverted said as
 * reverted; never a pass count beside "not run"), the first lines shown
 * and the rest behind "and N more agents · Q quiet", each name opening
 * its session, and calm ink for every level.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomeFeedDigest } from "./HomeFeedDigest";
import { digestHeader, digestLineWords, digestMoreWords } from "../lib/feed-words";
import { feedItem } from "../preview/feed";
import type { DigestInfo } from "../types/generated/DigestInfo";
import type { DigestLineInfo } from "../types/generated/DigestLineInfo";

afterEach(cleanup);

const line = (name: string, over: Partial<DigestLineInfo> = {}): DigestLineInfo => ({
  session_id: `s-${name}`,
  name,
  author: { provider: null, folder: null, folder_label: null, desktop: false },
  level: 1,
  state: "working",
  notable: false,
  stuck_min: null,
  left_processes: null,
  runway_left: null,
  restarted: false,
  task: null,
  turns: null,
  files: null,
  tests: null,
  passed: null,
  failed: null,
  reverted: false,
  caveat: null,
  quote: null,
  quote_kind: null,
  at_ms: 0,
  ...over,
});

const said = (one: DigestLineInfo) => {
  const words = digestLineWords(one);
  return `${one.name}${words.state}${words.said ?? ""}${words.facts ?? ""}${words.last ?? ""}`;
};

describe("AJ-E3712 — the digest's words", () => {
  it.each([
    [line("web", { level: 3, state: "failed", quote: "The build failed: missing module auth.", quote_kind: "failure" }), "web · failed: “The build failed: missing module auth.”"],
    [line("api", { level: 2, state: "asks", quote: "Should I bump the major version?", quote_kind: "asks" }), "api · asks you: “Should I bump the major version?”"],
    [line("infra", { level: 2, state: "stuck", stuck_min: 15, left_processes: 2 }), "infra · stuck for 15 min · left 2 processes running"],
    [line("parser", { state: "idle", turns: 1, reverted: true, quote: "The new date library broke two callers, so I reverted the whole change.", quote_kind: "last" }), "parser · idle · 1 turn · reverted · last: “The new date library broke two callers, so I reverted the whole change.”"],
    [line("docs", { state: "finished", tests: "passed", passed: 52, files: 3 }), "docs · finished · 3 files changed · tests passed (52)"],
    [line("ui", { state: "working" }), "ui · working"],
    [line("svc", { state: "working", restarted: true }), "svc · working · restarted after a crash"],
    [line("db", { state: "idle", tests: "not_run", caveat: "not_run" }), "db · idle · tests not run"],
  ])("%#: says the facts it holds", (one, words) => {
    expect(said(one)).toBe(words);
  });

  it("never puts a pass count beside 'not run', nor a caveat the facts don't hold", () => {
    const ran = said(line("x", { tests: "passed", passed: 4, caveat: "not_run" }));
    expect(ran).toContain("tests passed (4)");
    expect(ran).not.toContain("not run");
    for (const one of [line("a", { state: "failed", level: 3 }), line("b", { reverted: true })]) {
      expect(said(one)).not.toMatch(/partly done|!|\bI\b/);
    }
  });

  it("counts agents and folds the rest with the quiet ones", () => {
    expect(digestHeader({ lines: [line("a")] })).toBe("1 agent");
    expect(digestHeader({ lines: [line("a"), line("b")] })).toBe("2 agents");
    expect(digestMoreWords(3, 2)).toBe("and 3 more agents · 2 quiet");
    expect(digestMoreWords(1, 0)).toBe("and 1 more agent");
    expect(digestMoreWords(0, 0)).toBeNull();
  });
});

const digest = (count: number, quiet = 0): DigestInfo => ({
  start_ms: 0,
  end_ms: 1_000,
  lines: Array.from({ length: count }, (_, i) => line(`agent${i + 1}`, i === 0 ? { level: 3, state: "failed" } : {})),
  shown: Math.min(6, count),
  quiet,
  ai: false,
});

describe("AJ-E3712 — the digest item", () => {
  const item = feedItem({ id: "d:1", at_ms: 1_000, kind: "agent_digest", category: "digest" });

  it.each([1, 3, 7])("with %i agents shows at most six, the rest behind one fold", (count) => {
    const onOpen = vi.fn();
    const { getByTestId, queryByTestId } = render(<HomeFeedDigest item={item} digest={digest(count, 2)} when="10:45 AM" onOpen={onOpen} />);
    expect(getByTestId("digest-header").textContent).toBe(`${count} ${count === 1 ? "agent" : "agents"}`);
    expect(within(getByTestId("digest-lines")).getAllByTestId("digest-line")).toHaveLength(Math.min(6, count));
    if (count > 6) {
      const more = getByTestId("digest-more");
      expect(more.textContent).toBe("and 1 more agent · 2 quiet");
      fireEvent.click(more);
      expect(within(getByTestId("digest-lines")).getAllByTestId("digest-line")).toHaveLength(count);
      expect(queryByTestId("digest-more")).toBeNull();
    } else {
      expect(getByTestId("digest-quiet").textContent).toBe("2 quiet");
    }
  });

  it("opens an agent's session from its name, and keeps every line in calm ink", () => {
    const onOpen = vi.fn();
    const { getByRole, getByTestId } = render(<HomeFeedDigest item={item} digest={digest(3)} when="10:45 AM" onOpen={onOpen} />);
    fireEvent.click(getByRole("button", { name: "agent1" }));
    expect(onOpen).toHaveBeenCalledWith("s-agent1");
    expect(getByTestId("feed-digest").innerHTML).not.toMatch(/amber|red|danger|warn/);
  });
});
