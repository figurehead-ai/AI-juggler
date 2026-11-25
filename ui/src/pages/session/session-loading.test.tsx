/**
 * AJ-E1552 — a session page's skeletons draw only while their read
 * loads: the last messages, the Pools tab and an ended terminal's "what
 * it kept" each show a skeleton while loading (Still loading after 3 s),
 * their own empty words, and an error with Retry that reads again.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, it, vi } from "vitest";
import { resetConversationStore, type FeedSource } from "../../lib/conversation-store";
import * as poolModel from "../../lib/ipc/pool-model";
import * as terminalIo from "../../lib/ipc/terminal-io";
import { MOCK_SESSIONS } from "../../preview/fixtures";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../../test/loading-kit";
import { LastMessages } from "./LastMessages";
import { fromInstance } from "./model";
import { SessionPools } from "./SessionPools";
import { SessionTerminal, forgetKept } from "./SessionTerminal";

const SESSION = { ...MOCK_SESSIONS[0]!, id: "loading-kit-session" };

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  resetConversationStore();
  forgetKept();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const messages = (follow: FeedSource["follow"]) => render(<LastMessages model={fromInstance(SESSION)} visible source={{ key: "loading-kit", follow }} />);

describe("the last messages", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    messages(() => never());
    await expectSlowSkeleton("last-messages-skeleton");
  });
  it("empty: why there is none", async () => {
    messages(async () => ({ state: "none", subscription: null, generation: 1, revision: 1, layout: "stretches", turns: [], earlier: null, status: null, skipped: 0 }));
    await expectEmpty("last-messages-none", /\S/, "last-messages-skeleton");
  });
  it("error with Retry", async () => {
    const follow = vi.fn<FeedSource["follow"]>().mockRejectedValue(new Error("gone"));
    messages(follow);
    await expectErrorRetry("the conversation", follow, "last-messages-skeleton", "Couldn't read them: gone");
  });
});

describe("the Pools tab", () => {
  // Its reads are shared by session id: each test its own session.
  let n = 0;
  const one = () => ({ ...SESSION, id: `pools-kit-${(n += 1)}` });
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(poolModel, "poolsOfSession").mockReturnValue(never());
    render(<SessionPools session={one()} />);
    await expectSlowSkeleton("session-pools-loading");
  });
  it("empty: in no pool, and how to add it", async () => {
    const session = one();
    vi.spyOn(poolModel, "poolsOfSession").mockResolvedValue({ session_id: session.id, pools: [], skipped: [], account_unknown: false });
    render(<SessionPools session={session} />);
    await expectEmpty("session-pools-none", "In no pool now.", "session-pools-loading");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(poolModel, "poolsOfSession").mockRejectedValue(new Error("down"));
    render(<SessionPools session={one()} />);
    await expectErrorRetry("its pools", read, "session-pools-loading");
  });
});

describe("an ended terminal", () => {
  const ended = () => render(<SessionTerminal session={{ ...SESSION, conversation_id: null }} live={false} terminal={<div />} now={Date.now()} />);
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(terminalIo, "terminalKept").mockReturnValue(never());
    ended();
    await expectSlowSkeleton("ended-terminal-skeleton");
  });
  it("empty: nothing was kept, in words", async () => {
    vi.spyOn(terminalIo, "terminalKept").mockResolvedValue(false);
    ended();
    await expectEmpty("ended-terminal-none", /kept/, "ended-terminal-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(terminalIo, "terminalKept").mockRejectedValue(new Error("down"));
    ended();
    await expectErrorRetry("what its terminal kept", read, "ended-terminal-skeleton");
  });
});
