/**
 * AJ-E1527 — "Make it smoother": the optional permissions on the welcome,
 * in the same rows as first run's, never required; Home's small reminder
 * for the first one still missing; "Not now" remembered.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SmootherCard } from "./SmootherCard";
import type { OsAccessInfo } from "../lib/bridge";

const w = () => window as unknown as { __AIJ_OS_ACCESS__?: OsAccessInfo[]; __AIJ_OS_ACCESS_CALLS__?: string[] };

const entry = (id: string, status: string, overrides: Partial<OsAccessInfo> = {}): OsAccessInfo => ({
  id,
  title: id === "focus" ? "Jump to a session's own window" : "See a live preview of each session",
  why: "Why it helps.",
  system_name: id === "focus" ? "Accessibility and Automation" : "Screen Recording",
  status,
  action: status === "allowed" ? "none" : "prompt",
  note: null,
  ...overrides,
});

beforeEach(() => {
  localStorage.clear();
  w().__AIJ_OS_ACCESS_CALLS__ = [];
});
afterEach(() => {
  cleanup();
  delete w().__AIJ_OS_ACCESS__;
});

describe("Make it smoother", () => {
  it("the welcome lists what's missing in the shared rows; Allow asks the system", async () => {
    w().__AIJ_OS_ACCESS__ = [entry("focus", "not_asked"), entry("preview", "allowed")];
    render(<SmootherCard variant="welcome" />);
    const card = await screen.findByTestId("smoother");
    expect(card.textContent).toContain("Optional: AI Juggler works without them.");
    expect(screen.getByTestId("smoother-focus")).toBeTruthy();
    expect(screen.getByTestId("smoother-preview").textContent).toContain("Allowed");
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /^Allow Jump to/ }));
    });
    expect(w().__AIJ_OS_ACCESS_CALLS__).toEqual(["request:focus"]);
  });

  it("isn't shown when everything is allowed", async () => {
    w().__AIJ_OS_ACCESS__ = [entry("focus", "allowed")];
    const { container } = render(<SmootherCard variant="welcome" />);
    await waitFor(() => expect(container.textContent).toBe(""));
  });

  it("the welcome's Not now hides it and Home's reminder, for good", async () => {
    w().__AIJ_OS_ACCESS__ = [entry("focus", "not_asked")];
    render(<SmootherCard variant="welcome" />);
    fireEvent.click(await screen.findByTestId("smoother-not-now"));
    expect(screen.queryByTestId("smoother")).toBeNull();
    cleanup();
    const { container } = render(<SmootherCard variant="reminder" />);
    await waitFor(() => expect(container.textContent).toBe(""));
  });

  it("Home's reminder shows the first one missing; its Not now puts away that one only", async () => {
    w().__AIJ_OS_ACCESS__ = [entry("focus", "not_allowed", { action: "settings" }), entry("preview", "not_asked")];
    render(<SmootherCard variant="reminder" />);
    expect(await screen.findByTestId("smoother-reminder-focus")).toBeTruthy();
    expect(screen.queryByTestId("smoother-reminder-preview")).toBeNull();
    fireEvent.click(screen.getByTestId("smoother-reminder-not-now"));
    expect(await screen.findByTestId("smoother-reminder-preview")).toBeTruthy();
    expect(JSON.parse(localStorage.getItem("aij.smoother.dismissed")!)).toEqual(["focus"]);
  });
});
