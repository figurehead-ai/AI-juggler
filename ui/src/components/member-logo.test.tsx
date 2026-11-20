/**
 * D-126: a pool member's logo, one function and one component wherever a
 * member is drawn (New pool, the Pools card, the diagram's planned team
 * and its session nodes).
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { AppModel } from "../lib/apps";
import { MOCK_SESSIONS } from "../preview/fixtures";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { logoOf, MemberLogo } from "./MemberLogo";

afterEach(cleanup);

describe("a member's logo", () => {
  const app = (id: string, kind: AppModel["kind"], tool: string, appId: string, name: string): AppModel => ({ id, kind, tool, appId, app: name, profile: null, name, defaultName: name, named: false, railKey: id, folder: null, accountId: null, place: null, slot: 0 });
  const apps = [app("cli:work", "cli", "claude-code", "claude-app", "Claude Code"), app("app:codex-app:/p", "desktop", "codex", "codex-app", "Codex")];
  const sessions: InstanceInfo[] = [{ ...MOCK_SESSIONS[0]!, id: "s1", provider: "claude-code", kind: "terminal", app_id: null }];

  it("is its tool's logo for an App or a session (a CLI with its badge), a tile for the rest", () => {
    expect(logoOf("app:cli:work", { apps, sessions })).toEqual({ kind: "tool", tool: "claude-code", look: "cli", name: "Claude Code" });
    expect(logoOf("app:app:codex-app:/p", { apps, sessions })).toMatchObject({ kind: "tool", tool: "codex", look: "desktop", appId: "codex-app" });
    expect(logoOf("session:s1", { apps, sessions })).toMatchObject({ kind: "tool", tool: "claude-code", look: "cli" });
    for (const ref of ["all", "account:a", "mac:m", "session:gone"]) expect(logoOf(ref, { apps, sessions }).kind).toBe("tile");
  });

  it("a desktop session whose App isn't listed still looks like its desktop app (the diagram's nodes; was a CLI's)", () => {
    const desk: InstanceInfo = { ...sessions[0]!, id: "d1", provider: "codex", kind: "desktop" };
    expect(logoOf("session:d1", { apps: [], sessions: [desk] })).toEqual({ kind: "tool", tool: "codex", look: "desktop", name: "codex" });
  });

  it("carries the lead's crown or a reviewer's shield at its corner, and nothing for a member", () => {
    const view = render(<MemberLogo spec={logoOf("all", { apps, sessions })} badge="lead" />);
    expect(view.getByTestId("member-logo-role").dataset.role).toBe("lead");
    view.rerender(<MemberLogo spec={logoOf("all", { apps, sessions })} badge="reviewer" />);
    expect(view.getByTestId("member-logo-role").dataset.role).toBe("reviewer");
    view.rerender(<MemberLogo spec={logoOf("all", { apps, sessions })} badge="member" />);
    expect(view.queryByTestId("member-logo-role")).toBeNull();
  });
});
