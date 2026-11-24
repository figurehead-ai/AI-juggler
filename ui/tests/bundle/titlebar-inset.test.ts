// @vitest-environment node
/**
 * 0.5.1 F22: the logo sat under the window's traffic lights. The
 * title-bar inset is a design-system token: `pl-titlebar` only draws if
 * styles.css defines it, past the lights (about 78 px from the left).
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), "utf8");

describe("the title-bar inset", () => {
  it("is a token past the traffic lights, and the rail's head uses it", () => {
    const inset = read("../../src/styles.css").match(/--spacing-titlebar:\s*([\d.]+)rem;/);
    expect(Number(inset?.[1]) * 16).toBeGreaterThanOrEqual(78);
    expect(read("../../src/components/RailHeader.tsx")).toContain("pl-titlebar");
  });
});
