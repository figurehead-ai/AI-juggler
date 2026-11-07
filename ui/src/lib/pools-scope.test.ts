/**
 * The P9 walk (moving from 0.3): a 0.3 pool with no project said nothing
 * about where it works; D-62.16 says it now works in every project, and
 * says so, on its header's Scope chip (AJ-E3104 R1) and its card.
 */
import { describe, expect, it } from "vitest";
import { scopeChipWords, scopeLabel } from "./pools";

const pool = (kind: string, scope_all: boolean, scope: (string | null)[]) => ({ kind, scope_all, scope, project_id: scope[0] ?? null }) as never;
const projects = [{ id: "p-shop", name: "shop" }, { id: "p-blog", name: "blog" }] as never;

describe("a pool's scope in words", () => {
  it("a pool you made says where it works: every project, its projects", () => {
    expect(scopeChipWords(pool("user", true, []), projects)).toBe("every project");
    expect(scopeLabel(pool("user", true, []), projects)).toBe("Every project");
    expect(scopeChipWords(pool("user", false, ["p-shop"]), projects)).toBe("shop");
    expect(scopeChipWords(pool("user", false, ["p-shop", "p-blog"]), projects)).toBe("shop, blog");
    expect(scopeChipWords(pool("user", false, []), projects)).toBe("not set");
  });

  it("a project's own pool says it in its title instead", () => {
    expect(scopeChipWords(pool("default", false, ["p-shop"]), projects)).toBeNull();
  });
});
