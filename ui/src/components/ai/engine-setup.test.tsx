/**
 * AJ-E3823 vitest — the engine-setup guide: each Apple state in plain
 * words (a blocked or unanswered probe says "couldn't check" or
 * "blocked", never "can't run it", and never leads to a download
 * recommendation), the recommended badge in each case, the buttons'
 * words, "What is this?", and the roles assistive tech reads. On the
 * preview's data with the backend mocked: nothing is ever downloaded
 * before the Download click.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { forgetSharedReads } from "../../lib/shared-read";
import { primariesIn } from "../primitives";
import { STATES, catalog, jobs, seedModels, type ModelsState } from "../../preview/ai-models";
import type { MacInfo } from "../../types/generated/MacInfo";
import { EngineSetup } from "./EngineSetup";
import { setupView, showsSetup } from "./engine-setup-view";

afterEach(() => {
  cleanup();
  forgetSharedReads();
  for (const key of ["__AIJ_AI_CATALOG__", "__AIJ_AI_MODELS__", "__AIJ_AI_JOBS__", "__AIJ_AI_START__", "__AIJ_OPENED__", "__AIJ_OS_PROMPTS__"]) Reflect.deleteProperty(window, key);
});

/** A state with this Mac's Apple Intelligence set to `apple`. */
function withApple(apple: MacInfo["apple_ai"], base: ModelsState = STATES.appleoff!): ModelsState {
  const c = base.catalog;
  return { ...base, catalog: catalog({ ...c.mac!, apple_ai: apple }, c.recommendation!, c.models, c.listed) };
}

async function show(state: ModelsState, place: "bot" | "wizard" | "models" = "bot", props: { onSkip?: () => void } = {}): Promise<HTMLElement> {
  seedModels(structuredClone(state));
  render(<EngineSetup place={place} {...props} />);
  const setup = await screen.findByTestId("engine-setup");
  await waitFor(() => expect(setup.dataset.apple).not.toBe("checking"));
  return setup;
}

const chat = (state: ModelsState) => state.jobs.find((j) => j.job === "chat") ?? null;

describe("AJ-E3823 the guide's view", () => {
  it("nothing read yet: checking first", () => {
    expect(setupView(null, null, null).checking).toBe(true);
  });

  it("each Apple state, and the badge: Apple's when it can run here, the download only when it can't, none while unknown", () => {
    const cases: Array<[MacInfo["apple_ai"], string, "apple" | "download" | null]> = [
      ["on", "ready", "apple"],
      ["off", "off", "apple"],
      ["preparing", "preparing", "apple"],
      ["unavailable", "cant_run", "download"],
      ["unchecked", "unchecked", null],
      ["no_answer", "no_answer", null],
      ["blocked", "blocked", null],
    ];
    for (const [apple, words, badge] of cases) {
      const state = withApple(apple);
      const view = setupView(state.catalog, state.models, chat(state));
      expect(view.apple, apple).toBe(words);
      expect(view.recommended, apple).toBe(badge);
      expect(view.download?.name, apple).toBe("Qwen3 1.7B");
      expect(view.downloadAction, apple).toBe("download");
    }
  });

  it("an Intel Mac, or nothing that fits: no AI here, one line on what it needs", () => {
    const intel = catalog({ ...STATES.noapple!.catalog.mac!, apple_silicon: false, chip: null }, { model_id: null, why: null, nothing: "not_apple_silicon", needed_bytes: null, apple_ai_off: false }, []);
    const view = setupView(intel, null, null);
    expect(view.apple).toBe("cant_run");
    expect(view.noAi).toBe("not_apple_silicon");
    expect(view.recommended).toBeNull();
  });

  it("the Models screen leads with it only while Apple's AI is off, getting ready or unknown", () => {
    expect(showsSetup(STATES.appleoff!.catalog)).toBe(true);
    expect(showsSetup(STATES.noanswer!.catalog)).toBe(true);
    expect(showsSetup(STATES.blocked!.catalog)).toBe(true);
    expect(showsSetup(STATES.models!.catalog)).toBe(false);
    expect(showsSetup(STATES.noapple!.catalog)).toBe(false);
  });
});

describe("AJ-E3823 the guide on screen", () => {
  it("says it is checking first, as a status", async () => {
    render(<EngineSetup place="bot" />);
    expect(screen.getByRole("status").textContent).toBe("Checking what this Mac can do…");
  });

  it("ready: Apple's recommended with Use Apple's AI; Qwen named, sized, with the free disk; one primary", async () => {
    const setup = await show(withApple("on", STATES.models));
    expect(screen.getByTestId("setup-recommendation").textContent).toBe("Recommended: Apple’s built-in AI. It’s ready, with nothing to download.");
    const apple = screen.getByRole("article", { name: "Apple’s built-in AI" });
    expect(apple.dataset.recommended).toBe("");
    expect(within(apple).getByText("Recommended")).toBeTruthy();
    expect(screen.getByTestId("setup-apple-state").textContent).toBe("Ready");
    expect(within(apple).getByRole("button", { name: "Use Apple’s AI" })).toBeTruthy();
    const qwen = screen.getByRole("article", { name: "Qwen3 1.7B" });
    expect(qwen.dataset.recommended).toBeUndefined();
    expect(qwen.textContent).toContain("A small AI from the Qwen team, 1.8 GB, runs only on this Mac");
    expect(qwen.textContent).toContain("Not downloaded yet · 212 GB free on this Mac");
    expect(within(qwen).getByRole("button", { name: "Download Qwen3 1.7B · 1.8 GB" })).toBeTruthy();
    expect(primariesIn(setup)).toHaveLength(1);
    expect(primariesIn(setup)[0]!.textContent).toBe("Use Apple’s AI");
  });

  it("Use Apple's AI turns on Bot's answers only", async () => {
    await show(withApple("on", STATES.models));
    fireEvent.click(screen.getByRole("button", { name: "Use Apple’s AI" }));
    await waitFor(() => expect(window.__AIJ_AI_JOBS__?.find((j) => j.job === "chat")?.beta_on).toBe(true));
    expect(window.__AIJ_AI_JOBS__?.find((j) => j.job === "review")?.beta_on).toBe(false);
  });

  it("off: the badge says turn it on, the exact steps, and the button opens Apple Intelligence settings", async () => {
    const setup = await show(STATES.appleoff!);
    expect(screen.getByTestId("setup-recommendation").textContent).toBe("Recommended: Turn on Apple Intelligence (free, nothing to download).");
    expect(screen.getByTestId("setup-apple-state").textContent).toBe("Apple Intelligence is turned off");
    const steps = within(screen.getByTestId("setup-apple-steps")).getAllByRole("listitem");
    expect(steps.map((s) => s.textContent)).toEqual([
      "Click Open Apple Intelligence settings.",
      "Turn on Apple Intelligence.",
      "Wait while macOS gets it ready, then come back and click Check again.",
    ]);
    fireEvent.click(screen.getByRole("button", { name: "Open Apple Intelligence settings" }));
    await waitFor(() => expect(window.__AIJ_OPENED__).toEqual([{ kind: "system", id: "apple_intelligence" }]));
    expect(primariesIn(setup)).toHaveLength(1);
    expect(screen.getByRole("article", { name: "Qwen3 1.7B" }).dataset.recommended).toBeUndefined();
  });

  it("macOS still downloading it: says so, Apple's recommended, nothing to download", async () => {
    await show(withApple("preparing"));
    expect(screen.getByTestId("setup-apple-state").textContent).toBe("macOS is still downloading it");
    expect(screen.getByTestId("setup-recommendation").textContent).toContain("nothing for you to download");
  });

  for (const [apple, state, why] of [
    ["unchecked", "Couldn’t check", "AI Juggler couldn’t tell whether this Mac can run it."],
    ["no_answer", "Couldn’t check: no answer", "If macOS is showing a message about AI Juggler, answer it, then check again."],
    ["blocked", "AI Juggler’s helper was blocked by macOS", "macOS stopped AI Juggler’s helper before it could check."],
  ] as const) {
    it(`${apple}: "${state}", what to do, Check again; never "can't run it", never a download recommended`, async () => {
      const setup = await show(withApple(apple, STATES.noanswer));
      expect(screen.getByTestId("setup-apple-state").textContent).toBe(state);
      expect(screen.getByTestId("setup-apple-why").textContent).toContain(why);
      expect(setup.textContent).not.toContain("can’t run");
      const line = screen.getByTestId("setup-recommendation");
      expect(line.dataset.known).toBeUndefined();
      expect(line.textContent).not.toMatch(/^Recommended/);
      expect(line.textContent).toContain("Qwen3 1.7B is the other choice");
      expect(setup.querySelector("[data-recommended]")).toBeNull();
      expect(primariesIn(setup).map((b) => b.textContent)).toEqual(["Check again"]);
      expect(screen.getByRole("button", { name: "Download Qwen3 1.7B · 1.8 GB" })).toBeTruthy();
      fireEvent.click(screen.getByRole("button", { name: "Check again" }));
      await waitFor(() => expect(screen.getByRole("button", { name: "Check again" })).toBeTruthy());
      expect(window.__AIJ_AI_MODELS__?.download).toBeNull();
    });
  }

  // AJ-E1547: never checked because Bot's AI helper isn't set up yet: the
  // check is its first start, and the click goes through the gate first.
  it("not set up yet: says so, and Check this Mac is the one primary", async () => {
    window.__AIJ_OS_PROMPTS__ = [
      { id: "helper", title: "Start Bot's AI", why: "", system_says: "", click: "", if_declined: "", offered_again: "", needed: true, status: "not_asked", primed: false, change_later: null, background: "", never_shown: false, dialog: null },
    ];
    const setup = await show(withApple("unchecked", STATES.noanswer));
    await waitFor(() => expect(screen.getByTestId("setup-apple-state").textContent).toBe("Not checked yet"));
    expect(screen.getByTestId("setup-apple-why").textContent).toContain("tells you first");
    expect(setup.textContent).not.toContain("install AI Juggler again");
    expect(primariesIn(setup).map((b) => b.textContent)).toEqual(["Check this Mac"]);
    fireEvent.click(screen.getByTestId("setup-apple-set-up"));
    await waitFor(() => expect(screen.getByTestId("setup-apple-set-up").hasAttribute("disabled")).toBe(false));
    expect(window.__AIJ_AI_MODELS__?.download).toBeNull();
  });

  it("this Mac can't run Apple's: one line says so; Qwen recommended; Download starts only on its click", async () => {
    const setup = await show(STATES.noapple!);
    expect(screen.getByTestId("setup-apple-state").textContent).toContain("Apple’s built-in AI: This Mac can’t run it.");
    expect(screen.queryByRole("article", { name: "Apple’s built-in AI" })).toBeNull();
    expect(screen.getByTestId("setup-recommendation").textContent).toBe("Recommended: Qwen3 1.7B. Apple’s built-in AI can’t run on this Mac.");
    const qwen = screen.getByRole("article", { name: "Qwen3 1.7B" });
    expect(qwen.dataset.recommended).toBe("");
    expect(window.__AIJ_AI_MODELS__?.download).toBeNull();
    const download = within(qwen).getByRole("button", { name: "Download Qwen3 1.7B · 1.8 GB" });
    expect(primariesIn(setup)).toEqual([download]);
    fireEvent.click(download);
    await waitFor(() => expect(window.__AIJ_AI_MODELS__?.download?.id).toBe("qwen3-1-7b-q8"));
  });

  it("no AI possible: one line on what it needs, and Skip in the wizard", async () => {
    let skipped = 0;
    const intel: ModelsState = {
      ...STATES.nolist!,
      catalog: catalog({ ...STATES.nolist!.catalog.mac!, apple_silicon: false, chip: null }, { model_id: null, why: null, nothing: "not_apple_silicon", needed_bytes: null, apple_ai_off: false }, []),
    };
    await show(intel, "wizard", { onSkip: () => (skipped += 1) });
    expect(screen.getByTestId("setup-no-ai").textContent).toContain("Bot’s AI needs a Mac with Apple silicon.");
    fireEvent.click(screen.getByRole("button", { name: "Skip for now" }));
    expect(skipped).toBe(1);
  });

  it("What is this?: what it is, where it's kept, where it comes from, nothing leaves, how to remove it", async () => {
    await show(STATES.appleoff!);
    const toggle = screen.getByRole("button", { name: "What is this?" });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    const body = screen.getByTestId("setup-what-body").textContent ?? "";
    expect(body).toContain("a small AI (a language model) that runs on this Mac");
    expect(body).toContain("Qwen3 1.7B is one 1.8 GB file in AI Juggler’s own folder on this Mac.");
    expect(body).toContain("made by the Qwen team and downloaded from Hugging Face");
    expect(body).toContain("Nothing you ask leaves this Mac");
    expect(body).toContain("Remove Qwen3 1.7B any time in Settings › AI › Models.");
  });

  it("roles and words: a labelled group of two labelled choices, no bare Download, no internal words", async () => {
    const setup = await show(STATES.appleoff!);
    expect(screen.getByRole("region", { name: "Choose the small AI that writes Bot’s answers" })).toBeTruthy();
    expect(screen.getAllByRole("article")).toHaveLength(2);
    expect(screen.getByRole("heading", { name: "Set up Bot" })).toBeTruthy();
    const what = screen.getByTestId("setup-what-body").textContent ?? "";
    const text = (setup.textContent ?? "").replace(what, "");
    expect(text).not.toMatch(/Download ·/);
    expect(text).not.toMatch(/look-?ups?|gguf|apple_builtin|undefined|null|NaN/i);
    // "model" is never a bare word: only "What is this?" says it, explained.
    expect(text).not.toMatch(/\bmodels?\b/i);
  });

  it("in the Models screen: no heading or lead of its own; in the wizard: the lead, no heading", async () => {
    await show(STATES.appleoff!, "models");
    expect(screen.queryByRole("heading", { name: "Set up Bot" })).toBeNull();
    expect(screen.queryByText(/Bot is your AI assistant/)).toBeNull();
    cleanup();
    forgetSharedReads();
    await show(STATES.appleoff!, "wizard");
    expect(screen.queryByRole("heading", { name: "Set up Bot" })).toBeNull();
    expect(screen.getByText(/Bot is your AI assistant for everything AI Juggler sees/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Skip for now" })).toBeNull();
  });

  it("Qwen already downloaded and Apple's off: Use Qwen3 1.7B", async () => {
    const base = STATES.appleoff!;
    const state: ModelsState = { ...base, models: STATES.approval!.models, jobs: jobs() };
    await show(state);
    const qwen = screen.getByRole("article", { name: "Qwen3 1.7B" });
    expect(qwen.textContent).toContain("Downloaded, on this Mac");
    expect(within(qwen).getByRole("button", { name: "Use Qwen3 1.7B" })).toBeTruthy();
  });
});
