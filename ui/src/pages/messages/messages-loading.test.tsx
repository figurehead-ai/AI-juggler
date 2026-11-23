/**
 * AJ-E1552 — Messages' skeletons draw only while their read loads: the
 * thread list, a thread and the shared documents each show a skeleton
 * while loading (Still loading after 3 s), their own empty words, and an
 * error with Retry that reads again.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/messages";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../../test/loading-kit";
import { MessagesView } from "./MessagesView";
import { SharedDocs } from "./SharedDocs";
import { ThreadView } from "./ThreadView";

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const list = () => render(<MessagesView poolId="pool-a" members={[]} now={0} />);
const docs = () => render(<SharedDocs poolId="pool-a" now={0} />);
const thread = () => render(<ThreadView threadId="t-1" poolId="pool-a" members={[]} now={0} />);

describe("the thread list", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "messagesThreads").mockReturnValue(never());
    list();
    await expectSlowSkeleton("threads-skeleton");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "messagesThreads").mockResolvedValue({ threads: [], next: null, paused: null });
    list();
    await expectEmpty("messages-empty", "No messages yet.", "threads-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "messagesThreads").mockRejectedValue(new Error("down"));
    list();
    await expectErrorRetry("the threads", read, "threads-skeleton");
  });
});

describe("a thread", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "messagesThread").mockReturnValue(never());
    thread();
    await expectSlowSkeleton("thread-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "messagesThread").mockRejectedValue(new Error("down"));
    thread();
    await expectErrorRetry("this thread", read, "thread-skeleton");
  });
});

describe("the shared documents", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "messagesDocs").mockReturnValue(never());
    docs();
    await expectSlowSkeleton("shared-docs-skeleton");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "messagesDocs").mockResolvedValue({ docs: [], next: null });
    docs();
    await expectEmpty("shared-docs-empty", "None yet.", "shared-docs-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "messagesDocs").mockRejectedValue(new Error("down"));
    docs();
    await expectErrorRetry("the shared documents", read, "shared-docs-skeleton");
    expect(screen.queryByText("down")).toBeNull();
  });
});
