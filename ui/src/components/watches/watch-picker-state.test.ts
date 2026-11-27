/**
 * 0.5.2 G34 (RW1 F-RW1-16): the watch picker listed idle sessions as
 * "Working" (their stored status) while the rail said Idle. A session's
 * line in the picker says the rail's state, by its one rule. G32: the
 * form's default condition follows its target.
 */
import { describe, expect, it } from "vitest";
import { instance } from "../../test/fixtures";
import { sessionState } from "../../lib/rail-items";
import { targetBook } from "./watches-view";
import { defaultCode } from "./watch-form-state";

const sources = (sessions: ReturnType<typeof instance>[]) => ({ sessions, apps: [], projects: [{ id: "p-docs", name: "docs" }], pools: [], accounts: [], now: 1_800_000_000_000 });

describe("the watch picker's session states", () => {
  it("says what the rail says: idle after its turn ended, though its process still runs", () => {
    const idle = instance({ id: "s-1", status: "running", project_id: "p-docs", runtime: { kind: "hooks", label: null, beta: false, state: "idle", asks: [], can_steer: false, fell_back: null, error: null, limits: [], cut_off: null } });
    const book = targetBook(sources([idle]));
    const line = book.get("session:s-1")!.line;
    expect(sessionState(idle).word).toBe("Idle");
    expect(line.endsWith("· Idle")).toBe(true);
    expect(line).not.toContain("Working");
    expect(book.get("session:s-1")!.open).toBe(true);
    expect(book.get("project:p-docs")!.line).toContain("1");
  });

  it("an ended session is not open and says how it ended, as the rail does", () => {
    const ended = instance({ id: "s-2", status: "finished", project_id: "p-docs" });
    const option = targetBook(sources([ended])).get("session:s-2")!;
    expect(option.open).toBe(false);
    expect(option.line.endsWith(`· ${sessionState(ended).word}`)).toBe(true);
  });
});

/** 0.5.2 G32 (RW1 H1): the form kept "Its turn ends" for a project with
 * no session; its default follows the target. */
describe("the form's default condition", () => {
  const E = (code: string) => ({ code, label: code, told: code, need: false, param: null });
  const offered = ["finishes", "asks_you", "starts", "hits_limit", "runway_below"].map(E);
  it("is a turn ending for a session or a group with one open, a session starting for a group with none, a limit for an account", () => {
    const open = instance({ id: "s-1", status: "running", project_id: "p-docs" });
    const busy = targetBook(sources([open]));
    expect(defaultCode([{ kind: "session", id: "s-1" }], offered, busy)).toBe("finishes");
    expect(defaultCode([{ kind: "project", id: "p-docs" }], offered, busy)).toBe("finishes");
    const empty = targetBook(sources([]));
    expect(defaultCode([{ kind: "project", id: "p-docs" }], offered, empty)).toBe("starts");
    expect(defaultCode([{ kind: "any" }], offered, empty)).toBe("starts");
    expect(defaultCode([{ kind: "account", id: "a-1" }], offered, empty)).toBe("hits_limit");
    // Only what is on offer: the first of the target's own that is.
    expect(defaultCode([{ kind: "project", id: "p-docs" }], [E("asks_you")], empty)).toBe("asks_you");
    expect(defaultCode([{ kind: "project", id: "p-docs" }], [], empty)).toBeNull();
  });
});
