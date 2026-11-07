/**
 * The 7b walk (2026-11-07): Writer and reviewer of two Apps left both on
 * Member, though its card promised Lead and Reviewer, and nothing said so.
 * The Roles step names what's still to choose.
 */
import { describe, expect, it } from "vitest";
import { rolesToChoose } from "./new-pool";
import type { PoolRole } from "./ipc/pools";

const draft = (roles: PoolRole[]) => ({ template: "writer-reviewer", members: roles.map((role) => ({ role })) });

describe("the roles a template still needs", () => {
  it("are its roles nobody holds", () => {
    expect(rolesToChoose(draft(["member", "member"]))).toEqual(["lead", "reviewer"]);
    expect(rolesToChoose(draft(["lead", "member"]))).toEqual(["reviewer"]);
    expect(rolesToChoose(draft(["lead", "reviewer"]))).toEqual([]);
  });
});
