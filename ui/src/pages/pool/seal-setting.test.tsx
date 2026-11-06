/**
 * The P7 walk: a pool can be sealed and unsealed after it's made, and
 * each way says first what it changes, then asks (D-62, AJ-E3005).
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

const { ofSession, setSealed } = vi.hoisted(() => ({
  ofSession: vi.fn(async (id: string) => ({
    session_id: id,
    pools: id === "c1" ? [{ pool_id: "client", pool_name: "Client" }, { pool_id: "web", pool_name: "web" }] : [{ pool_id: "client", pool_name: "Client" }],
    skipped: id === "c1" ? [{ pool_id: "web", pool_name: "web", reason: "sealed_elsewhere", by_pool: "client", by_pool_name: "Client", words: "" }] : [],
    account_unknown: false,
  })),
  setSealed: vi.fn(async () => ({ pools: [], placements: [] })),
}));
vi.mock("../../lib/ipc/pool-model", () => ({ poolsOfSession: ofSession, poolSetSealed: setSealed }));
vi.mock("../../lib/pools", () => ({ setPools: vi.fn() }));

import { SealSetting, sealChanges } from "./SealSetting";
import type { PoolInfo } from "../../lib/ipc/pools";

const pool = (sealed: boolean) => ({ id: "client", name: "Client", kind: "user", sealed }) as unknown as PoolInfo;
const members = [{ id: "c1", name: "c1" }, { id: "c2", name: "c2" }];

afterEach(() => {
  cleanup();
  setSealed.mockClear();
});

describe("sealing a pool after it's made", () => {
  it("says who leaves which pool before sealing, and who comes back before unsealing", async () => {
    expect(await sealChanges(pool(false), members)).toEqual(["c1 leaves web."]);
    expect(await sealChanges(pool(true), members)).toEqual(["c1 goes back into web."]);
  });

  it("asks first, and seals only on Seal it", async () => {
    const view = render(<SealSetting pool={pool(false)} members={members} />);
    fireEvent.click(view.getByTestId("pool-seal-open"));
    await waitFor(() => expect(view.getByTestId("pool-seal-changes").textContent).toBe("c1 leaves web."));
    expect(setSealed).not.toHaveBeenCalled();
    fireEvent.click(view.getByTestId("pool-seal-yes"));
    await waitFor(() => expect(setSealed).toHaveBeenCalledWith("client", true));
  });
});
