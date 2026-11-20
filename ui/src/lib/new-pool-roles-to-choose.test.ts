/**
 * The 7b walk (2026-11-07): Writer and reviewer of two Apps left both on
 * Member, though its card promised Lead and Reviewer, and nothing said so.
 * The Roles step names what's still to choose.
 */
import { describe, expect, it } from "vitest";
import { draftOf, leadCandidates, leadOf, rolesToChoose, withLead, type Candidate } from "./new-pool";
import { templateById } from "./pool-templates";
import type { PoolRole } from "./ipc/pools";

const draft = (roles: PoolRole[]) => ({ template: "writer-reviewer", members: roles.map((role) => ({ role })) });

describe("the roles a template still needs", () => {
  it("are its roles nobody holds", () => {
    expect(rolesToChoose(draft(["member", "member"]))).toEqual(["lead", "reviewer"]);
    expect(rolesToChoose(draft(["lead", "member"]))).toEqual(["reviewer"]);
    expect(rolesToChoose(draft(["lead", "reviewer"]))).toEqual([]);
  });
});

// The rc26 P1 walk (D-92) and AJ-E3011's newcomer walks: with only Apps
// picked, no row offered Lead — a dead end. The lead picker offers the
// sessions open now, an App's next session, or no lead for now.
describe("who leads, never a dead end (AJ-E3011 §3)", () => {
  const app = (ref: string, role: PoolRole = "member") => ({ ref, kind: "app" as const, label: ref, role, level: null, reach: "later" as const });
  const session = (ref: string, role: PoolRole = "member") => ({ ref, kind: "session" as const, label: ref, role, level: null, reach: "full" as const });
  const base = { ...draftOf(templateById("blank"), "web"), members: [app("app:cli:work"), app("app:cli:codex", "reviewer")] };

  it("an App's next session leads: the App stays a Member, the pool waits for its session", () => {
    const next = withLead(base, { kind: "next", app: "app:cli:work" });
    expect(leadOf(next)).toEqual({ kind: "next", app: "app:cli:work" });
    expect(next.members.map((member) => member.role)).toEqual(["member", "reviewer"]);
  });

  it("a session open now leads: it joins as Lead, and the wait ends", () => {
    const open: Candidate = { ref: "session:s1", kind: "session", label: "web · fix", detail: "web · Work · Working", group: "open", reach: "full", blocked: null, lastMs: 2 };
    const led = withLead(withLead(base, { kind: "next", app: "app:cli:work" }), { kind: "session", ref: "session:s1" }, open);
    expect(leadOf(led)).toEqual({ kind: "session", ref: "session:s1" });
    expect(led.leadNext).toBeNull();
    expect(led.members.find((member) => member.ref === "session:s1")?.role).toBe("lead");
  });

  it("one session leads: picking another makes the last a Member; no lead for now clears both", () => {
    const two = { ...base, members: [...base.members, session("session:a", "lead"), session("session:b")] };
    const moved = withLead(two, { kind: "session", ref: "session:b" });
    expect(moved.members.filter((member) => member.role === "lead").map((member) => member.ref)).toEqual(["session:b"]);
    const none = withLead(moved, { kind: "none" });
    expect(leadOf(none)).toEqual({ kind: "none" });
    expect(none.members.some((member) => member.role === "lead")).toBe(false);
  });

  it("offers the sessions open now that can act, most recent first", () => {
    const row = (ref: string, over: Partial<Candidate>): Candidate => ({ ref, kind: "session", label: ref, detail: "", group: "open", reach: "full", blocked: null, lastMs: 0, ...over });
    const rows = [row("old", { lastMs: 1 }), row("new", { lastMs: 9 }), row("ended", { group: "recent" }), row("watched", { reach: "watched" }), row("out", { blocked: "Works in docs" })];
    expect(leadCandidates(rows).map((one) => one.ref)).toEqual(["new", "old"]);
  });
});
