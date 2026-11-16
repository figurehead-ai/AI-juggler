/**
 * Tailwind v4 makes `text-<name>` from both a size (`--text-<name>`) and a
 * colour (`--color-<name>`). When both exist, the colour wins and the size,
 * its line height and its weight are dropped: 0.4.6's calm tokens added
 * `--color-section` and every `text-section` title (101 of them) lost its
 * 15 px semibold. The same holds for the other namespaces Tailwind reads
 * by one utility prefix, so no theme key may be shared between them.
 */
import { describe, expect, it } from "vitest";
import styles from "../styles.css?raw";

/** The names a namespace defines (`--text-section` → "section"), its
 * sub-keys (`--text-section--line-height`) left out. */
function names(namespace: string): Set<string> {
  const found = new Set<string>();
  for (const match of styles.matchAll(new RegExp(`--${namespace}-([a-z0-9-]+?)(--[a-z-]+)?:`, "g"))) {
    if (!match[2]) found.add(match[1]!);
  }
  return found;
}

/** Namespaces that share one utility prefix in Tailwind v4. */
const SHARED: [prefix: string, a: string, b: string][] = [
  ["text-", "text", "color"],
  ["shadow-", "shadow", "color"],
  ["font-", "font", "font-weight"],
];

describe("theme keys never collide in a utility prefix", () => {
  it("the sizes it guards are read", () => {
    expect(names("text").size).toBeGreaterThan(3);
  });
  it.each(SHARED)("no two namespaces share a name under `%s<name>`", (_prefix, a, b) => {
    const first = names(a);
    expect([...names(b)].filter((name) => first.has(name) && !(a === "font" && name.startsWith("weight-")))).toEqual([]);
  });
});
