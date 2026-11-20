/**
 * SK-2 (the 0.4.6 soak, A5): an Account out of room offered "Continue in
 * another app…" on Home's card for a session whose Account page greyed
 * it (it had no conversation to carry). Home, the bell and the Account
 * page now use one rule (`cantSwitchYet`): a session that can't switch
 * yet is never the one Home's card continues.
 */
import { describe, expect, it } from "vitest";
import { homeNeedsOf } from "./needs-you";
import { primaryWords } from "./home-needs";
import { openOnAccount } from "./handover-focus";
import { cantSwitchYet } from "./switch-not-yet";
import type { NoticeInfo } from "./ipc/notices";
import { instance } from "../test/fixtures";

const quota: NoticeInfo = {
  id: 7,
  kind: "quota",
  needs: true,
  title: "Work has run out of room",
  detail: "0% of its 5-hour window left",
  project_id: null,
  project: null,
  target: { session_id: null, conversation_id: null, account_id: "claude-work", project_id: null, page: null, task_id: null, pool_id: null },
  facts: { window: "5-hour", resets_at_ms: null, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "Work" },
  action: "hand-over",
  meter: 0,
  created_ms: 1,
  unread: true,
  subject: "n:7",
  snoozed: false,
  snoozed_until_ms: null,
  muted: false,
  dismissed: false,
};

describe("SK-2 Home's account card continues only a session that can switch", () => {
  it("passes over a working session with nothing to carry: the card opens its account, as its page greys the switch", () => {
    const fresh = instance({ id: "w1", account_id: "claude-work", status: "running", conversation_id: null });
    expect(cantSwitchYet(fresh)).toBe("It has no conversation to carry over yet");
    expect(openOnAccount("claude-work", [fresh], [])).toBeNull();
    const [need] = homeNeedsOf([quota], [], [], [fresh], null, Date.now());
    expect(need?.sessionId).toBeUndefined();
    expect(primaryWords(need!)).toBe("Open its app");
  });

  it("continues the one that can", () => {
    const fresh = instance({ id: "w1", account_id: "claude-work", status: "running", conversation_id: null, last_activity_ms: 2 });
    const talking = instance({ id: "w2", account_id: "claude-work", status: "running", conversation_id: "conv-2", last_activity_ms: 1 });
    const [need] = homeNeedsOf([quota], [], [], [fresh, talking], null, Date.now());
    expect(need?.sessionId).toBe("w2");
    expect(primaryWords(need!)).toBe("Continue in another app…");
  });
});
