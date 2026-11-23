/**
 * AJ-E1548 #2 (F9): "This is wrong" on Bot's written answer does
 * something real: the answer collapses to what marking it does (Bot won't
 * use it in this conversation; it is never sent back to the AI), with
 * Undo and Ask again, which asks the same question, naming the answer
 * marked wrong so the backend adds its note.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AiJobInfo } from "../../lib/ipc/ai";
import { BOT_NOW, STATES } from "../../preview/bot";
import { openBot, resetBot } from "../../lib/bot-store";

let job: AiJobInfo | null = null;
const ready = (info: AiJobInfo | null | undefined) => info?.state === "ready" || info?.state === "waiting";
vi.mock("../../lib/ai-availability", () => ({ useAiJob: () => job, useAiJobs: () => (job ? [job] : []), isShown: ready, useBotReady: () => ready(job) }));
vi.mock("../../lib/ai-models", () => ({ useModelsCatalog: () => null, useModelsInstalled: () => null, refreshModels: () => undefined }));

const { BotPanelBody } = await import("./BotDock");

function seed(name: keyof typeof STATES): void {
  const state = STATES[name]!;
  window.__AIJ_BOT__ = structuredClone(state.bot);
  window.__AIJ_WATCH_VIEW__ = state.watches ? structuredClone(state.watches) : {};
  job = state.jobs.find((info) => info.job === "chat") ?? null;
}

function panel(): void {
  act(() => openBot());
  render(<BotPanelBody onClear={() => undefined} onMemories={() => undefined} />);
}

beforeEach(() => {
  resetBot();
  vi.spyOn(Date, "now").mockReturnValue(BOT_NOW);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete window.__AIJ_BOT__;
  delete window.__AIJ_WATCH_VIEW__;
});

describe("This is wrong", () => {
  it("AJ-E1548 #2: This is wrong collapses the answer to what it does, with Undo and Ask again (the same question, naming the answer marked wrong)", async () => {
    seed("beta");
    panel();
    await screen.findByTestId("bot-written");
    const asked = window.__AIJ_BOT__!.turns![0]!;
    fireEvent.click(screen.getByTestId("bot-how-made"));
    fireEvent.click(screen.getByTestId("bot-wrong"));
    const marked = await screen.findByTestId("bot-wrong-marked");
    expect(marked.textContent).toBe("Marked wrong: Bot won’t use it in this conversation.");
    expect(screen.getByTestId("bot-written").querySelectorAll("[aria-label^='record ']")).toHaveLength(0);
    // the records it rests on stay below
    expect(screen.getAllByTestId("bot-row")).toHaveLength(2);
    expect(window.__AIJ_BOT__?.calls).toContainEqual(["bot_wrong", { turn: asked.id, wrong: true }]);
    // Undo brings the answer back
    fireEvent.click(screen.getByTestId("bot-wrong-undo"));
    await waitFor(() => expect(screen.queryByTestId("bot-wrong-marked")).toBeNull());
    expect(window.__AIJ_BOT__?.calls).toContainEqual(["bot_wrong", { turn: asked.id, wrong: false }]);
    expect(screen.getByTestId("bot-written").querySelectorAll("[aria-label^='record ']")).toHaveLength(2);
    // marked again (How this was made is still open), Ask again asks the
    // same question with its chip, naming it
    fireEvent.click(screen.getByTestId("bot-wrong"));
    fireEvent.click(await screen.findByTestId("bot-ask-again"));
    await waitFor(() => expect(window.__AIJ_BOT__?.calls).toContainEqual(["bot_ask", { question: asked.question, scope: asked.scope, again: asked.id }]));
  });
});
