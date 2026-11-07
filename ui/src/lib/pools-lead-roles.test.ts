/** D-92: Lead is one session's role; broad members are never offered it. */
import { describe, expect, it } from "vitest";

import { leadHint, rolesFor } from "./pools";

describe("Lead only for a session (D-92)", () => {
  it("offers Lead to a session, by kind or stored member", () => {
    for (const one of ["session", "session:s1", "work:w1", "conversation:folder/c1"]) {
      expect(rolesFor(one)).toContain("lead");
      expect(leadHint(one)).toBeNull();
    }
  });

  it("never offers Lead to an App, Account, tool, Mac or everyone, and says why", () => {
    for (const one of ["app", "app:cli:folder-lab", "account:a1", "tool:codex", "mac:this", "everyone", "all"]) {
      expect(rolesFor(one)).toEqual(["member", "reviewer", "observer"]);
    }
    expect(leadHint("app:cli:folder-lab")).toBe("Lead is one session; pick one of this App's sessions to lead.");
    expect(leadHint("all")).toBe("Lead is one session; pick one of this pool's sessions to lead.");
  });
});
