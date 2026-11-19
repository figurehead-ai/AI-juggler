/**
 * AJ-E3816 — the view's stand-in echo says each of the backend's shared
 * cases word for word (lib/fixtures/watch-echo.json, which the Rust echo
 * is tested against too), so the words the preview shows are the app's.
 */
import { describe, expect, it } from "vitest";
import cases from "../lib/fixtures/watch-echo.json";
import type { Watch } from "../types/generated/Watch";
import { echoParts, joined } from "./watch-echo";

describe("AJ-E3816 the stand-in echo", () => {
  it("says every shared case as the backend does, its parts joined", () => {
    const names = cases.names as Record<string, string>;
    const agents = cases.agents as Record<string, string>;
    const shelf = {
      target: (t: Watch["targets"][number]) => (t.kind === "any" ? null : (names[`${t.kind}:${t.id}`] ?? null)),
      agent: (id: string) => agents[id] ?? null,
    };
    expect(cases.cases.length).toBeGreaterThanOrEqual(10);
    for (const one of cases.cases) {
      const watch: Watch = JSON.parse(JSON.stringify(one.watch));
      const parts = echoParts(watch, shelf, { nowMs: cases.nowMs, offsetHours: cases.offsetHours });
      expect(joined(parts)).toBe(one.words);
      // The names are marked, and only quoted names are.
      for (const span of parts.head) expect(span.name).toBe(span.text.startsWith("“") && span.text.endsWith("”"));
    }
  });
});
