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
import { fromAccess, permissionsValue } from "../../lib/setup-permissions";
import { STATES as MODEL_STATES } from "../../preview/ai-models";
import { TAKE_TOUR_EVENT } from "../../lib/tour";
import type { OsAccessInfo } from "../../types/generated/OsAccessInfo";
import type { AiJobInfo } from "../../types/generated/AiJobInfo";

const seam = (name: string, value: unknown) => Reflect.set(window, name, value);
const seamOf = (name: string): unknown => Reflect.get(window, name);

const access = (id: string, status: string, over: Partial<OsAccessInfo> = {}): OsAccessInfo => ({
  id,
  title: id === "focus" ? "Jump to a session's own window" : "See a live preview of each session",
  why: "Why it helps.",
  system_name: id === "focus" ? "Accessibility and Automation" : "Screen Recording",
  status,
  action: status === "allowed" ? "none" : "prompt",
  note: null,
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

  it("each permission comes in the OS-prompt registry's shape (AJ-E1547)", () => {
    const entry = fromAccess(access("focus", "not_asked"));
    expect(entry).toMatchObject({ id: "focus", needed: true, picture: null });
    expect(entry.system_says).toContain("accessibility features");
    expect(entry.click.length).toBeGreaterThan(0);
    expect(entry.if_declined.length).toBeGreaterThan(0);
    expect(fromAccess(access("other", "not_asked")).system_says).toBe("macOS asks about Screen Recording.");
    expect(permissionsValue([fromAccess(access("focus", "allowed")), fromAccess(access("preview", "not_asked"))])).toBe("1 of 2 allowed");
    expect(permissionsValue([fromAccess(access("focus", "allowed"))])).toBe("All allowed");
  });
});

describe("Mac access", () => {
  it("one card per permission; Allow raises the system prompt and the grant shows live; Skip for now on each", async () => {
    seam("__AIJ_OS_ACCESS__", [access("focus", "not_asked"), access("preview", "not_asked")]);
    render(<SetupWizard resumeAtAccess openLoginTerminal={() => null} onClose={() => null} />);
    const focus = await screen.findByTestId("setup-permission-focus");
    expect(screen.getByTestId("setup-permission-preview")).toBeTruthy();
    expect(screen.getByTestId("setup-title").textContent).toBe("Let AI Juggler work with your Mac");
    // What macOS will show, what to click, what happens if declined.
    const prompt = within(focus).getByTestId("setup-permission-focus-prompt");
    expect(prompt.textContent).toContain("macOS will show");
    expect(prompt.textContent).toContain("Click");
    expect(prompt.textContent).toContain("If you say no");
    expect(focus.textContent).toContain("macOS will ask you · optional · change it later in Settings");
    await act(async () => {
      fireEvent.click(within(focus).getByTestId("setup-permission-focus-allow"));
    });
    expect(seamOf("__AIJ_OS_ACCESS_CALLS__")).toEqual(["request:focus"]);
    await waitFor(() => expect(screen.getByTestId("setup-permission-focus").dataset.state).toBe("allowed"));
    expect(screen.getByTestId("setup-permission-focus-allowed").textContent).toContain("Allowed");
    // The other one is skipped, and says how to allow it later.
    fireEvent.click(screen.getByTestId("setup-permission-preview-skip"));
    expect(screen.getByTestId("setup-permission-preview").dataset.state).toBe("skipped");
    expect(screen.getByTestId("setup-permission-preview").textContent).toContain("Skipped for now");
    // The steps beside say how far it got; Next names Meet Bot.
    expect(screen.getByTestId("setup-next").textContent).toContain("Next: Meet Bot");
    expect(screen.getByTestId("setup-steps").textContent).toContain("1 of 2 allowed");
  });

  it("a grant given in System Settings shows when the window comes back", async () => {
    seam("__AIJ_OS_ACCESS__", [access("focus", "not_allowed", { action: "settings" })]);
    render(<SetupWizard resumeAtAccess openLoginTerminal={() => null} onClose={() => null} />);
    await screen.findByTestId("setup-permission-focus");
    seam("__AIJ_OS_ACCESS__", [access("focus", "allowed")]);
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() => expect(screen.getByTestId("setup-permission-focus").dataset.state).toBe("allowed"));
  });

  it("isn't a step when nothing is missing", async () => {
    seam("__AIJ_OS_ACCESS__", [access("focus", "allowed")]);
    render(<SetupWizard openLoginTerminal={() => null} onClose={() => null} />);
    await waitFor(() => expect(screen.getByTestId("setup-steps").textContent).not.toContain("Mac access"));
  });
});

describe("left and resumed", () => {
  it("Not now remembers the step; opened again, it carries on there; finishing forgets it", async () => {
    seam("__AIJ_OS_ACCESS__", [access("focus", "not_asked")]);
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
    seam("__AIJ_OS_ACCESS__", [access("focus", "not_asked")]);
    render(<SetupWizard openLoginTerminal={() => null} onClose={onClose} />);
    expect(await screen.findByTestId("first-run")).toBeTruthy();
    expect(screen.getByTestId("first-run").dataset.embedded).toBe("true");
    // First run titles its own step (no second title over it).
    expect(screen.queryByTestId("setup-title")).toBeNull();
    expect(screen.getByTestId("setup-eyebrow").textContent).toBe("Step 1 of 5");
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
  ] as const)("%s: says what Bot does, and never offers an engine of its own", async (_name, state, standing) => {
    seam("__AIJ_AI_JOBS__", MODEL_STATES[state]!.jobs);
    const onOpenModels = vi.fn();
    render(<MeetBotStep onOpenModels={onOpenModels} />);
    const step = screen.getByTestId("setup-bot");
    expect(step.textContent).toContain("your AI assistant for everything AI Juggler sees");
    expect(step.textContent).toContain("Cites every answer");
    expect(step.textContent).toContain("Never acts");
    expect(step.textContent).not.toMatch(/look-ups?/i);
    await waitFor(() => expect(screen.getByTestId("setup-bot-engine").dataset.standing).toBe(standing));
    // No download is ever started from here (D-131): no Download button.
    expect(screen.queryByRole("button", { name: /Download/ })).toBeNull();
    if (standing === "ready") {
      expect(screen.getByTestId("setup-bot-engine").textContent).toContain("Bot is ready");
    } else {
      fireEvent.click(screen.getByTestId("setup-bot-models"));
      expect(onOpenModels).toHaveBeenCalledOnce();
    }
  });

  it("the foot keeps Skip for now while Bot isn't ready", async () => {
    seam("__AIJ_OS_ACCESS__", [access("focus", "allowed")]);
    seam("__AIJ_AI_JOBS__", MODEL_STATES.nolist!.jobs);
    render(<SetupWizard openLoginTerminal={() => null} onClose={() => null} />);
    await waitFor(() => expect(screen.getByTestId("setup-steps").textContent).not.toContain("Mac access"));
    fireEvent.click(screen.getByTestId("setup-skip"));
    expect(await screen.findByTestId("setup-bot")).toBeTruthy();
    expect(screen.getByTestId("setup-skip").textContent).toBe("Skip for now");
    expect(screen.getByTestId("setup-next").textContent).toContain("Next: Keep running");
  });
});

describe("Keep it running and the tour", () => {
  it("offers opening at login only where it can, keeping awake on, then the tour", async () => {
    seam("__AIJ_OS_ACCESS__", [access("focus", "allowed")]);
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
    seam("__AIJ_OS_ACCESS__", [access("focus", "allowed")]);
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
