/**
 * The rc31 walk (P11): adding a session to a second pool on the canvas
 * took it out of the first. D-62 decision 10: a session can be in several
 * pools, so adding keeps its other pools and Undo takes it out of this
 * one only; a sealed pool (decision 14) moves it, and says which pool it
 * left.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";

const calls: string[] = [];
const toasts: { message: string; detail?: string; undo?: { onUndo: () => Promise<void> } }[] = [];

vi.mock("../../lib/ipc/pools", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../lib/ipc/pools")>()),
  poolMove: vi.fn(async (session: string, pool: string) => {
    calls.push(`move:${session}:${pool}`);
    return { pools: [] };
  }),
}));
vi.mock("../../lib/ipc/pool-model", () => ({
  memberRef: (kind: string, id: string) => `${kind}:${id}`,
  poolJoin: vi.fn(async (pool: string, member: string, role: string) => {
    calls.push(`join:${pool}:${member}:${role}`);
    return { pools: [] };
  }),
  poolLeave: vi.fn(async (pool: string, member: string) => {
    calls.push(`leave:${pool}:${member}`);
    return { pools: [] };
  }),
}));
vi.mock("../../lib/pools", () => ({ setPools: vi.fn(), defaultPoolIdOf: vi.fn() }));
vi.mock("../../components/Toast", () => ({ showToast: (toast: (typeof toasts)[number]) => toasts.push(toast) }));

import { addToPool } from "./actions";
import { previewPoolsInfo, type PoolInfo } from "../../lib/ipc/pools";
import { MOCK_SESSIONS } from "../../preview/fixtures";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

const session: InstanceInfo = { ...MOCK_SESSIONS[0]!, id: "s1" };
const poolB = (sealed: boolean): PoolInfo => ({ ...previewPoolsInfo().pools[0]!, id: "pool-b", name: "B", scope_all: true, sealed });
const place = vi.fn(async () => {});

beforeEach(() => {
  calls.length = 0;
  toasts.length = 0;
});

describe("adding a session on the canvas", () => {
  it("keeps it in the pool it's in, and Undo takes it out of this one only", async () => {
    await addToPool(poolB(false), session, null, place, "pool-a", "A");
    expect(calls).toEqual(["join:pool-b:session:s1:member"]);
    expect(toasts[0]!.detail).toBeUndefined();
    await toasts[0]!.undo!.onUndo();
    expect(calls).toEqual(["join:pool-b:session:s1:member", "leave:pool-b:session:s1"]);
  });

  it("moves it into a sealed pool and says which pool it left", async () => {
    await addToPool(poolB(true), session, null, place, "pool-a", "A");
    expect(calls).toEqual(["move:s1:pool-b"]);
    expect(toasts[0]!.detail).toContain("It left «A»");
    await toasts[0]!.undo!.onUndo();
    expect(calls).toEqual(["move:s1:pool-b", "move:s1:pool-a"]);
  });
});
