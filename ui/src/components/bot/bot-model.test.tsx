/**
 * AJ-E3825 (D-144 in full, D-148): Bot through the real model, in the
 * window. A general answer is labelled as Bot's own and shows no record
 * chips, with the scope line when off-topic; a failure is one plain line
 * with its fix; each answer has a calm details line (an estimate says
 * "≈ … est."); the header names the model and its state; the topic's
 * totals open context and memory breakdowns.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BotDetailsLine, BotFailedAnswer, BotGeneralAnswer } from "./BotAnswerParts";
import { BotUsageBar } from "./BotUsageBar";
import { BotEngineBadge } from "./BotHead";
import { GENERAL_LABEL, SCOPE_LINE, countWords, engineStateWords, failureWords, startingLine } from "../../lib/bot-engine-words";
import { refreshEngine, resetEngineView } from "../../lib/bot-engine";
import { newTopic, resetBot } from "../../lib/bot-store";
import type { AiEngineStatusInfo } from "../../lib/ipc/ai-engine";
import type { BotDetails } from "../../types/generated/BotDetails";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";
import type { AiFailure } from "../../types/generated/AiFailure";

vi.mock("./bot-nav", () => ({ useBotNav: () => ({ openSettings: vi.fn(), openRow: vi.fn(), openShortcut: vi.fn() }) }));

const details: BotDetails = {
  model_name: "Apple's on-device model",
  tokens_in: { n: 812, estimated: false },
  tokens_out: { n: 18, estimated: true },
  ms: 1400,
  records: 2,
  context: {
    window: 4096,
    used: { n: 812, estimated: false },
    answer_room: 220,
    parts: [
      { part: "instructions", tokens: { n: 120, estimated: true } },
      { part: "memory", tokens: { n: 10, estimated: true } },
      { part: "records", tokens: { n: 500, estimated: true } },
      { part: "history", tokens: { n: 150, estimated: true } },
      { part: "question", tokens: { n: 12, estimated: true } },
    ],
  },
  memory: [
    { kind: "preferences", n: 1, tokens: { n: 6, estimated: true } },
    { kind: "notes", n: 0, tokens: { n: 0, estimated: true } },
    { kind: "asked", n: 2, tokens: { n: 150, estimated: true } },
    { kind: "watches", n: 0, tokens: { n: 0, estimated: false } },
  ],
};

function turn(over: Partial<BotTurnInfo>): BotTurnInfo {
  return {
    id: "t1",
    at_ms: 0,
    question: "what is 3+5",
    lang: "en",
    outcome: "general",
    scope: null,
    rows: [],
    total: 0,
    tallies: [],
    unknown: null,
    shortcut: null,
    looked: { reach: "everything", scope: null, memories: 0 },
    written: null,
    memory: null,
    used: [],
    live: true,
    redacted: false,
    flagged: false,
    general: null,
    failure: null,
    details: null,
    ...over,
  };
}

const status = (over: Partial<AiEngineStatusInfo>): AiEngineStatusInfo => ({
  state: "running",
  failure: null,
  model_id: "apple-on-device",
  model_name: "Apple's on-device model",
  short_name: "Apple on-device",
  engine: "apple_builtin",
  version: "26.1",
  context_tokens: 4096,
  started_ms: 1,
  answered_ms: null,
  today: { answers: 0, tokens_in: { n: 0, estimated: false }, tokens_out: { n: 0, estimated: false } },
  ...over,
});

beforeEach(() => {
  resetBot();
  resetEngineView();
});
afterEach(() => {
  cleanup();
  delete window.__AIJ_BOT__;
  delete window.__AIJ_AI_ENGINE__;
});

describe("a general answer (D-148)", () => {
  it("is labelled as Bot's own, claims no records, and says what Bot is for when off-topic", () => {
    render(<BotGeneralAnswer turn={turn({ general: { text: "8.", model_name: "Apple's on-device model", off_topic: true } })} />);
    expect(screen.getByTestId("bot-general-label").textContent).toBe(GENERAL_LABEL);
    expect(screen.getByText("8.")).toBeTruthy();
    expect(screen.getByTestId("bot-scope-line").textContent).toBe(SCOPE_LINE);
    expect(screen.queryByTestId("bot-rows")).toBeNull();
  });

  it("has no scope line when it's about Bot itself", () => {
    render(<BotGeneralAnswer turn={turn({ general: { text: "I run on Apple's on-device model.", model_name: "m", off_topic: false } })} />);
    expect(screen.queryByTestId("bot-scope-line")).toBeNull();
  });
});

describe("a failure", () => {
  const kinds: AiFailure[] = ["off", "not_downloaded", "helper_didnt_start", "timed_out", "refused", "too_long", "not_this_mac", "unknown"];
  it.each(kinds)("%s is one plain line with its fix and Try again", (kind) => {
    render(<BotFailedAnswer turn={turn({ outcome: "failed", failure: kind })} model="Apple's on-device model" />);
    const line = screen.getByTestId("bot-failed");
    const words = failureWords(kind, "Apple's on-device model");
    expect(line.textContent).toContain(words.line);
    expect(line.textContent).toContain(words.fix);
    expect(line.textContent).not.toMatch(/nothing matches|I found nothing/i);
    expect(screen.getByTestId("bot-try-again")).toBeTruthy();
  });

  it("Apple Intelligence off names the switch", () => {
    expect(failureWords("off", "m").fix).toContain("System Settings › Apple Intelligence & Siri");
  });
});

describe("the details line", () => {
  it("names the model, tokens in and out (an estimate labelled), the time and the records", () => {
    render(<BotDetailsLine turn={turn({ outcome: "answered", details })} />);
    expect(screen.getByTestId("bot-details").textContent).toBe("Apple's on-device model · 812 in · ≈ 18 est. out · 1.4 s · 2 records");
  });

  it("a general answer lists no records", () => {
    render(<BotDetailsLine turn={turn({ outcome: "general", details })} />);
    expect(screen.getByTestId("bot-details").textContent).not.toContain("record");
  });

  it("counts read short and say when estimated", () => {
    expect(countWords({ n: 3100, estimated: false })).toBe("3.1k");
    expect(countWords({ n: 820, estimated: true })).toBe("≈ 820 est.");
  });
});

describe("the header", () => {
  it("names the model and its live state", async () => {
    window.__AIJ_AI_ENGINE__ = status({ state: "running" });
    await act(async () => refreshEngine());
    render(<BotEngineBadge beta />);
    expect(screen.getByTestId("bot-engine").textContent).toContain("Apple on-device · Running");
    expect(screen.getByTestId("bot-engine").textContent).toContain("Beta");
  });

  it("says Starting…, Idle and Failed: why", () => {
    expect(engineStateWords(status({ state: "starting" }), false)).toBe("Starting…");
    expect(engineStateWords(status({ state: "idle" }), false)).toBe("Idle");
    expect(engineStateWords(status({ state: "failed", failure: "off" }), false)).toBe("Failed: Apple Intelligence is off");
    expect(startingLine(status({}))).toBe("Starting Apple's on-device model…");
  });
});

describe("the topic's totals", () => {
  it("show tokens this topic and open context and memory breakdowns", async () => {
    window.__AIJ_BOT__ = {
      usage: { answers: 2, tokens_in: { n: 1600, estimated: false }, tokens_out: { n: 40, estimated: true }, latest: details },
    };
    const memories = vi.fn();
    await act(async () => newTopic());
    render(<BotUsageBar onMemories={memories} />);
    await act(async () => {
      await Promise.resolve();
    });
    expect(screen.getByTestId("bot-topic-tokens").textContent).toBe("This topic ≈ 1.6k est. tokens");
    const context = screen.getByTestId("bot-context");
    expect(context.textContent).toBe("context 812 / 4.1k");
    fireEvent.click(context);
    const breakdown = screen.getByTestId("bot-context-breakdown").textContent ?? "";
    for (const label of ["Instructions", "Bot’s memory", "Records", "Conversation history", "Your question", "Room for the answer"]) {
      expect(breakdown).toContain(label);
    }
    expect(breakdown).toContain("≈ 500 est.");
    fireEvent.click(screen.getByTestId("bot-memory"));
    const memory = screen.getByTestId("bot-memory-breakdown").textContent ?? "";
    expect(memory).toContain("Preferences and names");
    expect(memory).toContain("Earlier questions and answers");
    fireEvent.click(screen.getByText("Manage what Bot remembers"));
    expect(memories).toHaveBeenCalled();
  });

  it("shows nothing before the first answer", async () => {
    render(<BotUsageBar onMemories={() => undefined} />);
    await act(async () => {
      await Promise.resolve();
    });
    expect(screen.queryByTestId("bot-usage")).toBeNull();
  });
});
