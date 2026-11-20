/**
 * The Bot review's H1, in the window: a turn whose memory was forgotten
 * shows that it was, never the words ("I'll remember: …") or a receipt.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { BotTurn } from "./BotTurn";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";

const NOW = new Date(2026, 10, 19, 11, 30).getTime();

function turn(over: Partial<BotTurnInfo>): BotTurnInfo {
  return {
    id: "t1", at_ms: NOW - 60_000, question: "", lang: "en", outcome: "remembered", scope: null, rows: [], total: 0, unknown: null, shortcut: null,
    looked: { reach: "everything", scope: null, memories: 0 }, written: null, memory: null, used: [], live: true, redacted: true, flagged: false,
    ...over,
  };
}

afterEach(cleanup);

describe("a forgotten memory", () => {
  it("shows Forgotten, never its words, even if a stale copy still carried them", () => {
    const stale = { key: "note:x", slot: "note", text: "qzxwv secret words", about: null, about_kind: null, at_ms: NOW, used: 0, versions: 0 };
    render(<BotTurn turn={turn({ memory: stale })} now={NOW} latest />);
    expect(screen.getByTestId("bot-forgotten").textContent).toBe("Forgotten.");
    expect(document.body.textContent).not.toContain("qzxwv");
    expect(screen.queryByTestId("bot-memory-echo")).toBeNull();
  });

  it("a kept memory still echoes", () => {
    const kept = { key: "note:x", slot: "note", text: "review after lunch", about: null, about_kind: null, at_ms: NOW, used: 0, versions: 0 };
    render(<BotTurn turn={turn({ redacted: false, question: "Remember: review after lunch", memory: kept })} now={NOW} latest />);
    expect(screen.getByTestId("bot-memory-echo").textContent).toContain("review after lunch");
  });
});
