/**
 * AJ-E1527 — the optional permissions, never required: Home's small
 * reminder for the first one still missing, in first run's rows; "Not
 * now" remembered (AJ-E1545 moved the welcome's card into the setup
 * wizard's permissions step).
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SmootherCard } from "./SmootherCard";
import type { OsAccessInfo } from "../types/generated/OsAccessInfo";

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
  it("isn't shown when everything is allowed", async () => {
    w().__AIJ_OS_ACCESS__ = [entry("focus", "allowed")];
    const { container } = render(<SmootherCard />);
    await waitFor(() => expect(container.textContent).toBe(""));
  });

  it("an earlier Not now on the welcome's old card still puts the reminder away", async () => {
    w().__AIJ_OS_ACCESS__ = [entry("focus", "not_asked")];
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    const { container } = render(<SmootherCard />);
    await waitFor(() => expect(container.textContent).toBe(""));
  });

  it("Allow on the reminder asks the system", async () => {
    w().__AIJ_OS_ACCESS__ = [entry("focus", "not_asked")];
    render(<SmootherCard />);
    await screen.findByTestId("smoother-reminder-focus");
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /^Allow Jump to/ }));
    });
    expect(w().__AIJ_OS_ACCESS_CALLS__).toEqual(["request:focus"]);
  });

  it("Home's reminder shows the first one missing; its Not now puts away that one only", async () => {
    w().__AIJ_OS_ACCESS__ = [entry("focus", "not_allowed", { action: "settings" }), entry("preview", "not_asked")];
    render(<SmootherCard />);
    expect(await screen.findByTestId("smoother-reminder-focus")).toBeTruthy();
    expect(screen.queryByTestId("smoother-reminder-preview")).toBeNull();
    fireEvent.click(screen.getByTestId("smoother-reminder-not-now"));
    expect(await screen.findByTestId("smoother-reminder-preview")).toBeTruthy();
    expect(JSON.parse(localStorage.getItem("aij.smoother.dismissed")!)).toEqual(["focus"]);
  });
});
