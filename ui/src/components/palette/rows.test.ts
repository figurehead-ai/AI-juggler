/**
 * The 0.4 design review on ⌘K's rows: typed, the groups follow how well
 * their best row matches (16), so an exact project name leads a loose
 * "Go to Privacy"; untyped, a session under Recent isn't listed again
 * under Open now (52).
 */
import { describe, expect, it, vi } from "vitest";
import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";
import type { PaletteCommand } from "../../lib/palette-commands";
import { parseQuery } from "../../lib/rail-view";
import { buildGroups, type PaletteInputs } from "./rows";

const NOW = Date.now();
const session = (id: string, label: string): InstanceInfo =>
  ({ id, provider: "claude-code", account_id: "a1", account_label: "Work Max", status: "running", kind: "terminal", git_branch: null, cwd: "/w/alpha", project_id: null, conversation_id: null, quota: null, label, last_activity_ms: NOW }) as unknown as InstanceInfo;
const project = (id: string, name: string): ProjectInfo => ({ id, name, root: `/w/${name}`, missing: false, last_activity_ms: NOW }) as unknown as ProjectInfo;
const command = (id: string, label: string): PaletteCommand => ({ id, label, icon: "go", run: vi.fn() }) as unknown as PaletteCommand;

function inputs(typed: string, over: Partial<PaletteInputs> = {}): PaletteInputs {
  return {
    typed,
    query: parseQuery(typed),
    actions: [],
    selectedHeading: null,
    commands: [command("go:privacy", "Go to Privacy"), command("new-session", "New session…")],
    app: { searchHistory: vi.fn() } as unknown as PaletteInputs["app"],
    sessions: [session("s1", "Write release notes")],
    conversations: [],
    remoteMachines: [],
    projects: [project("p1", "payments-api")],
    apps: [],
    accounts: [],
    recents: [],
    leftovers: 0,
    unread: 0,
    settings: [],
    ...over,
  };
}

describe("⌘K ranks by match quality across groups (0.4 design review, 16)", () => {
  it("puts the exact match above a loose action", () => {
    const groups = buildGroups(inputs("pay")).map((group) => group.key);
    expect(groups).toContain("actions");
    expect(groups.indexOf("projects")).toBeLessThan(groups.indexOf("actions"));
    expect(groups.at(-1)).toBe("search");
  });

  it("keeps an action group first when it matches at least as well", () => {
    const groups = buildGroups(inputs("new", { projects: [project("p1", "newsletter")] })).map((group) => group.key);
    expect(groups.slice(0, 2)).toEqual(["actions", "projects"]);
  });
});

describe("⌘K lists a session once (0.4 design review, 52)", () => {
  it("leaves one under Recent out of Open now", () => {
    const sessions = [session("s1", "feat/read-model"), session("s2", "Write release notes")];
    const groups = buildGroups(inputs("", { sessions, recents: [{ kind: "session", id: "s1" }] }));
    const ids = (key: string) => groups.find((group) => group.key === key)?.rows.map((row) => row.id) ?? [];
    expect(ids("recent")).toEqual(["s1"]);
    expect(ids("open")).toEqual(["s2"]);
  });
});
