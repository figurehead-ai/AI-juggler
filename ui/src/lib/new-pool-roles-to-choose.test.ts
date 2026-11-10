/**
 * The 7b walk (2026-11-07): Writer and reviewer of two Apps left both on
 * Member, though its card promised Lead and Reviewer, and nothing said so.
 * The Roles step names what's still to choose.
 */
import { describe, expect, it } from "vitest";
import { leadLater, rolesToChoose } from "./new-pool";
import type { PoolRole } from "./ipc/pools";

const draft = (roles: PoolRole[]) => ({ template: "writer-reviewer", members: roles.map((role) => ({ role })) });

describe("the roles a template still needs", () => {
  it("are its roles nobody holds", () => {
    expect(rolesToChoose(draft(["member", "member"]))).toEqual(["lead", "reviewer"]);
    expect(rolesToChoose(draft(["lead", "member"]))).toEqual(["reviewer"]);
    expect(rolesToChoose(draft(["lead", "reviewer"]))).toEqual([]);
  });
});

// The rc26 P1 walk (D-92): Writer and reviewer of two Apps said "Still to
// choose: Lead, Reviewer. Pick them in the list below", where no row
// offered Lead. It says plainly how a Lead gets chosen.
describe("a Lead no member here can take", () => {
  const member = (kind: "session" | "app" | "account", role: PoolRole = "member", reach: "full" | "watched" | "later" = kind === "session" ? "full" : "later") => ({ kind, role, reach });

  it("is said with how one gets chosen when only Apps (or other groups) are picked", () => {
    const words = leadLater({ template: "writer-reviewer", members: [member("app"), member("app", "reviewer")] });
    expect(words).toMatch(/^No Lead yet: Lead is one session/);
    expect(words).toContain("click it on the pool's canvas and make it Lead");
    expect(leadLater({ template: "writer-reviewer", members: [member("account"), member("session", "member", "watched")] })).not.toBeNull();
  });

  it("is nothing when a session here can lead, one leads already, or the template wants none", () => {
    expect(leadLater({ template: "writer-reviewer", members: [member("app"), member("session")] })).toBeNull();
    expect(leadLater({ template: "writer-reviewer", members: [member("session", "lead"), member("app")] })).toBeNull();
    expect(leadLater({ template: "rules-by-account", members: [member("account")] })).toBeNull();
    expect(leadLater({ template: "writer-reviewer", members: [] })).toBeNull();
  });
});
