/**
 * SK48-10 (the 0.4.8 soak, P11): a session in pools A and B said "Pool: A"
 * in its header. Every pool it's in is named there, each a door to its
 * pool; one pool reads as before.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import { SessionPoolChip } from "./HeaderDoors";

const POOLS = [
  { id: "p-a", name: "A", kind: "named", members: 2 },
  { id: "p-b", name: "B", kind: "named", members: 2 },
  { id: "p-web", name: "web · all sessions", kind: "default", members: 1 },
];
let inPools = ["p-a", "p-b", "p-web"];

vi.mock("../../lib/pools", async (actual) => ({
  ...(await actual<typeof import("../../lib/pools")>()),
  usePools: () => ({ pools: POOLS }),
  usePoolPlace: () => ({ pool_id: "p-a" }),
  poolById: (_pools: unknown, id: string) => POOLS.find((pool) => pool.id === id),
}));
vi.mock("./useSessionPools", () => ({
  useSessionPools: () => ({ session_id: "s1", pools: inPools.map((pool_id) => ({ pool_id })), skipped: [], account_unknown: false }),
}));

afterEach(() => {
  cleanup();
  inPools = ["p-a", "p-b", "p-web"];
});

it("names both pools a session is in, each a door to its pool", () => {
  const openPool = vi.fn();
  const view = render(
    <ActionHostProvider host={{ openPool }}>
      <SessionPoolChip sessionId="s1" />
    </ActionHostProvider>,
  );
  // Its project's own pool, with no one else in it, isn't named.
  expect(view.getByTestId("session-pool-chip").textContent).toBe("Pools:A,B");
  const doors = view.getAllByTestId("session-pool-door");
  fireEvent.click(doors[1]!);
  expect(openPool).toHaveBeenCalledWith("p-b");
});

it("one pool reads as before", () => {
  inPools = ["p-a"];
  const view = render(
    <ActionHostProvider host={{ openPool: vi.fn() }}>
      <SessionPoolChip sessionId="s1" />
    </ActionHostProvider>,
  );
  expect(view.getByTestId("session-pool-chip").textContent).toBe("Pool: A");
});

it("keeps the pool its place names when its pools list leaves it out (the project's own pool)", () => {
  inPools = [];
  const view = render(
    <ActionHostProvider host={{ openPool: vi.fn() }}>
      <SessionPoolChip sessionId="s1" />
    </ActionHostProvider>,
  );
  expect(view.getByTestId("session-pool-chip").textContent).toBe("Pool: A");
});
