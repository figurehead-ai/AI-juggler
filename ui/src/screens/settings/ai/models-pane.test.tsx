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
import { resetEngineView } from "../../../lib/bot-engine";
import { primariesIn } from "../../../components/primitives";
import { GEMMA, STATES, catalog, seedModels, type ModelsState } from "../../../preview/ai-models";
import { ModelsPane } from "./ModelsPane";

const INTERNAL = [/\bpacks?\b/i, /harness/i, /gguf/i, /apple_builtin/i, /not_yet/, /beta_off/, /\bundefined\b/, /\bnull\b/, /NaN/];

afterEach(() => {
  cleanup();
  forgetSharedReads();
  resetEngineView();
  for (const key of ["__AIJ_AI_CATALOG__", "__AIJ_AI_MODELS__", "__AIJ_AI_JOBS__", "__AIJ_AI_START__", "__AIJ_AI_ENGINE__", "__AIJ_OPENED__"]) Reflect.deleteProperty(window, key);
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
    // AJ-E3826: only the Beta jobs this version has (the turn review isn't built yet).
    expect(rec.textContent).toContain("Turns on Bot’s AI and Digest as Beta, on this model.");
    expect(screen.getAllByTestId("model-card").map((card) => card.dataset.model)).toEqual(["qwen3-1-7b-q8", "granite-4-1-3b-q4km", "nemotron-3-nano-4b-q4km", "gemma-4-e2b-q4-0"]);
    expect(screen.getByTestId("models-total").textContent).toBe("0 GB");
    expect(document.getElementById("settings-ai-installed")?.textContent).toContain("Nothing downloaded yet");
    // D-126: one filled primary in the view.
    expect(primariesIn(pane)).toHaveLength(1);
    plain(pane.textContent ?? "");
  });

  it("AJ-E1548 #7: the lead says what ships", async () => {
    const pane = await show(STATES.models!);
    expect(pane.textContent).not.toContain("Beta reviews");
    const { PAGE } = await import("./models-words");
    expect(PAGE.lead).toMatch(/^The AI that writes Bot's answers\./);
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
    await waitFor(() => expect(window.__AIJ_AI_JOBS__?.filter((job) => job.beta_on).map((job) => job.job)).toEqual(["chat", "digest"]));
  });

  it("a download recommended where Apple's AI can't run: Download names it, its size and what it needs", async () => {
    await show(STATES.noapple!);
    expect(screen.getByTestId("models-recommended-action").textContent).toBe("Download Qwen3 1.7B · 1.8 GB");
    expect(screen.getByTestId("models-fit").textContent).toContain("48 GB · needs 11.8 GB");
    expect(screen.queryByTestId("models-apple-tip")).toBeNull();
    expect(screen.queryByTestId("engine-setup")).toBeNull();
    expect(screen.queryByTestId("models-jobs")).toBeNull();
  });

  // AJ-E3823: Apple's AI off, unanswered or blocked: the setup guide leads
  // in the Recommended card's place; never a download recommended, and the
  // AI it names isn't listed twice.
  for (const [name, apple] of [
    ["appleoff", "off"],
    ["noanswer", "no_answer"],
    ["blocked", "blocked"],
  ] as const) {
    it(`${name}: the setup guide on top, no download recommended, nothing shown twice`, async () => {
      const pane = await show(STATES[name]!);
      const setup = await screen.findByTestId("engine-setup");
      expect(setup.dataset.place).toBe("models");
      expect(setup.dataset.apple).toBe(apple);
      expect(screen.queryByTestId("models-recommended")).toBeNull();
      expect(screen.queryByTestId("models-nothing")).toBeNull();
      expect(screen.queryByTestId("models-apple-tip")).toBeNull();
      expect(within(setup).getByTestId("setup-download-card").dataset.recommended).toBeUndefined();
      const cards = screen.getAllByTestId("model-card").map((card) => card.dataset.model);
      expect(cards).not.toContain("qwen3-1-7b-q8");
      expect(primariesIn(pane)).toHaveLength(1);
      plain(pane.textContent ?? "");
    });
  }

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
    await waitFor(() => expect(screen.getByTestId("models-recommended-action").textContent).toBe("Download Qwen3 1.7B · 1.8 GB"));
  });

  it("one installed and in use: Bot's AI leads, and a second model asks for approval every time", async () => {
    await show(STATES.approval!);
    expect(screen.queryByTestId("models-recommended")).toBeNull();
    expect(screen.getByTestId("models-bot").textContent).toContain("Qwen3 1.7B");
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
    expect(screen.queryByTestId("models-recommended")).toBeNull();
    // Two models can write Bot's answers here: Automatic first, and chosen.
    const automatic = screen.getByTestId("models-bot-choice-automatic");
    expect(automatic.dataset.chosen).toBe("true");
    await waitFor(() => expect(within(automatic).getByTestId("models-bot-state").textContent).toBe("Apple's on-device model · Idle · starts when you ask Bot"));
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
    expect(confirm.textContent).toContain("Remove Nemotron 3 Nano 4B? Frees 2.8 GB. Bot’s AI goes back to Automatic (Apple's on-device model). You can't download it again: we no longer list it.");
    fireEvent.click(within(confirm).getByTestId("models-remove-confirmed"));
    await waitFor(() => expect(window.__AIJ_AI_MODELS__?.installed.map((m) => m.id)).toEqual(["qwen3-1-7b-q8"]));
  });

  it("a job's picker: Automatic first, then only installed models that passed it, each with its icon (M1)", async () => {
    await show(STATES.installed!);
    fireEvent.click(screen.getByTestId("models-pick-digest"));
    const menu = await screen.findByRole("menu");
    const items = within(menu).getAllByRole("menuitemradio");
    expect(items[0]!.textContent).toMatch(/^Automatic/);
    expect(items.slice(1).map((line) => (line.textContent ?? "").split("Beta")[0])).toEqual(["Apple's on-device model"]);
    for (const item of items) expect(item.querySelector("svg"), `${item.textContent}: its icon`).not.toBeNull();
    // The chosen one's icon is in the field.
    expect(screen.getByTestId("models-pick-digest").querySelector("svg:not(:last-child)")).not.toBeNull();
    await act(async () => fireEvent.click(items[1]!));
    await waitFor(() => expect(window.__AIJ_AI_JOBS__?.find((job) => job.job === "digest")?.pick).toBe("apple-on-device"));
  });

  it("paints the last answer at once when opened again", async () => {
    await show(STATES.models!);
    cleanup();
    render(<ModelsPane />);
    expect(screen.getByTestId("models-recommended")).toBeTruthy();
  });
});
