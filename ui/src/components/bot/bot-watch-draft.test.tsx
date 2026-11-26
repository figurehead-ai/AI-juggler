/**
 * AJ-E3830 (D-149): Bot's watch request opens the Watches form filled in,
 * once, and says so in one line; nothing is added here (the form's own
 * click adds the watch). Something Bot can't watch is said in the
 * request's words; a turn read back keeps its line and its way back.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { BotTurn } from "./BotTurn";
import { onWatchesIntent, takeWatchesIntent } from "../../lib/watches-open";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";

const NOW = 1_800_000_000_000;
const DRAFT: WatchDraftInfo = { targets: [{ kind: "session", id: "s-api" }], matching: "each", conditions: [{ code: "finishes" }], lifetime: { kind: "unsaid" }, evenInQuietHours: false, chatTurn: "t1" };

function turn(over: Partial<BotTurnInfo>): BotTurnInfo {
  return {
    id: "t1", at_ms: NOW - 60_000, question: "Let me know when api finishes.", lang: "en", outcome: "watch_draft", scope: null, rows: [], total: 0, tallies: [], unknown: null, shortcut: null,
    looked: { reach: "everything", scope: null, memories: 0 }, written: null, memory: null, used: [], live: true, redacted: false, flagged: false,
    ...over,
  };
}

afterEach(() => {
  cleanup();
  takeWatchesIntent();
});

describe("Bot fills the Watches form", () => {
  it("opens the form filled in, once, and says to check it there", () => {
    let opened = 0;
    const stop = onWatchesIntent(() => (opened += 1));
    const { rerender } = render(<BotTurn turn={turn({ watch_draft: { kind: "filled", draft: DRAFT } })} now={NOW} latest />);
    rerender(<BotTurn turn={turn({ watch_draft: { kind: "filled", draft: DRAFT } })} now={NOW} latest />);
    expect(opened).toBe(1);
    expect(takeWatchesIntent()).toEqual({ kind: "new", draft: DRAFT });
    expect(screen.getByTestId("bot-watch-draft").textContent).toContain("Check it, then add the watch there.");
    stop();
  });

  it("says what it can't watch in the request's words, with the nearest", () => {
    render(<BotTurn turn={turn({ watch_draft: { kind: "cant_watch", asked: "tests pass", draft: DRAFT } })} now={NOW} latest />);
    expect(screen.getByTestId("bot-watch-draft").textContent).toBe("I can't watch for “tests pass”. The form is open with the nearest I can.");
  });

  it("a turn read back opens nothing by itself, and its button opens the form", () => {
    let opened = 0;
    const stop = onWatchesIntent(() => (opened += 1));
    render(<BotTurn turn={turn({ live: false })} now={NOW} latest={false} />);
    expect(opened).toBe(0);
    expect(screen.getByTestId("bot-watch-draft").textContent).toBe("Bot filled in the Watches form for this.");
    fireEvent.click(screen.getByTestId("bot-watch-draft-open"));
    expect(opened).toBe(1);
    stop();
  });

  it("in German", () => {
    render(<BotTurn turn={turn({ lang: "de", watch_draft: { kind: "form" } })} now={NOW} latest />);
    expect(screen.getByTestId("bot-watch-draft").textContent).toContain("Formular");
  });
});

describe("the drafted watch in plain words", () => {
  it("shows the preview's words, target included, before the user adds it", async () => {
    const ipc = await import("../../lib/ipc/watches");
    const spy = vi.spyOn(ipc, "watchesPreview").mockResolvedValue({ words: "Tell you when “api” finishes. Once.", problem: null });
    render(<BotTurn turn={turn({ live: false, watch_draft: { kind: "filled", draft: DRAFT } })} now={NOW} latest />);
    expect((await screen.findByTestId("bot-watch-draft-words")).textContent).toBe("Tell you when “api” finishes. Once.");
    spy.mockRestore();
  });

  it("shows nothing for a draft with no target (the form asks which)", () => {
    render(<BotTurn turn={turn({ live: false, watch_draft: { kind: "pick", draft: { ...DRAFT, targets: [] } } })} now={NOW} latest />);
    expect(screen.queryByTestId("bot-watch-draft-words")).toBeNull();
  });
});
