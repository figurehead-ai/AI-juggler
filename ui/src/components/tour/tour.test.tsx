/**
 * AJ-E1546 — the tour: the registry (each step's feature, version, target
 * and Bot's words), completion stored per id, the once-only "New in" offer
 * for steps not seen yet, and the coach marks (anchored when the element
 * is on screen, the clip otherwise; Back, Next, Skip, the progress bar,
 * Esc; "Ask Bot about this" only while Bot's AI is ready; Bot's own words
 * without an AI, D-144).
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Tour, placeCard } from "./Tour";
import { TourHost } from "./TourHost";
import { currentOffer, doneSteps, markDone, markOffered, newOffer, newer, startTour, stepsFor, TOUR, type TourStep } from "../../lib/tour";
import { isClip } from "../../lib/clips";
import { STATES as MODEL_STATES } from "../../preview/ai-models";

const seam = (name: string, value: unknown) => Reflect.set(window, name, value);

beforeEach(() => {
  localStorage.clear();
  Reflect.deleteProperty(window, "__AIJ_AI_JOBS__");
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  document.body.innerHTML = "";
});

describe("the registry", () => {
  it("every step has a unique id, a feature, its version, a target by id, Bot's words and a question", () => {
    expect(new Set(TOUR.map((step) => step.id)).size).toBe(TOUR.length);
    expect(new Set(TOUR.map((step) => step.feature)).size).toBe(TOUR.length);
    for (const step of TOUR) {
      expect(step.since).toMatch(/^\d+\.\d+\.\d+$/);
      expect(step.target).toMatch(/data-testid=/);
      expect(step.text.length).toBeGreaterThan(20);
      expect(step.ask.length).toBeGreaterThan(5);
      if (step.clip) expect(isClip(step.clip)).toBe(true);
      expect(`${step.text} ${step.textNoAi ?? ""}`).not.toMatch(/look-ups?/i);
    }
  });

  it("starts at Bot's panel, in Bot's voice, with words for no AI too (D-144)", () => {
    const first = TOUR[0]!;
    expect(first.id).toBe("bot");
    expect(first.opens).toBe("bot");
    expect(first.text).toContain("your AI assistant for everything AI Juggler sees");
    expect(first.textNoAi).toContain("Set up Bot");
    expect(first.textNoAi).toContain("Search (⌘K)");
  });

  it("covers the specced stops", () => {
    expect(TOUR.map((step) => step.id)).toEqual(["bot", "open-now", "needs-you", "session", "usage", "switch", "pools", "processes", "settings"]);
  });
});

describe("completion per id and the New in offer", () => {
  it("stores each step seen, once", () => {
    markDone("bot");
    markDone("bot");
    markDone("usage");
    expect(doneSteps()).toEqual(["bot", "usage"]);
  });

  it("offers nothing to someone who never took it, or once everything is seen", () => {
    expect(newOffer(TOUR, [], [])).toBeNull();
    expect(newOffer(TOUR, TOUR.map((step) => step.id), [])).toBeNull();
  });

  it("after an update, offers only the steps not seen, once, as New in the newest version", () => {
    const extra: TourStep = { ...TOUR[1]!, id: "later", feature: "later", since: "0.5.2" };
    const steps = [...TOUR, extra];
    const seen = TOUR.map((step) => step.id);
    expect(newOffer(steps, seen, [])).toEqual({ ids: ["later"], label: "New in 0.5.x" });
    expect(newOffer(steps, seen, ["later"])).toBeNull();
    expect(newer("0.5.2", "0.5.10")).toBe(false);
    expect(newer("0.10.0", "0.9.9")).toBe(true);
  });

  it("the offer reads this Mac's memory, and is put away once answered", () => {
    for (const step of TOUR.slice(1)) markDone(step.id);
    expect(currentOffer()).toEqual({ ids: ["bot"], label: "New in 0.5.x" });
    markOffered(["bot"]);
    expect(currentOffer()).toBeNull();
  });

  it("the host shows the offer only where it's allowed, and Show me starts those steps", async () => {
    for (const step of TOUR.slice(1)) markDone(step.id);
    const { rerender } = render(<TourHost offerAllowed={false} />);
    expect(screen.queryByTestId("tour-new")).toBeNull();
    rerender(<TourHost offerAllowed />);
    expect(screen.getByTestId("tour-new").textContent).toContain("New in 0.5.x");
    await act(async () => {
      fireEvent.click(screen.getByTestId("tour-new-show"));
    });
    expect(await screen.findByTestId("tour")).toBeTruthy();
    expect(screen.getByTestId("tour-count").textContent).toBe("1 of 1");
    expect(currentOffer()).toBeNull();
  });
});

describe("the coach marks", () => {
  const steps = stepsFor(["open-now", "usage", "pools"]);

  it("anchors on its element when it's on screen, else shows the clip; Next, Back, the progress bar", async () => {
    const target = document.createElement("div");
    target.dataset.testid = "rail-open-now";
    target.getBoundingClientRect = () => ({ top: 100, left: 20, width: 200, height: 40, bottom: 140, right: 220, x: 20, y: 100, toJSON: () => ({}) });
    document.body.append(target);
    const onEnd = vi.fn();
    render(<Tour steps={steps} onEnd={onEnd} />);
    await waitFor(() => expect(screen.getByTestId("tour").dataset.anchored).toBe("true"));
    expect(screen.getByTestId("tour-ring")).toBeTruthy();
    expect(screen.getByRole("progressbar").getAttribute("value")).toBe("1");
    expect(screen.queryByTestId("tour-back")).toBeNull();
    fireEvent.click(screen.getByTestId("tour-next"));
    // Usage's tab isn't on screen here: its clip stands in.
    await waitFor(() => expect(screen.getByTestId("tour").dataset.anchored).toBe("false"));
    expect(screen.getByTestId("clip-what-usage")).toBeTruthy();
    expect(screen.getByRole("progressbar").getAttribute("value")).toBe("2");
    fireEvent.click(screen.getByTestId("tour-back"));
    expect(screen.getByTestId("tour").dataset.step).toBe("open-now");
    expect(doneSteps()).toEqual(["open-now", "usage"]);
  });

  it("Esc ends it, Skip ends it, Done ends it; a dialog named by its title", () => {
    const onEnd = vi.fn();
    render(<Tour steps={steps} onEnd={onEnd} />);
    const card = screen.getByTestId("tour-card");
    expect(card.tagName).toBe("DIALOG");
    expect(card.getAttribute("aria-modal")).toBe("true");
    expect(document.getElementById(card.getAttribute("aria-labelledby")!)!.textContent).toBe("Open now");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onEnd).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByTestId("tour-skip"));
    expect(onEnd).toHaveBeenCalledTimes(2);
    fireEvent.keyDown(window, { key: "ArrowRight" });
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(screen.getByTestId("tour-next").textContent).toBe("Done");
    fireEvent.click(screen.getByTestId("tour-next"));
    expect(onEnd).toHaveBeenCalledTimes(3);
  });

  it("Ask Bot about this shows only while Bot's AI is ready; without one, Bot says how to set it up", async () => {
    seam("__AIJ_AI_JOBS__", MODEL_STATES.nolist!.jobs);
    const { unmount } = render(<Tour steps={stepsFor(["bot"])} onEnd={() => null} />);
    await waitFor(() => expect(screen.getByTestId("tour-card").textContent).toContain("Set up Bot"));
    expect(screen.queryByTestId("tour-ask-bot")).toBeNull();
    unmount();
    seam("__AIJ_AI_JOBS__", MODEL_STATES.installed!.jobs);
    // The shared read keeps its answer until told it changed.
    window.dispatchEvent(new Event("ai://jobs-changed"));
    render(<Tour steps={stepsFor(["usage"])} onEnd={() => null} />);
    expect(await screen.findByTestId("tour-ask-bot")).toBeTruthy();
  });

  it("places the card under, over or beside its element, inside the window", () => {
    const view = { width: 1280, height: 800 };
    expect(placeCard({ top: 40, left: 600, width: 100, height: 30 }, view)).toEqual({ top: 82, left: 470 });
    expect(placeCard({ top: 700, left: 1250, width: 20, height: 30 }, view)).toEqual({ bottom: 112, left: 904 });
    // Bot's panel, tall at the right: the card beside it, centred.
    expect(placeCard({ top: 60, left: 880, width: 400, height: 700 }, view)).toEqual({ top: 400, middle: true, left: 508 });
  });

  it("starts from anywhere through its event", async () => {
    render(<TourHost offerAllowed={false} />);
    await act(async () => startTour(["settings"]));
    expect(await screen.findByTestId("tour")).toBeTruthy();
    expect(screen.getByTestId("tour").dataset.step).toBe("settings");
  });
});
