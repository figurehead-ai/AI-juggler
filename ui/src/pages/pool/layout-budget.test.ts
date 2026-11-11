/**
 * AJ-E2808's performance budget (vitest): the automatic layout of 50
 * members takes ≤ 50 ms, with half of them already placed by hand (the
 * placed ones never move). CPU time, so a busy Mac doesn't fail it.
 */
import { describe, expect, it } from "vitest";

import { cpuTimed } from "../../test/cpuTime";
import { layout, type LayoutNode, type Point } from "./layout";

const MEMBERS: LayoutNode[] = Array.from({ length: 50 }, (_, index) => ({ id: `s-${String(index).padStart(2, "0")}`, role: index === 0 ? "lead" : "member" }));
const PLACED = new Map<string, Point>(MEMBERS.filter((_, index) => index % 2 === 1).map((node, index) => [node.id, { x: 40 + (index % 5) * 300, y: 900 + Math.floor(index / 5) * 160 }]));

describe("the automatic layout's budget", () => {
  it("lays out 50 members in ≤ 50 ms and keeps the placed ones where they are", () => {
    layout(MEMBERS, PLACED); // warm up the JIT once
    const runs = Array.from({ length: 5 }, () => cpuTimed(() => layout(MEMBERS, PLACED)));
    const median = runs.map((run) => run.cpuMs).sort((a, b) => a - b)[2] ?? Infinity;
    expect(median).toBeLessThanOrEqual(50);
    const out = runs[0]!.result;
    expect(out.size).toBe(50);
    for (const [id, at] of PLACED) expect(out.get(id)).toEqual(at);
  });
});
