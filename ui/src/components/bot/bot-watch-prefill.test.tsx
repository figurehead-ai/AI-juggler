/**
 * AJ-E3830 (0.5.2) — Bot pre-fills the Watches form; the user saves. The
 * turn opens the form once with what code verified and says, in one line,
 * what it left; with every field verified and the Watches Beta on, Bot's
 * confirm card instead (nothing written until Watch; Change… opens the
 * form filled in; Cancel writes nothing).
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { BotTurn } from "./BotTurn";
import { onWatchesIntent, takeWatchesIntent } from "../../lib/watches-open";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";
import type { WatchPrefill } from "../../types/generated/WatchPrefill";

const NOW = 1_800_000_000_000;

function prefill(over: Partial<WatchPrefill>): WatchPrefill {
  return {
    draft: { targets: [{ kind: "session", id: "s-api" }], matching: "each", conditions: [{ code: "finishes" }], lifetime: { kind: "unsaid" }, evenInQuietHours: false, chatTurn: "t1" },
    left: [],
    why: "verified",
    asked: null,
    nearest: null,
    card: false,
    ...over,
  };
}

function turn(p: WatchPrefill, over: Partial<BotTurnInfo> = {}): BotTurnInfo {
  return {
    id: "t1", at_ms: NOW - 60_000, question: "Let me know when api finishes.", lang: "en", outcome: "watch_draft", scope: null, rows: [], total: 0, tallies: [], unknown: null, shortcut: null,
    looked: { reach: "everything", scope: null, memories: 0 }, written: null, memory: null, used: [], live: true, redacted: false, flagged: false,
    watch_draft: { kind: "prefilled", prefill: p },
    ...over,
  };
}

afterEach(() => {
  cleanup();
  takeWatchesIntent();
  vi.restoreAllMocks();
});

describe("Bot pre-fills the Watches form", () => {
  it("opens the form once with the pre-fill and names what it left", () => {
    let opened = 0;
    const stop = onWatchesIntent(() => (opened += 1));
    const p = prefill({ left: ["lifetime"], why: "unread", asked: "bis ich es entferne" });
    const { rerender } = render(<BotTurn turn={turn(p)} now={NOW} latest />);
    rerender(<BotTurn turn={turn(p)} now={NOW} latest />);
    expect(opened).toBe(1);
    expect(takeWatchesIntent()).toEqual({ kind: "new", prefill: p });
    expect(screen.getByTestId("bot-watch-draft").textContent).toBe("I couldn't read “bis ich es entferne”. Left for you to pick: how long. The Watches form is open.");
    // A partial draft's echo would say a default as if it were asked: none shown.
    expect(screen.queryByTestId("bot-watch-draft-words")).toBeNull();
    stop();
  });

  it("says what it can't watch, and that the nearest is marked", () => {
    render(<BotTurn turn={turn(prefill({ draft: { ...prefill({}).draft, conditions: [] }, left: ["condition"], why: "cant_watch", asked: "tests pass", nearest: "finishes" }))} now={NOW} latest />);
    expect(screen.getByTestId("bot-watch-draft").textContent).toBe("I can't watch for “tests pass”. The Watches form is open with what I could fill in. The nearest it can watch is marked.");
  });

  it("in German, from the same fields", () => {
    render(<BotTurn turn={turn(prefill({ left: ["lifetime", "quiet_hours"], why: "unread", asked: "yo" }), { lang: "de" })} now={NOW} latest />);
    expect(screen.getByTestId("bot-watch-draft").textContent).toBe("„yo“ konnte ich nicht lesen. Noch zu wählen: wie lange und die Ruhezeiten. Das Formular ist offen.");
  });

  it("with nothing it could fill, says so", () => {
    render(<BotTurn turn={turn(prefill({ draft: { ...prefill({}).draft, targets: [], conditions: [] }, left: ["target", "condition", "lifetime", "quiet_hours"], why: "unread", asked: "x" }))} now={NOW} latest />);
    expect(screen.getByTestId("bot-watch-draft").textContent).toBe("I couldn't fill the form from your words. The Watches form is open.");
  });
});

describe("Bot's confirm card (Watches Beta)", () => {
  it("shows the code's words, opens no form, and writes only on Watch", async () => {
    const ipc = await import("../../lib/ipc/watches");
    vi.spyOn(ipc, "watchesPreview").mockResolvedValue({ words: "Tell you when “api” finishes. Once.", problem: null });
    const create = vi.spyOn(ipc, "watchesCreate").mockResolvedValue({ words: "“api” finishes. Once." });
    let opened = 0;
    const stop = onWatchesIntent(() => (opened += 1));
    render(<BotTurn turn={turn(prefill({ card: true }))} now={NOW} latest />);
    expect(screen.getByTestId("bot-watch-draft").textContent).toBe("Here is the watch. Check it, then click Watch.");
    expect((await screen.findByTestId("bot-watch-echo")).textContent).toBe("Tell you when “api” finishes. Once.");
    expect(screen.getByTestId("bot-watch-confirm-suggested").textContent).toContain("How long wasn't said");
    expect(opened).toBe(0);
    expect(create).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId("bot-watch-confirm-confirm"));
    expect((await screen.findByTestId("bot-watch-done")).textContent).toBe("Watching: “api” finishes. Once.");
    expect(create).toHaveBeenCalledTimes(1);
    stop();
  });

  it("Change… opens the form filled in; Cancel writes nothing", async () => {
    const ipc = await import("../../lib/ipc/watches");
    vi.spyOn(ipc, "watchesPreview").mockResolvedValue({ words: "Tell you when “api” finishes. Today.", problem: null });
    const create = vi.spyOn(ipc, "watchesCreate");
    const p = prefill({ card: true, draft: { ...prefill({}).draft, lifetime: { kind: "today" } } });
    render(<BotTurn turn={turn(p)} now={NOW} latest />);
    await screen.findByText("Tell you when “api” finishes. Today.");
    expect(screen.queryByTestId("bot-watch-confirm-suggested")).toBeNull();
    fireEvent.click(screen.getByTestId("bot-watch-confirm-change"));
    expect(takeWatchesIntent()).toEqual({ kind: "new", prefill: p });
    fireEvent.click(screen.getByTestId("bot-watch-confirm-cancel"));
    expect(screen.getByTestId("bot-watch-confirm-closed")).toBeTruthy();
    expect(create).not.toHaveBeenCalled();
  });

  it("a problem the form's check finds keeps Watch off", async () => {
    const ipc = await import("../../lib/ipc/watches");
    vi.spyOn(ipc, "watchesPreview").mockResolvedValue({ words: "Tell you when “api” finishes.", problem: "50 watches is the most." });
    render(<BotTurn turn={turn(prefill({ card: true }))} now={NOW} latest />);
    await screen.findByText("50 watches is the most.");
    expect(screen.getByTestId("bot-watch-confirm-confirm").hasAttribute("disabled")).toBe(true);
  });
});
