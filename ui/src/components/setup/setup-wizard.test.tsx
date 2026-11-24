/**
 * AJ-E1545 — the setup wizard: its steps (the permissions only while one
 * is missing; Meet Bot after them, before Keep it running; the tour
 * last), Next naming where it goes, each permission its own card with
 * Allow raising the system prompt and the grant seen live (✓), Skip for
 * now on every card and step, Meet Bot's three standings by the router's
 * own answer (D-144's words), Keep it running only where it applies, and
 * the tour offered at the end.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SetupWizard } from "./SetupWizard";
import { MeetBotStep, botStanding } from "./MeetBotStep";
import { loginOffered } from "./KeepRunningStep";
import { eyebrow, keepValue, nextWords, pagesOf, setupSteps } from "../../lib/setup-wizard";
import { promptsValue, setupPrompts } from "./PermissionsStep";
import { STATES as MODEL_STATES } from "../../preview/ai-models";
import { TAKE_TOUR_EVENT } from "../../lib/tour";
import type { OsPromptInfo } from "../../lib/ipc/os-prompts";
import type { AiJobInfo } from "../../types/generated/AiJobInfo";

const seam = (name: string, value: unknown) => Reflect.set(window, name, value);
const seamOf = (name: string): unknown => Reflect.get(window, name);

/** AJ-E1547's registry entry, as the gate lists it. */
const prompt = (id: string, status: string, over: Partial<OsPromptInfo> = {}): OsPromptInfo => ({
  id,
  title: id === "accessibility" ? "Jump to a session's own window" : "See a live preview of each session",
  why: "Why it helps.",
  system_says: "“AI Juggler” would like to control this computer.",
  click: "Open System Settings",
  if_declined: "Opening a session still brings its app forward.",
  offered_again: "You can turn this on later in Settings › Desktop apps.",
  needed: true,
  status,
  primed: false,
  change_later: "System Settings › Privacy & Security",
  background: "waits",
  never_shown: false,
  dialog: { style: "alert", title: "“AI Juggler” would like to control this computer", body: "Grant access in Privacy & Security.", buttons: ["Deny", "Open System Settings"], click: "Open System Settings" },
  ...over,
});

const chatOf = (jobs: AiJobInfo[]) => jobs.find((job) => job.job === "chat") ?? null;

beforeEach(() => {
  for (const name of Object.keys(window).filter((key) => key.startsWith("__AIJ_"))) Reflect.deleteProperty(window, name);
  seam("__AIJ_SCAN_REPORT__", { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false });
  localStorage.clear();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe("the steps, as data", () => {
  it("the indicator: Your apps · Sign in · Mac access · Meet Bot · Keep running; the access step only while one is missing; the tour after", () => {
    expect(setupSteps(null)).toEqual(["apps", "signin", "access", "bot", "keep"]);
    expect(setupSteps(2)).toEqual(["apps", "signin", "access", "bot", "keep"]);
    expect(setupSteps(0)).toEqual(["apps", "signin", "bot", "keep"]);
    expect(pagesOf(setupSteps(0))).toEqual(["apps", "signin", "bot", "keep", "tour"]);
  });

  it("Next names where it goes; the eyebrow says where it is", () => {
    const steps = setupSteps(1);
    expect(nextWords(steps, "access")).toBe("Next: Meet Bot");
    expect(nextWords(steps, "bot")).toBe("Next: Keep running");
    expect(nextWords(steps, "keep")).toBe("Finish setup");
    expect(nextWords(steps, "tour")).toBeNull();
    expect(eyebrow(steps, "bot")).toBe("Step 4 of 5");
    expect(eyebrow(steps, "tour")).toBe("All steps done");
  });

  it("says what Keep it running keeps on", () => {
    expect(keepValue(true, true)).toBe("Opens at login · Stays awake");
    expect(keepValue(null, true)).toBe("Stays awake");
    expect(keepValue(false, false)).toBe("As macOS does");
    expect(loginOffered("unavailable")).toBe(false);
    expect(loginOffered("off")).toBe(true);
    expect(loginOffered(null)).toBe(false);
  });

  it("offers the gate's prompts this setup needs, not the ones other steps prime (AJ-E1547)", () => {
    const all = [prompt("accessibility", "not_asked"), prompt("notifications", "not_asked", { needed: false }), prompt("login-item", "not_asked"), prompt("helper", "not_asked")];
    expect(setupPrompts(all).map((one) => one.id)).toEqual(["accessibility"]);
    expect(promptsValue([prompt("accessibility", "allowed"), prompt("screen-recording", "not_asked")])).toBe("1 of 2 allowed");
    expect(promptsValue([prompt("accessibility", "allowed")])).toBe("All allowed");
  });
});

describe("Mac access", () => {
  it("one card per prompt, through the gate: the dialog's picture and what to click; Allow primes it; ✓ only when the system says so; Skip for now on each", async () => {
    seam("__AIJ_OS_PROMPTS__", [prompt("accessibility", "not_asked"), prompt("screen-recording", "not_asked")]);
    render(<SetupWizard resumeAtAccess openLoginTerminal={() => null} onClose={() => null} />);
    const card = await screen.findByTestId("setup-permission-accessibility");
    expect(screen.getByTestId("setup-permission-screen-recording")).toBeTruthy();
    expect(screen.getByTestId("setup-title").textContent).toBe("Let AI Juggler work with your Mac");
    const shown = within(card).getByTestId("setup-permission-accessibility-prompt");
    expect(within(shown).getByTestId("os-prompt-picture")).toBeTruthy();
    expect(shown.textContent).toContain("Click");
    expect(shown.textContent).toContain("If you say no");
    expect(card.textContent).toContain("change it later in System Settings › Privacy & Security");
    await act(async () => {
      fireEvent.click(within(card).getByTestId("setup-permission-accessibility-allow"));
    });
    // Primed and called; the system hasn't answered yet: no ✓, and the card says where its dialog may be.
    expect(screen.getByTestId("setup-permission-accessibility").dataset.state).not.toBe("allowed");
    expect(screen.getByTestId("setup-permission-accessibility-asked").textContent).toContain("behind this window");
    // The system says yes: seen when the window comes back.
    seam("__AIJ_OS_PROMPTS__", [prompt("accessibility", "allowed"), prompt("screen-recording", "not_asked")]);
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() => expect(screen.getByTestId("setup-permission-accessibility").dataset.state).toBe("allowed"));
    expect(screen.getByTestId("setup-permission-accessibility-allowed").textContent).toContain("Allowed");
    fireEvent.click(screen.getByTestId("setup-permission-screen-recording-skip"));
    expect(screen.getByTestId("setup-permission-screen-recording").dataset.state).toBe("skipped");
    expect(screen.getByTestId("setup-next").textContent).toContain("Next: Meet Bot");
    expect(screen.getByTestId("setup-steps").textContent).toContain("1 of 2 allowed");
  });

  it("isn't a step when nothing needed is missing", async () => {
    seam("__AIJ_OS_PROMPTS__", [prompt("accessibility", "allowed"), prompt("notifications", "not_asked", { needed: false })]);
    render(<SetupWizard openLoginTerminal={() => null} onClose={() => null} />);
    await waitFor(() => expect(screen.getByTestId("setup-steps").textContent).not.toContain("Mac access"));
  });
});

describe("left and resumed", () => {
  it("Not now remembers the step; opened again, it carries on there; finishing forgets it", async () => {
    seam("__AIJ_OS_PROMPTS__", [prompt("accessibility", "not_asked")]);
    const first = render(<SetupWizard resumeAtAccess openLoginTerminal={() => null} onClose={() => null} />);
    await screen.findByTestId("setup-permissions");
    fireEvent.click(screen.getByTestId("setup-not-now"));
    first.unmount();
    render(<SetupWizard openLoginTerminal={() => null} onClose={() => null} />);
    expect(await screen.findByTestId("setup-permissions")).toBeTruthy();
    expect(screen.getByTestId("setup-wizard").dataset.step).toBe("access");
    fireEvent.keyDown(screen.getByTestId("setup-wizard"), { key: "Escape" });
    expect(localStorage.getItem("aij.setup.at")).toBe("access");
  });
});

describe("Find your apps", () => {
  it("is first run's discovery in place, with Skip for now and Not now in the foot", async () => {
    const onClose = vi.fn();
    seam("__AIJ_OS_PROMPTS__", [prompt("accessibility", "not_asked")]);
    render(<SetupWizard openLoginTerminal={() => null} onClose={onClose} />);
    expect(await screen.findByTestId("first-run")).toBeTruthy();
    expect(screen.getByTestId("first-run").dataset.embedded).toBe("true");
    // First run titles its own step (no second title over it).
    expect(screen.queryByTestId("setup-title")).toBeNull();
    expect(screen.getByTestId("setup-eyebrow").textContent).toBe("Step 1 of 5");
    // Its own actions (Bring in N) sit in the wizard's sticky foot.
    await waitFor(() => expect(within(screen.getByTestId("setup-foot-slot")).queryByTestId("first-run-actions")).not.toBeNull());
    fireEvent.click(screen.getByTestId("setup-skip"));
    // Past Sign in (part of Your apps' flow) to the permissions.
    expect(await screen.findByTestId("setup-permissions")).toBeTruthy();
    expect(screen.getByTestId("setup-steps").textContent).toContain("Skipped for now");
    fireEvent.click(screen.getByTestId("setup-not-now"));
    expect(onClose).toHaveBeenCalledOnce();
  });
});

describe("Meet Bot", () => {
  it("stands by the router's own answer: ready, getting ready, or to be set up", () => {
    expect(botStanding(chatOf(MODEL_STATES.installed!.jobs))).toBe("ready");
    expect(botStanding({ ...chatOf(MODEL_STATES.models!.jobs)!, state: "waiting" })).toBe("waiting");
    expect(botStanding(chatOf(MODEL_STATES.models!.jobs))).toBe("setup");
    expect(botStanding(chatOf(MODEL_STATES.nolist!.jobs))).toBe("setup");
    expect(botStanding(null)).toBe("setup");
  });

  it.each([
    ["Apple's model on this Mac, Bot off", "models", "setup"],
    ["no Apple model, the local offer", "noapple", "setup"],
    ["no engine possible", "nolist", "setup"],
    ["Bot on", "installed", "ready"],
  ] as const)("%s: says what Bot does; the shared engine setup, never a download of its own", async (_name, state, standing) => {
    const models = MODEL_STATES[state]!;
    seam("__AIJ_AI_JOBS__", models.jobs);
    seam("__AIJ_AI_CATALOG__", models.catalog);
    seam("__AIJ_AI_MODELS__", models.models);
    const onSkip = vi.fn();
    render(<MeetBotStep onSkip={onSkip} onChosen={() => null} />);
    const step = screen.getByTestId("setup-bot");
    expect(step.textContent).toContain("Cites every answer");
    expect(step.textContent).toContain("Never acts");
    expect(step.textContent).toContain("Search (⌘K)");
    expect(step.textContent).not.toMatch(/look-ups?/i);
    await waitFor(() => expect(screen.getByTestId("setup-bot-engine").dataset.standing).toBe(standing));
    if (standing === "ready") {
      expect(screen.getByTestId("setup-bot-engine").textContent).toContain("Bot is ready");
      expect(screen.queryByTestId("engine-setup")).toBeNull();
    } else {
      const setup = await screen.findByTestId("engine-setup");
      expect(setup.dataset.place).toBe("wizard");
      expect(setup.textContent).toContain("your AI assistant for everything AI Juggler sees");
      fireEvent.click(within(setup).getByTestId("setup-skip"));
      expect(onSkip).toHaveBeenCalledOnce();
    }
    // Nothing is downloaded without the user's own click (D-131).
    expect(Reflect.get(Object(seamOf("__AIJ_AI_MODELS__")), "download") ?? null).toBeNull();
  });

  it("Skip for now is the engine setup's; Next steps down beside its own primary", async () => {
    seam("__AIJ_OS_PROMPTS__", [prompt("accessibility", "allowed")]);
    seam("__AIJ_AI_JOBS__", MODEL_STATES.noapple!.jobs);
    seam("__AIJ_AI_CATALOG__", MODEL_STATES.noapple!.catalog);
    seam("__AIJ_AI_MODELS__", MODEL_STATES.noapple!.models);
    render(<SetupWizard openLoginTerminal={() => null} onClose={() => null} />);
    await waitFor(() => expect(screen.getByTestId("setup-steps").textContent).not.toContain("Mac access"));
    fireEvent.click(screen.getByTestId("setup-skip"));
    expect(await screen.findByTestId("setup-bot")).toBeTruthy();
    const setup = await screen.findByTestId("engine-setup");
    expect(within(setup).getByTestId("setup-skip").textContent).toContain("Skip");
    expect(screen.getByTestId("setup-next").textContent).toContain("Next: Keep running");
    expect(screen.getByTestId("setup-next").hasAttribute("data-primary")).toBe(false);
    fireEvent.click(within(setup).getByTestId("setup-skip"));
    expect(await screen.findByTestId("setup-keep")).toBeTruthy();
  });
});

describe("Keep it running and the tour", () => {
  it("offers opening at login only where it can, keeping awake on, then the tour", async () => {
    seam("__AIJ_OS_PROMPTS__", [prompt("accessibility", "allowed")]);
    seam("__AIJ_LOGIN_ITEM__", "unavailable");
    const onClose = vi.fn();
    const asked = vi.fn();
    window.addEventListener(TAKE_TOUR_EVENT, asked);
    render(<SetupWizard openLoginTerminal={() => null} onClose={onClose} />);
    await waitFor(() => expect(screen.getByTestId("setup-steps").textContent).not.toContain("Mac access"));
    fireEvent.click(screen.getByTestId("setup-skip"));
    fireEvent.click(await screen.findByTestId("setup-next"));
    expect(await screen.findByTestId("setup-keep")).toBeTruthy();
    expect(await screen.findByTestId("setup-keep-awake")).toBeTruthy();
    expect(screen.queryByTestId("setup-keep-login")).toBeNull();
    expect(screen.getByTestId("setup-keep-awake").getAttribute("aria-checked")).toBe("true");
    fireEvent.click(screen.getByTestId("setup-next"));
    expect(await screen.findByTestId("setup-tour")).toBeTruthy();
    fireEvent.click(screen.getByTestId("setup-take-tour"));
    expect(onClose).toHaveBeenCalledOnce();
    expect(asked).toHaveBeenCalledOnce();
    window.removeEventListener(TAKE_TOUR_EVENT, asked);
  });

  it("turns opening at login on where the build can", async () => {
    seam("__AIJ_OS_PROMPTS__", [prompt("accessibility", "allowed")]);
    seam("__AIJ_LOGIN_ITEM__", "off");
    render(<SetupWizard openLoginTerminal={() => null} onClose={() => null} />);
    await waitFor(() => expect(screen.getByTestId("setup-steps").textContent).not.toContain("Mac access"));
    fireEvent.click(screen.getByTestId("setup-skip"));
    fireEvent.click(await screen.findByTestId("setup-next"));
    const login = await screen.findByTestId("setup-keep-login");
    await act(async () => {
      fireEvent.click(login);
    });
    await waitFor(() => expect(seamOf("__AIJ_LOGIN_ITEM__")).toBe("on"));
    expect(screen.getByTestId("setup-keep-login").getAttribute("aria-checked")).toBe("true");
  });
});
