/**
 * 0.5.2 G31 (RW1 F-RW1-12): the chip's scope goes with its ask (× before
 * asking widens it at once), and New topic starts empty: the earlier
 * turns, their cards and Set buttons with them, come back only through
 * Earlier, read back as kept.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AiJobInfo } from "../../lib/ipc/ai";
import { BOT_NOW, STATES } from "../../preview/bot";
import { ask, botState, newTopic, openBot, resetBot } from "../../lib/bot-store";

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

describe("a topic and a scope of their own", () => {
  /** 0.5.2 G31 (RW1 F-RW1-12: two identical Set reminder buttons): New
   * topic starts empty; the earlier turns, their cards with them, come
   * back only through Earlier, read back as kept. */
  it("New topic starts empty, Earlier bringing back what came before", async () => {
    seed("empty");
    panel();
    await act(async () => {
      await ask("What needs me?");
    });
    expect(screen.getAllByTestId("bot-turn")).toHaveLength(1);
    await act(async () => {
      await newTopic();
    });
    expect(screen.queryAllByTestId("bot-turn")).toHaveLength(0);
    expect(botState().scope).toBeNull();
    await act(async () => {
      fireEvent.click(screen.getByTestId("bot-earlier"));
    });
    await waitFor(() => expect(screen.getAllByTestId("bot-turn").length).toBeGreaterThan(0));
  });

  it("× asks about everything before it is asked", async () => {
    seed("lookup");
    act(() => openBot({ kind: "session", id: "s-api", name: "api", sessions: [] }));
    render(<BotPanelBody onClear={() => undefined} onMemories={() => undefined} />);
    fireEvent.click(screen.getByTestId("bot-scope-x"));
    expect(screen.queryByTestId("bot-scope-chip")).toBeNull();
    await act(async () => {
      await ask("Anything stuck?");
    });
    const widened: unknown = window.__AIJ_BOT__?.calls?.filter(([c]) => c === "bot_ask").at(-1)?.[1];
    expect(Reflect.get(Object(widened), "scope")).toBeNull();
  });
});
