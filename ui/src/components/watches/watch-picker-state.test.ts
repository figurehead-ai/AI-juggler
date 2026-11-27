/**
 * 0.5.2 G34 (RW1 F-RW1-16): the watch picker listed idle sessions as
 * "Working" (their stored status) while the rail said Idle. A session's
 * line in the picker says the rail's state, by its one rule. G32: the
 * form's default condition follows its target.
 */
import { describe, expect, it } from "vitest";
import { instance } from "../../test/fixtures";
import { sessionState } from "../../lib/rail-items";
import { pickerLists, targetBook } from "./watches-view";
import { startersFor } from "./WatchesEmpty";
import type { ConversationInfo } from "../../types/generated/ConversationInfo";
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

/** 0.5.2 G45 (RW1 W1): a session started outside AI Juggler (in the
 * user's own terminal, on an App they added) is in Open now, so the
 * picker finds it by its name, its project and App count it as open,
 * and it is a target of its own by its key; a closed one is listed but
 * not open, and one put away isn't listed. */
describe("the watch picker and a session AI Juggler didn't start", () => {
  const T = 1_800_000_000_000;
  const outside = (over: Partial<ConversationInfo> = {}): ConversationInfo => ({ id: "c-1", tool: "claude-code", account_id: "acct-work", account_label: "work", project_id: "p-docs", cwd: "/lab/docs", title: "Update the summary cell in the report notebook", git_branch: null, app: "cli:acct-work", started_ms: T - 120_000, last_activity_ms: T - 60_000, tokens: null, live: true, session_id: null, places: [], pid: 4242, state: "idle", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over });
  const apps = [{ id: "cli:acct-work", kind: "cli" as const, name: "cc-work", tool: "claude-code", folder: { id: "acct-work" } }];
  const book = (conversations: ConversationInfo[]) => targetBook({ ...sources([]), apps, conversations });

  it("finds it by its name, open and Idle, and counts it in its project, App and Any", () => {
    const b = book([outside()]);
    const option = b.get("session:conversation:acct-work:c-1")!;
    expect(option.name).toBe("Update the summary cell in the report notebook");
    expect(option.open).toBe(true);
    expect(option.line).toBe("cc-work · docs · Idle");
    const { matches, openNow } = pickerLists(b, "summary cell");
    expect(matches!.map((o) => o.key)).toContain("session:conversation:acct-work:c-1");
    expect(openNow.map((o) => o.key)).toEqual(["session:conversation:acct-work:c-1"]);
    expect(b.get("project:p-docs")!.openSessions).toBe(1);
    expect(b.get("project:p-docs")!.line).toContain("1");
    expect(b.get("app:cli:acct-work")!.openSessions).toBe(1);
    expect(b.get("any")!.openSessions).toBe(1);
  });

  it("a closed one is listed, not open; one an AI Juggler session runs or one put away isn't its own", () => {
    const b = book([
      outside({ id: "c-2", live: false, state: null }),
      outside({ id: "c-3", session_id: "s-9" }),
      outside({ id: "c-4", archived_here: true }),
    ]);
    expect(b.get("session:conversation:acct-work:c-2")!.open).toBe(false);
    expect(b.has("session:conversation:acct-work:c-3")).toBe(false);
    expect(b.has("session:conversation:acct-work:c-4")).toBe(false);
    expect(b.get("project:p-docs")!.openSessions).toBe(0);
  });
});

/** 0.5.2 G46 (RW1 W2): the Finishes starter said "· working now" for an
 * idle session; it says the rail's word, as the picker does. */
describe("the Watches starter card", () => {
  it("says an idle session's own state", async () => {
    const idle = instance({ id: "s-1", label: "Split the cart total", status: "running", project_id: "p-docs", runtime: { kind: "hooks", label: null, beta: false, state: "idle", asks: [], can_steer: false, fell_back: null, error: null, limits: [], cut_off: null } });
    const option = targetBook(sources([idle])).get("session:s-1")!;
    const offered = [{ code: "finishes", label: "", told: "", need: false, param: null }];
    const [finishes] = await startersFor([option], null, async () => offered);
    expect(finishes!.line).toBe(option.line);
    expect(finishes!.line.endsWith("· Idle")).toBe(true);
    expect(finishes!.line).not.toContain("working");
  });
});

/** 0.5.2 G54 (RW1 W5): the starter card's accessible text read
 * "…notebook”finishes its turn": the space after a name sat at the start
 * of an element, which the accessible text drops. It is a bare text node
 * beside the name. */
describe("an echo's names and the words beside them", () => {
  it("keep their spaces outside any element", async () => {
    const { render } = await import("@testing-library/react");
    const { createElement } = await import("react");
    const { EchoHead } = await import("./WatchParts");
    const spans = [
      { text: "Tell you when ", name: false },
      { text: "“Update the summary cell in the report notebook”", name: true },
      { text: " finishes its turn.", name: false },
    ];
    const { container } = render(createElement("p", null, createElement(EchoHead, { spans })));
    const p = container.querySelector("p")!;
    expect(p.textContent).toBe("Tell you when “Update the summary cell in the report notebook” finishes its turn.");
    for (const element of p.querySelectorAll("*")) {
      expect(element.textContent).toBe(element.textContent.trim());
    }
    expect(p.childNodes[2]!.nodeType).toBe(Node.TEXT_NODE);
    expect(p.childNodes[2]!.textContent!.startsWith(" ")).toBe(true);
  });
});
