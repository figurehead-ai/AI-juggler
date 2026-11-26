/**
 * AJ-E3832 G9: a memory Bot answered from is a record of its own: its row
 * says it is what the user told Bot, and its chip opens What Bot remembers
 * on that memory.
 */
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AiJobInfo } from "../../lib/ipc/ai";
import { BOT_NOW, R, STATES } from "../../preview/bot";
import { rowPlace, stateSub } from "../../lib/bot-words";
import { resetBot } from "../../lib/bot-store";
import { focusMemory } from "../../lib/bot-memory-focus";
import type { BotRowInfo } from "../../types/generated/BotRowInfo";

let job: AiJobInfo | null = null;
const ready = (info: AiJobInfo | null | undefined) => info?.state === "ready" || info?.state === "waiting";
vi.mock("../../lib/ai-availability", () => ({ useAiJob: () => job, useAiJobs: () => (job ? [job] : []), isShown: ready, useBotReady: () => ready(job) }));
vi.mock("../../lib/ai-models", () => ({ useModelsCatalog: () => null, useModelsInstalled: () => null, refreshModels: () => undefined }));

const { BotPage } = await import("./BotPage");

beforeEach(() => {
  resetBot();
  vi.spyOn(Date, "now").mockReturnValue(BOT_NOW);
  const full = STATES.full!;
  window.__AIJ_BOT__ = structuredClone(full.bot);
  job = full.jobs.find((info) => info.job === "chat") ?? null;
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete window.__AIJ_BOT__;
});

describe("a memory as a record", () => {
  it("says it is what the user told Bot", () => {
    const row: BotRowInfo = { ...R.web, kind: "memory", id: "sub-1", title: "the payments agent", state: null, about: "api", memory_key: "alias:session:s-api", at_ms: BOT_NOW - 2 * 86_400_000 };
    expect(rowPlace(row)).toBe("What Bot remembers · your name for api");
    expect(stateSub(row, BOT_NOW)).toContain("you said");
    expect(rowPlace({ ...row, about: null })).toBe("What Bot remembers · your note");
  });

  it("its chip opens What Bot remembers on that memory", async () => {
    const key = window.__AIJ_BOT__!.memories!.items[0]!.key;
    render(<BotPage onBack={() => undefined} />);
    await screen.findAllByTestId("bot-turn");
    focusMemory(key);
    act(() => void window.dispatchEvent(new Event("aij:bot-memories")));
    const memory = await screen.findByTestId("bot-memories");
    await waitFor(() => expect(within(memory).getAllByRole("listitem").some((li) => li.getAttribute("aria-current") === "true")).toBe(true));
  });
});
