/**
 * AJ-E3505 req. 4: the rail and the Floor read a session the same way.
 * `fixtures/session-states.json` holds the cases; the Floor's Rust
 * (`crates/juggler-floor/src/state.rs`) runs the same file.
 */
import { describe, expect, it } from "vitest";

import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ConversationInfo } from "./ipc/conversations";
import { conversationItem, needWord, sessionItem } from "./rail-items";
import cases from "./fixtures/session-states.json";

const NOW = new Date(2026, 10, 4, 12, 0, 0).getTime();

function own(c: (typeof cases.own)[number]): InstanceInfo {
  return {
    id: "s", provider: "claude-code", account_id: "a1", account_label: "Work", kind: "terminal", project_id: null, cwd: "/w/app",
    started_at_ms: NOW - 60_000, last_activity_ms: NOW, status: c.status, needs_attention: c.needs_attention,
    runtime: c.runtime === null ? null : { state: c.runtime },
  } as unknown as InstanceInfo;
}

function other(c: (typeof cases.other)[number]): ConversationInfo {
  return {
    id: "c", tool: "claude-code", account_id: "a1", account_label: "Work", project_id: null, cwd: "/w/app", title: "Talk", places: [],
    started_ms: NOW - 60_000, last_activity_ms: NOW, live: c.live, state: c.state, probably: c.probably, viewed_ms: c.looked ? NOW + 1 : null,
    background: false, inactive: false, subtask_count: null,
  } as unknown as ConversationInfo;
}

describe("one session, one state on the rail and the Floor (AJ-E3505)", () => {
  it.each(cases.own)("AI Juggler's own: $status / $needs_attention / $runtime, snoozed $snoozed → $rail, $word", (c) => {
    const item = sessionItem(own(c), () => null, c.snoozed);
    expect([item.state, needWord(item)]).toEqual([c.rail, c.word]);
  });

  it.each(cases.other)("another app's: live $live, $state, looked $looked → $rail, $word", (c) => {
    const item = conversationItem(other(c), () => null, null);
    expect([item.state, needWord(item)]).toEqual([c.rail, c.word]);
  });
});
