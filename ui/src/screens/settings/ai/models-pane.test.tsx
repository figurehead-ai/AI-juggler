/**
 * AJ-E3812 vitest — Settings › AI › Models in each state (none installed,
 * one installed, downloading, withdrawn, no list, offline, not enough
 * disk), on the preview's data with the backend mocked: nothing is ever
 * downloaded. Also the maintainer's three fixes: no Attention hints on a
 * model card, memory minimums from the list, and the honesty line with
 * its "How we test models" link; one filled primary per view; and the
 * clicks (Use it, the second-model approval, Remove, a job's pick).
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { forgetSharedReads } from "../../../lib/shared-read";
import { primariesIn } from "../../../components/primitives";
import { GEMMA, STATES, catalog, seedModels, type ModelsState } from "../../../preview/ai-models";
import { ModelsPane } from "./ModelsPane";
import { JobsPane } from "./JobsPane";

const INTERNAL = [/\bpacks?\b/i, /harness/i, /gguf/i, /apple_builtin/i, /not_yet/, /beta_off/, /\bundefined\b/, /\bnull\b/, /NaN/];

afterEach(() => {
  cleanup();
  forgetSharedReads();
  for (const key of ["__AIJ_AI_CATALOG__", "__AIJ_AI_MODELS__", "__AIJ_AI_JOBS__", "__AIJ_AI_START__", "__AIJ_OPENED__"]) Reflect.deleteProperty(window, key);
});

async function show(state: ModelsState): Promise<HTMLElement> {
  seedModels(structuredClone(state));
  render(<ModelsPane />);
  const pane = await screen.findByTestId("models-screen");
  await waitFor(() => expect(screen.queryByTestId("models-skeleton")).toBeNull());
  await waitFor(() => expect(screen.getByTestId("models-installed")).toBeTruthy());
  return pane;
}

function plain(text: string): void {
  for (const word of INTERNAL) expect(text, `"${word}" on screen`).not.toMatch(word);
}

describe("AJ-E3812 Settings › AI › Models", () => {
  it("none installed: Apple's model recommended with Use it, four tested models, nothing downloaded", async () => {
    const pane = await show(STATES.models!);
    const rec = screen.getByTestId("models-recommended");
    expect(rec.textContent).toContain("Apple's on-device model");
    expect(rec.textContent).toContain("no download, no disk");
    expect(within(rec).getByTestId("models-recommended-action").textContent).toBe("Use it");
    expect(rec.textContent).toContain("Turns on Turn reviews and Bot answers as Beta, on this model.");
    expect(screen.getAllByTestId("model-card").map((card) => card.dataset.model)).toEqual(["qwen3-1-7b-q8", "granite-4-1-3b-q4km", "nemotron-3-nano-4b-q4km", "gemma-4-e2b-q4-0"]);
    expect(screen.getByTestId("models-total").textContent).toBe("0 GB");
    expect(document.getElementById("settings-ai-installed")?.textContent).toContain("Nothing downloaded yet");
    // D-126: one filled primary in the view.
    expect(primariesIn(pane)).toHaveLength(1);
    plain(pane.textContent ?? "");
  });

  it("fix 1: no Attention hints on a model card; it shows only in Each job uses, needing no model", async () => {
    await show(STATES.models!);
    for (const card of screen.getAllByTestId("model-card")) expect(card.textContent).not.toContain("Attention hints");
    expect(screen.getByTestId("models-recommended").textContent).not.toContain("Attention hints");
    const attention = screen.getByTestId("models-job-attention");
    expect(attention.textContent).toContain("No model needed");
    expect(within(attention).queryByRole("switch")).toBeNull();
  });

  it("fix 2: each card's memory is the list's own minimum (8 GB, Gemma 16 GB), never a flat figure", async () => {
    const state = STATES.models!;
    const bigger = { ...GEMMA, min_memory_gb: 24 };
    await show({ ...state, catalog: catalog(state.catalog.mac!, state.catalog.recommendation!, state.catalog.models.map((m) => (m.id === GEMMA.id ? bigger : m))) });
    const gemma = screen.getAllByTestId("model-card").find((card) => card.dataset.model === GEMMA.id)!;
    expect(within(gemma).getByTestId("model-facts").textContent).toContain("24 GB");
    expect(within(gemma).getByTestId("model-facts").textContent).toContain("Mac or more");
    const qwen = screen.getAllByTestId("model-card")[0]!;
    expect(within(qwen).getByTestId("model-facts").textContent).toContain("8 GB");
  });

  it("fix 3: the honesty line sits by the verdicts and opens How we test models", async () => {
    await show(STATES.models!);
    const line = screen.getByTestId("models-honesty");
    expect(line.textContent).toContain("not on real messages");
    fireEvent.click(within(line).getByRole("button", { name: "How we test models" }));
    expect(window.__AIJ_OPENED__).toContainEqual({ kind: "guide", id: "ai-models#how-we-test-models" });
  });

  it("Use it turns on the Beta jobs the model passed", async () => {
    await show(STATES.models!);
    fireEvent.click(screen.getByTestId("models-recommended-action"));
    await waitFor(() => expect(window.__AIJ_AI_JOBS__?.filter((job) => job.beta_on).map((job) => job.job)).toEqual(["review", "chat"]));
  });

  it("a download recommended: Download with its size and what it needs, and Apple's tip", async () => {
    await show(STATES.noapple!);
    expect(screen.getByTestId("models-recommended-action").textContent).toBe("Download · 1.8 GB");
    expect(screen.getByTestId("models-fit").textContent).toContain("48 GB · needs 11.8 GB");
    expect(screen.getByTestId("models-apple-tip").textContent).toContain("Turn on Apple Intelligence");
    expect(screen.queryByTestId("models-jobs")).toBeNull();
  });

  it("downloading: progress, the size done, Cancel, how it's checked", async () => {
    await show(STATES.downloading!);
    const box = screen.getByTestId("models-downloading");
    expect(box.textContent).toContain("0.7 GB of 1.8 GB");
    expect(within(box).getByRole("progressbar").getAttribute("value")).toBe("38");
    expect(box.textContent).toContain("Cancel leaves nothing behind");
    // One download at a time: every card's Download waits for this one.
    for (const card of screen.getAllByTestId("model-card")) expect(within(card).getByTestId("model-download").hasAttribute("disabled")).toBe(true);
    fireEvent.click(within(box).getByTestId("models-cancel"));
    await waitFor(() => expect(window.__AIJ_AI_MODELS__?.download?.state).toBe("cancelled"));
    await waitFor(() => expect(screen.getByTestId("models-recommended-action").textContent).toBe("Download · 1.8 GB"));
  });

  it("one installed: In use, and a second model asks for approval every time", async () => {
    await show(STATES.approval!);
    expect(screen.getByTestId("models-recommended").textContent).toContain("In use");
    const granite = screen.getAllByTestId("model-card").find((card) => card.dataset.model === "granite-4-1-3b-q4km")!;
    fireEvent.click(within(granite).getByTestId("model-download"));
    const dialog = await screen.findByTestId("models-approval");
    expect(dialog.textContent).toContain("You already have Qwen3 1.7B");
    expect(dialog.textContent).toContain("3.9 GB");
    expect(dialog.textContent).toContain("Stays on this Mac.");
    expect(dialog.textContent).toContain("Asked for every extra model.");
    expect(document.activeElement?.textContent).toBe("Not now");
    fireEvent.click(within(dialog).getByTestId("models-approval-download"));
    await waitFor(() => expect(window.__AIJ_AI_MODELS__?.download?.id).toBe("granite-4-1-3b-q4km"));
    expect(screen.queryByTestId("models-approval")).toBeNull();
  });

  it("Not now is remembered", async () => {
    await show(STATES.approval!);
    const granite = screen.getAllByTestId("model-card").find((card) => card.dataset.model === "granite-4-1-3b-q4km")!;
    fireEvent.click(within(granite).getByTestId("model-download"));
    fireEvent.click(await screen.findByTestId("models-approval-not-now"));
    await waitFor(() => expect(window.__AIJ_AI_MODELS__?.declined).toEqual(["granite-4-1-3b-q4km"]));
  });

  it("not enough disk: the exact numbers, a way to fix it, Download held on every card", async () => {
    await show(STATES.nodisk!);
    const alert = screen.getByTestId("models-no-disk");
    expect(alert.textContent).toContain("A download needs its 1.8 GB plus 10 GB kept free for your work: 11.8 GB. This Mac has 11.2 GB free, so free up 0.6 GB");
    expect(screen.getByTestId("models-fit").textContent).toContain("What it needs");
    fireEvent.click(within(alert).getByRole("button", { name: "Open Storage settings" }));
    expect(window.__AIJ_OPENED__).toContainEqual({ kind: "system", id: "storage" });
    for (const card of screen.getAllByTestId("model-card")) {
      expect(within(card).getByTestId("model-download").hasAttribute("disabled")).toBe(true);
      expect(card.textContent).toMatch(/Needs \d+(\.\d)? GB free first/);
    }
  });

  it("offline: Apple's model still in use, the list unreachable, the installed one working", async () => {
    await show(STATES.offline!);
    expect(screen.getByTestId("models-recommended").textContent).toContain("In use");
    expect(screen.getByTestId("models-unread").textContent).toContain("Can't read the list of tested models");
    expect(screen.queryAllByTestId("model-card")).toHaveLength(0);
    expect(screen.getByTestId("models-installed").textContent).toContain("Qwen3 1.7B");
    expect(document.getElementById("settings-ai-installed")?.textContent).toContain("1 downloaded · works offline");
    expect(screen.queryByTestId("models-withdrawn")).toBeNull();
  });

  it("no list on a Mac with nothing to recommend: says why, with no button", async () => {
    const pane = await show(STATES.nolist!);
    expect(screen.getByTestId("models-nothing").textContent).toContain("Every job keeps its plain version.");
    expect(primariesIn(pane)).toHaveLength(0);
    expect(screen.getByTestId("models-unread").textContent).toContain("No model shows until");
  });

  it("withdrawn: kept with its note; Remove says what it frees and that it can't come back", async () => {
    await show(STATES.installed!);
    expect(screen.getByTestId("models-total").textContent).toBe("4.7 GB");
    expect(screen.getByTestId("models-withdrawn").textContent).toBe("We no longer list this model. It keeps working until you remove it.");
    fireEvent.click(screen.getAllByTestId("models-remove")[1]!);
    const confirm = screen.getByTestId("models-remove-confirm");
    expect(confirm.textContent).toContain("Remove Nemotron 3 Nano 4B? Frees 2.8 GB. Bot answers goes back to Automatic (Apple's on-device model). You can't download it again: we no longer list it.");
    fireEvent.click(within(confirm).getByTestId("models-remove-confirmed"));
    await waitFor(() => expect(window.__AIJ_AI_MODELS__?.installed.map((m) => m.id)).toEqual(["qwen3-1-7b-q8"]));
  });

  it("a job's picker: Automatic first, then only installed models that passed it", async () => {
    await show(STATES.installed!);
    fireEvent.click(screen.getByTestId("models-pick-chat"));
    const menu = await screen.findByRole("menu");
    const lines = within(menu).getAllByRole("menuitemradio").map((item) => item.textContent ?? "");
    expect(lines[0]).toMatch(/^Automatic/);
    expect(lines.slice(1).map((line) => line.split("Beta")[0])).toEqual(["Apple's on-device model", "Qwen3 1.7B", "Nemotron 3 Nano 4B"]);
    await act(async () => fireEvent.click(within(menu).getAllByRole("menuitemradio")[0]!));
    await waitFor(() => expect(window.__AIJ_AI_JOBS__?.find((job) => job.job === "chat")?.pick).toBeNull());
  });

  it("paints the last answer at once when opened again", async () => {
    await show(STATES.models!);
    cleanup();
    render(<ModelsPane />);
    expect(screen.getByTestId("models-recommended")).toBeTruthy();
  });

  it("Settings › AI › Jobs: the same rows on their own", async () => {
    seedModels(structuredClone(STATES.installed!));
    render(<JobsPane />);
    const rows = await screen.findByTestId("models-jobs");
    await waitFor(() => expect(rows.textContent).toContain("On · your pick, Nemotron 3 Nano 4B"));
    expect(screen.getByTestId("models-honesty")).toBeTruthy();
  });
});
