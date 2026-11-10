/**
 * Walk K1 (AJ-E3306, rc24): a project's pool with 500 members, all ended
 * and in it through their project (no member of their own), drew no node,
 * and its canvas still taught "Three steps to a working pool" with "Add
 * who's in it" unticked. Who's in it is its sessions: the canvas says
 * they show as they start, and teaches no step already done.
 */
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { refreshPools } from "../../lib/pools";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { PoolPage } from "../PoolPage";
import { resetCanvasTests, seams } from "./canvas.test-kit";

// The backend's rows for a project's sessions: in it through the project.
vi.mock("../../lib/ipc/pools", async (real) => {
  const actual = await real<typeof import("../../lib/ipc/pools")>();
  return {
    ...actual,
    poolMembers: async (...args: Parameters<typeof actual.poolMembers>) => {
      const page = await actual.poolMembers(...args);
      return { ...page, items: page.items.map((row) => ({ ...row, via: "project" })) };
    },
  };
});

beforeEach(resetCanvasTests);
afterEach(() => {
  cleanup();
  localStorage.clear();
});

const ended: InstanceInfo[] = MOCK_SESSIONS.slice(0, 3).map((session) => ({ ...session, status: "finished", runtime: undefined }));

describe("a pool whose sessions are all ended (walk K1)", () => {
  it("has who's in it: no 'Three steps', and it says its sessions show as they start", async () => {
    seams().__AIJ_POOLS__ = {
      pools: [{ id: "pool-web", name: "web", rules: [] }],
      members: ended.map((session) => ({ session_id: session.id, pool_id: "pool-web", role: "member" })),
    };
    await act(async () => {
      await refreshPools();
    });
    render(
      <ConversationsProvider want={null}>
        <ActionHostProvider host={{}}>
          <PoolPage poolId="pool-web" sessions={ended} projects={MOCK_PROJECTS} accounts={[]} now={Date.now()} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("canvas-rules-only").textContent).toContain("None of its sessions is open: they show here as they start"));
    expect(screen.queryByText("Three steps to a working pool")).toBeNull();
    expect(screen.queryAllByTestId("canvas-node")).toHaveLength(0);
  });
});
