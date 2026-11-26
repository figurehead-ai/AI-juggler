/**
 * AJ-E3826 vitest — Settings › AI › Models says what Bot uses and whether
 * it runs: R1 the Bot's AI card (the choice in the accent's soft fill, its
 * live state and today's totals from the engine status, a new choice
 * through the chat job's pick), R2 Recommended only while no model is in
 * use, R3 each job's row in plain words (on, Beta with its switch, off
 * and why; never a bare "Not yet"), and the audit's M2 (a job's chip lit
 * while it is on). The preview's data; the backend mocked.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { forgetSharedReads } from "../../../lib/shared-read";
import { resetEngineView } from "../../../lib/bot-engine";
import { STATES, engine, jobs, seedModels, type ModelsState } from "../../../preview/ai-models";
import { ModelsPane } from "./ModelsPane";
import { stateLine, todayLine } from "./ModelsBotCard";

const NOW = Date.UTC(2026, 10, 19, 12);

beforeEach(() => {
  vi.spyOn(Date, "now").mockReturnValue(NOW);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  forgetSharedReads();
  resetEngineView();
  for (const key of ["__AIJ_AI_CATALOG__", "__AIJ_AI_MODELS__", "__AIJ_AI_JOBS__", "__AIJ_AI_START__", "__AIJ_AI_ENGINE__", "__AIJ_OPENED__"]) Reflect.deleteProperty(window, key);
});

async function show(state: ModelsState): Promise<HTMLElement> {
  seedModels(structuredClone(state));
  render(<ModelsPane />);
  const pane = await screen.findByTestId("models-screen");
  await waitFor(() => expect(screen.getByTestId("models-installed")).toBeTruthy());
  return pane;
}

const lineOf = (job: string) => screen.getByTestId(`models-job-line-${job}`).textContent;

describe("AJ-E3826 R1: the Bot's AI card", () => {
  it("opens the page once a model is in use: the choice in the accent's soft fill, running, answered 2 min ago, today's totals with the estimate labelled", async () => {
    const pane = await show(STATES.inuse!);
    const card = screen.getByTestId("models-bot");
    // It leads: the page's first section.
    expect(pane.querySelector("section")?.id).toBe("settings-ai-bot");
    const chosen = within(card).getByTestId("models-bot-choice-apple-on-device");
    expect(chosen.dataset.chosen).toBe("true");
    expect(chosen.className).toContain("bg-accent-soft");
    await waitFor(() => expect(within(chosen).getByTestId("models-bot-state").textContent).toBe("Running · answered 2 min ago"));
    expect(within(chosen).getByTestId("models-bot-state").dataset.tone).toBe("live");
    expect(within(card).getByTestId("models-bot-today").textContent).toBe("Today · 12 answers · 8.1k tokens in · ≈ 2.3k est. out");
    // One model here: no Automatic and no radios to choose among.
    expect(within(card).queryByRole("radio")).toBeNull();
  });

  it("failed: why and the fix in one line, the warning's mark, never a fill", async () => {
    await show(STATES.failed!);
    const state = await screen.findByTestId("models-bot-state");
    await waitFor(() => expect(state.textContent).toBe("Not running: Apple's on-device model took too long to answer. Try again, or ask a shorter question."));
    expect(state.dataset.tone).toBe("warn");
    expect(state.querySelector("svg")?.getAttribute("class")).toContain("text-warning");
    expect(screen.getByTestId("models-bot").innerHTML).not.toMatch(/bg-warning/);
  });

  it("several models: Automatic first, the pick chosen, and choosing another writes the chat job's pick", async () => {
    await show(STATES.installed!);
    const radios = within(screen.getByTestId("models-bot")).getAllByRole("radio");
    expect(radios.map((radio) => radio.getAttribute("aria-label"))).toEqual(["Automatic", "Apple's on-device model", "Qwen3 1.7B", "Nemotron 3 Nano 4B"]);
    expect(radios[3]!.matches(":checked")).toBe(true);
    await act(async () => fireEvent.click(radios[2]!));
    await waitFor(() => expect(window.__AIJ_AI_JOBS__?.find((job) => job.job === "chat")?.pick).toBe("qwen3-1-7b-q8"));
    await act(async () => fireEvent.click(radios[0]!));
    await waitFor(() => expect(window.__AIJ_AI_JOBS__?.find((job) => job.job === "chat")?.pick).toBeNull());
  });

  it("the state and totals in words: idle, starting, answering, off; none today; an estimate always labelled", () => {
    expect(stateLine(null, false, NOW)).toBeNull();
    expect(stateLine(engine({ state: "idle" }), false, NOW)?.words).toBe("Idle · starts when you ask Bot");
    expect(stateLine(engine({ state: "idle", answered_ms: NOW - 3 * 3_600_000 }), false, NOW)?.words).toBe("Idle · answered 3 h ago");
    expect(stateLine(engine({ state: "running" }), false, NOW)?.words).toBe("Running");
    expect(stateLine(engine({ state: "idle" }), true, NOW)?.words).toBe("Starting…");
    expect(stateLine(engine({ state: "answering" }), false, NOW)?.words).toBe("Answering…");
    expect(stateLine(engine({ state: "off" }), false, NOW)?.words).toBe("Off · turn on Bot’s AI under Each job uses");
    expect(todayLine(engine())).toBe("Today · no answers yet");
    expect(todayLine(engine({ today: { answers: 1, tokens_in: { n: 812, estimated: false }, tokens_out: { n: 18, estimated: false } } }))).toBe("Today · 1 answer · 812 tokens in · 18 out");
    expect(todayLine(engine({ today: { answers: 2, tokens_in: { n: 900, estimated: true }, tokens_out: { n: 40, estimated: true } } }))).toBe("Today · 2 answers · ≈ 900 est. tokens in · ≈ 40 est. out");
  });
});

describe("AJ-E3826 R2: Recommended for this Mac only while no model is set up", () => {
  it("nothing in use: Recommended leads with its one primary, no Bot's AI card", async () => {
    await show(STATES.models!);
    expect(screen.getByTestId("models-recommended")).toBeTruthy();
    expect(screen.queryByTestId("models-bot")).toBeNull();
  });

  for (const name of ["inuse", "failed", "approval", "offline", "installed"]) {
    it(`${name}: a model in use, so no Recommended and no "In use" beside it`, async () => {
      const pane = await show(STATES[name]!);
      expect(screen.queryByTestId("models-recommended")).toBeNull();
      expect(pane.textContent).not.toContain("Recommended for this Mac");
      expect(screen.getByTestId("models-bot")).toBeTruthy();
    });
  }
});

describe("AJ-E3826 R3: each AI job's row says its real state (D-149)", () => {
  it("in use: on with its model, Beta, or off and why, the notes from the models section; the chip lit while on", async () => {
    await show(STATES.inuse!);
    expect(lineOf("review")).toBe("Off · Not built yet");
    expect(lineOf("chat")).toBe("On · Beta with Apple's on-device model");
    expect(lineOf("attention")).toBe("On · Built from the facts, so it needs no model");
    expect(lineOf("feed")).toBe("Off · The plain line reads better for now.");
    expect(lineOf("digest")).toBe("On · Beta with Apple's on-device model");
    expect(lineOf("watches")).toBe("Off · Drafts are being made exact before Bot fills the form.");
    const lit = (job: string) => screen.getByTestId(`models-job-${job}`).querySelector("[data-on]") !== null;
    expect(["review", "chat", "attention", "feed", "digest", "watches"].filter(lit)).toEqual(["chat", "attention", "digest"]);
    // Bot's model is chosen in the card: its row has its switch, no picker.
    expect(screen.getByTestId("models-beta-chat")).toBeTruthy();
    expect(screen.queryByTestId("models-pick-chat")).toBeNull();
    // The turn review isn't built: no switch with nothing behind it.
    expect(screen.queryByTestId("models-beta-review")).toBeNull();
  });

  it("nothing on yet: each Beta off says what the tests found, with its switch", async () => {
    await show(STATES.models!);
    expect(lineOf("chat")).toBe("Off · Preferred over the plain answer in 75 % of blind checks.");
    expect(lineOf("digest")).toBe("Off · Preferred over the plain roundup in 64 % of blind checks.");
    expect(screen.getByTestId("models-beta-digest")).toBeTruthy();
    expect(screen.getByTestId("models-job-chat").querySelector("[data-on]")).toBeNull();
  });

  it("a job this version doesn't have is on no model card: no Turn reviews Beta beside Use it's promise", async () => {
    await show(STATES.models!);
    const rec = screen.getByTestId("models-recommended");
    expect(rec.textContent).not.toContain("Turn reviews");
    expect(rec.textContent).toContain("Turns on Bot’s AI and Digest as Beta");
    for (const card of screen.getAllByTestId("model-card")) expect(card.textContent).not.toContain("Turn reviews");
  });

  it("a job's why is the notes of the model it would run on (Automatic's), not Bot's own pick", async () => {
    await show(STATES.installed!);
    expect(lineOf("chat")).toBe("On · Beta, your pick: Nemotron 3 Nano 4B");
    expect(lineOf("watches")).toBe("Off · Drafts are being made exact before Bot fills the form.");
  });

  it("never a bare Not yet; a job stopped by an update says so, not 'not built'", async () => {
    const paused = { ...STATES.inuse!, jobs: jobs({ chat: STATES.inuse!.jobs[2], digest: { state: "off", why: null } }) };
    await show(paused);
    expect(lineOf("digest")).toBe("Off · Paused for now by an AI Juggler update");
    const table = screen.getByTestId("models-jobs");
    expect(table.textContent).not.toMatch(/Not yet/);
    for (const row of within(table).getAllByRole("listitem")) expect(row.textContent).toMatch(/· /);
  });
});
