/**
 * The rc27 P1 walk: an App picked in the New pool wizard became a card
 * reading "7 sessions now" (one open, six long ended) while Review listed
 * its members. Review and the card now say the same: its open sessions
 * here now, by the resolver's most-specific rule.
 */
import { describe, expect, it } from "vitest";
import { account, instance } from "../test/fixtures";
import type { DraftMember } from "./new-pool";
import { openNowByMember, openNowWords } from "./pool-open-now";

const member = (ref: string, kind: DraftMember["kind"]): DraftMember => ({ ref, kind, label: ref, role: "member", level: null, reach: kind === "session" ? "full" : "later" });

const draft = (members: DraftMember[]) => ({ every: false, projects: ["web"] as (string | null)[], members });

const apps = [{ id: "cli:lab", folder: account({ id: "lab" }) }, { id: "cli:codex-g", folder: account({ id: "codex-g" }) }];

describe("an App member's sessions now (the rc27 P1 walk)", () => {
  const sessions = [
    instance({ id: "lead", account_id: "lab", project_id: "web", status: "running" }),
    instance({ id: "other", account_id: "lab", project_id: "web", status: "waiting_user" }),
    instance({ id: "ended-1", account_id: "lab", project_id: "web", status: "finished" }),
    instance({ id: "ended-2", account_id: "lab", project_id: "web", status: "stopped" }),
    instance({ id: "elsewhere", account_id: "lab", project_id: "api", status: "running" }),
    instance({ id: "codex-ended", account_id: "codex-g", project_id: "web", provider: "codex", status: "finished" }),
  ];

  it("counts the open ones in scope that no more specific member names", () => {
    const counts = openNowByMember(draft([member("session:lead", "session"), member("app:cli:lab", "app"), member("app:cli:codex-g", "app")]), sessions, { apps, subscriptions: [] });
    // The Lead is named on its own; the App brings the other open one.
    expect(Object.fromEntries(counts)).toEqual({ "session:lead": 1, "app:cli:lab": 1, "app:cli:codex-g": 0 });
  });

  it("an App is more specific than its Tool, its Tool than everyone", () => {
    const counts = openNowByMember(draft([member("all", "everyone"), member("tool:claude-code", "tool"), member("app:cli:lab", "app")]), sessions, { apps, subscriptions: [] });
    expect(Object.fromEntries(counts)).toEqual({ all: 0, "tool:claude-code": 0, "app:cli:lab": 2 });
  });

  it("says it as the canvas's card says it", () => {
    expect(openNowWords(1)).toBe("1 open now · new ones join");
    expect(openNowWords(0)).toBe("none open now · new ones join");
  });
});
