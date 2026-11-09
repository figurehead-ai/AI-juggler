/**
 * AJ-E3009 R3: the Tasks screen's pool filter follows the pickers rule.
 */
import { describe, expect, it } from "vitest";
import { poolPicker, type PoolChoice } from "./pool-picker";

const pool = (id: string, members: number, scope: (string | null)[] = ["p1"], scope_all = false): PoolChoice => ({ id, name: id, members, scope, scope_all });

describe("the Tasks pool filter (AJ-E3009 R3, the pickers rule)", () => {
  it("suggests the pools with members, the most first and at most five; empty ones are marked", () => {
    const pools = [pool("empty", 0), pool("two", 2), pool("nine", 9), pool("one", 1), pool("a", 3), pool("b", 4), pool("c", 5)];
    const { options, groups } = poolPicker(pools, null);
    expect(groups[0]!.values).toEqual(["nine", "c", "b", "a", "two"]);
    expect(options.find((option) => option.value === "empty")).toMatchObject({ empty: true, hint: "No members" });
    expect(options.find((option) => option.value === "one")).toMatchObject({ empty: false, hint: "1 member" });
  });

  it("under a project: only the pools whose scope holds it, a pool over every project too", () => {
    const pools = [pool("here", 2, ["p1"]), pool("there", 3, ["p2"]), pool("both", 1, ["p1", "p2"]), pool("every", 1, [], true)];
    expect(poolPicker(pools, "p1").options.map((option) => option.value)).toEqual(["here", "both", "every"]);
    expect(poolPicker(pools, null).options).toHaveLength(4);
  });
});
