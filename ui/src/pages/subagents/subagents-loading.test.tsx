/**
 * AJ-E1552 — Subagents draw a skeleton only while their read loads: the
 * list and a subagent's page each show a skeleton while loading (Still
 * loading after 3 s), their own empty words, and an error with Retry (the
 * list's own "Try again") that reads again.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import * as ipc from "../../lib/ipc/subagents";
import { forgetPaged } from "../../lib/paged";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../../test/loading-kit";
import { ProjectSubagentsView as ProjectSubagents } from "./ProjectSubagentsView";
import { SubagentDetail } from "./SubagentDetail";

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  forgetPaged();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const list = (projectId = "p1") => render(<ProjectSubagents projectId={projectId} now={0} />);
const detail = (subagentId = "a-1") => render(<SubagentDetail target={{ profileId: "acc", conversationId: "c-1", subagentId }} sessionTitle="web" visible onBack={() => {}} onOpen={() => {}} onConversation={null} />);

describe("the subagent list", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "subagentsProject").mockReturnValue(never());
    list();
    await expectSlowSkeleton("subagents-loading");
  });
  it("empty", async () => {
    list("a-project-with-none");
    await expectEmpty("subagents-empty", "None of these sessions has started a subagent.", "subagents-loading");
  });
  it("error with Try again", async () => {
    const read = vi.spyOn(ipc, "subagentsProject").mockRejectedValue(new Error("down"));
    list();
    expect((await screen.findByTestId("subagents-unread")).textContent).toContain("Its subagents couldn't be read");
    expect(screen.queryByTestId("subagents-loading")).toBeNull();
    const before = read.mock.calls.length;
    screen.getByRole("button", { name: "Try again" }).click();
    await vi.waitFor(() => expect(read.mock.calls.length).toBeGreaterThan(before));
  });
});

describe("a subagent's page", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "subagentsDetail").mockReturnValue(never());
    detail();
    await expectSlowSkeleton("subagent-detail-skeleton");
  });
  it("empty: no longer on record", async () => {
    detail("no-such-subagent");
    await expectEmpty("subagent-detail-missing", /./, "subagent-detail-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "subagentsDetail").mockRejectedValue(new Error("down"));
    detail();
    await expectErrorRetry("this subagent", read, "subagent-detail-skeleton");
    expect(screen.queryByText("down")).toBeNull();
  });
});
