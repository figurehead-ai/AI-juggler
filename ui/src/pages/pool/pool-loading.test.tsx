/**
 * AJ-E1552 — a pool page's skeletons draw only while their read loads:
 * an empty pool's setup (one record: loading and an error with Retry,
 * never empty), a member's tasks, and the running summary's facts and
 * Live stream each show a skeleton while loading (Still loading after
 * 3 s), their own empty words, and an error with Retry that reads again.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { SLOW_MS } from "../../lib/loading";
import { poolById, refreshPools } from "../../lib/pools";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import * as model from "../../lib/ipc/pool-model";
import * as tasks from "../../lib/ipc/tasks";
import type { PoolCanvasInfo } from "../../lib/ipc/pool-canvas";
import { poolsList } from "../../lib/ipc/pools";
import type { PoolRules } from "../../lib/pool-rules";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../../test/loading-kit";
import { PoolPage } from "../PoolPage";
import { RunningSummary } from "./PoolOverview";
import { openPage, resetCanvasTests, second, seams, seedTeam } from "./canvas.test-kit";

beforeEach(() => {
  resetCanvasTests();
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

/** An empty pool, opened in its guided setup. */
async function openSetup() {
  seams().__AIJ_POOLS__ = { pools: [{ id: "pool-new", name: "New", rules: [] }], members: [] };
  await act(async () => {
    await refreshPools();
  });
  return render(
    <ConversationsProvider want={null}>
      <ActionHostProvider host={{}}>
        <PoolPage poolId="pool-new" sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} now={Date.now()} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

describe("an empty pool's setup (one record: never empty)", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(model, "poolRules").mockReturnValue(never());
    await openSetup();
    await expectSlowSkeleton("pool-page-loading");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(model, "poolRules").mockRejectedValue(new Error("down"));
    await openSetup();
    await screen.findByText("Couldn't load this pool's setup.");
    await expectErrorRetry("this pool's setup", read, "pool-page-loading");
  });
});

describe("a member's tasks", () => {
  const select = async () => {
    seedTeam();
    const view = await openPage();
    return () => fireEvent.click(view.container.querySelector<HTMLButtonElement>(`[data-node-name="${second.id}"]`)!, { detail: 0 });
  };
  it("loading", async () => {
    vi.spyOn(tasks, "tasksOfSession").mockReturnValue(never());
    const click = await select();
    vi.useFakeTimers();
    click();
    await expectSlowSkeleton("inspect-node-tasks-skeleton");
  });
  it("empty", async () => {
    vi.spyOn(tasks, "tasksOfSession").mockResolvedValue({ items: [], next_cursor: null, total: 0 });
    (await select())();
    await expectEmpty("inspect-node-tasks-empty", "It hasn't held or worked a task here.", "inspect-node-tasks-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(tasks, "tasksOfSession").mockRejectedValue(new Error("down"));
    (await select())();
    await expectErrorRetry("its tasks", read, "inspect-node-tasks-skeleton");
  });
});

describe("the running summary's facts and Live", () => {
  const quiet: PoolCanvasInfo = { pool_id: "pool-team", positions: [], edges: [], asks: [], stream: [], since_ms: 0, anchors: {} };
  const rules: PoolRules = { rules: null, watched: [], reload: () => {} };
  const summary = async (canvas: PoolCanvasInfo | null, failed: unknown, reread = vi.fn()) => {
    seedTeam();
    const pool = poolById(await poolsList(), "pool-team");
    if (!pool) throw new Error("the seeded pool");
    const setup = { rows: [], onOpen: () => {}, tools: { projects: [], apps: [] } };
    const progress = { tools: null, toolsItem: false, blockers: [], done: 0, total: 0, complete: false };
    return render(<RunningSummary pool={pool} canvas={canvas} canvasRead={{ failed, reread }} members={[]} open={[]} rules={rules} ruleLabel={() => ""} where="web" needs={0} now={0} setup={setup} progress={progress} onSelect={() => {}} leadNext={null} />);
  };
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(tasks, "tasksCounts").mockReturnValue(never());
    await summary(null, undefined);
    for (const id of ["fact-tasks-skeleton", "fact-messages-skeleton", "inspect-stream-skeleton"]) expect(screen.getByTestId(id)).toBeTruthy();
    expect(screen.queryByTestId("loading-slow")).toBeNull();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SLOW_MS);
    });
    expect(screen.getAllByTestId("loading-slow").map((node) => node.textContent)).toEqual(["Still loading…", "Still loading…", "Still loading…"]);
  });
  it("empty", async () => {
    vi.spyOn(tasks, "tasksCounts").mockResolvedValue({ ready: 0, in_progress: 0, in_review: 0, blocked: 0, todo: 0, done: 0, cancelled: 0 });
    await summary(quiet, undefined);
    await expectEmpty("inspect-stream-empty", "Nothing yet.", "inspect-stream-skeleton");
    expect((await screen.findByText("None open")).closest("[data-fact]")?.getAttribute("data-fact")).toBe("tasks");
    expect(screen.getByText("None yet").closest("[data-fact]")?.getAttribute("data-fact")).toBe("messages");
  });
  it("error with Retry", async () => {
    const counts = vi.spyOn(tasks, "tasksCounts").mockRejectedValue(new Error("down"));
    const reread = vi.fn();
    await summary(null, new Error("down"), reread);
    await screen.findByText("Couldn't load its tasks.");
    await expectErrorRetry("its tasks", counts, "fact-tasks-skeleton");
    await expectErrorRetry("the pool's activity", reread, "inspect-stream-skeleton");
    await expectErrorRetry("its messages", reread, "fact-messages-skeleton");
    expect(screen.queryByText("down")).toBeNull();
  });
});
