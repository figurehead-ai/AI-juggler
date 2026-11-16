/**
 * P1 step 8 (the rc29 walk): a running Claude app session under "Sessions
 * in other apps" showed its Add with no reason; the row says how it takes
 * part before it's added (AJ-E3013: through its app's connector).
 */
import { describe, expect, it } from "vitest";
import { draftOf, sessionCandidates } from "./new-pool";
import { templateById } from "./pool-templates";
import type { ConversationInfo } from "../types/generated/ConversationInfo";

const running: ConversationInfo = {
  id: "c1",
  tool: "claude-code",
  account_id: "acc",
  account_label: "Work",
  project_id: "web",
  cwd: "/Users/you/lab/web",
  title: "Disk space",
  git_branch: null,
  started_ms: 1,
  last_activity_ms: 5,
  tokens: null,
  live: true,
  session_id: null,
  places: [{ app: "Claude", shown: "the Claude app", app_id: null, profile: null, instance: null, pid: 1, kind: "desktop", app_pid: null, label: "Claude" }],
  pid: null,
  state: "working",
  probably: false,
  viewed_ms: null,
  background: false,
  inactive: false,
  subtask_count: null,
};

describe("a session another app runs, offered to a pool", () => {
  it("says how it takes part before it's added: its app's connector (AJ-E3013), no Continue fix", () => {
    const rows = sessionCandidates({
      draft: draftOf(templateById("writer-reviewer"), "web"),
      sessions: [],
      conversations: [running],
      pools: null,
      tools: [],
      projectName: (id) => id ?? "no project",
      nameOf: () => "",
      now: 10,
    });
    const row = rows.find((one) => one.ref === "conversation:acc/c1");
    expect(row?.reach).toBe("watched");
    expect(row?.blocked).toBeNull();
    expect(row?.note).toBe("Runs in the Claude app: with AI Juggler's connector in its app (added with your OK) it talks in the pool; without it, it's Watched.");
    expect(row?.fix).toBeUndefined();
  });
});
