import { describe, expect, it } from "vitest";
import { readDelta } from "./delta-words";

describe("0.5.1 F71: the switch-back update, read back", () => {
  it("keeps every line: the lead, each turn by who said it, the places and the notes", () => {
    const read = readDelta(
      "While you were away, this work continued on Work, then Personal — 3 new turns.\n\n…2 earlier turns, not shown.\nOn Work:\n- [user] fix it\nOn Personal:\n- [assistant] fixed\n- [user] thanks\nOn Team: AI Juggler couldn't find the conversation there, so what happened on it isn't included.\n\nThe project's files are as they were left on Personal. Pick up from here.",
    );
    expect(read.lead).toBe("While you were away, this work continued on Work, then Personal — 3 new turns.");
    expect(read.groups).toEqual([
      { where: "On Work", turns: [{ who: "You", text: "fix it" }] },
      { where: "On Personal", turns: [{ who: "Reply", text: "fixed" }, { who: "You", text: "thanks" }] },
    ]);
    expect(read.notes).toEqual([
      "…2 earlier turns, not shown.",
      "On Team: AI Juggler couldn't find the conversation there, so what happened on it isn't included.",
      "The project's files are as they were left on Personal. Pick up from here.",
    ]);
  });
});
