/**
 * D-118 (AJ-E3702 R2) vitest — Home's pinned zone ("Needs you now"): no
 * area title of its own (the zone's head and the page's heading say it),
 * no filter chips (the feed's filters are the one filter row, and never
 * hide a need), and past 3 the rest wait behind "+n more need you".
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomeNeeds, PINNED_SHOWN, revealNeed } from "./HomeNeeds";
import { primaryWords, type HomeNeed } from "../lib/home-needs";

afterEach(cleanup);

const signin: HomeNeed = { key: "signin:a", kind: "signin", title: "Work needs its sign-in", detail: "", severe: false };
const mac: HomeNeed = { key: "mac", kind: "mac", title: "This Mac is working hard", detail: "", severe: false };
const many = (n: number): HomeNeed[] => Array.from({ length: n }, (_, at) => ({ key: `signin:${at}`, kind: "signin", title: `Account ${at} needs its sign-in`, detail: "", severe: false }));

describe("D-118 — the pinned zone", () => {
  it("has no area title over it: the zone's head says what needs you (0.4 design review, 42)", () => {
    render(<HomeNeeds needs={[signin, mac]} handlers={{ onOpenSession: vi.fn() }} />);
    expect(screen.getByRole("region", { name: "Needs you" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Needs you" })).toBeNull();
    expect(screen.queryByRole("radiogroup")).toBeNull();
  });

  it("says a limit as its rows do, and Continue one way (0.4 design review, 69)", () => {
    const limit: HomeNeed = { key: "lim", kind: "limit", title: "Work hit its limit", detail: "", severe: true };
    expect(primaryWords(limit)).toBe("Continue in another app…");
  });

  it("shows 3, then the rest behind +n more need you, every one a click away", () => {
    render(<HomeNeeds needs={many(5)} handlers={{ onOpenSession: vi.fn() }} />);
    expect(PINNED_SHOWN).toBe(3);
    expect(screen.getAllByTestId("home-need-signin")).toHaveLength(3);
    expect(screen.getByTestId("home-needs-more").textContent).toBe("+2 more need you");
    fireEvent.click(screen.getByTestId("home-needs-more"));
    expect(screen.getAllByTestId("home-need-signin")).toHaveLength(5);
    expect(screen.queryByTestId("home-needs-more")).toBeNull();
  });

  it("opens to a held need when the heading names it", async () => {
    render(<HomeNeeds needs={many(5)} handlers={{ onOpenSession: vi.fn() }} />);
    revealNeed("[data-need-key='signin:4']");
    expect(await screen.findByText("Account 4 needs its sign-in")).toBeTruthy();
  });
});

describe("Segmented, wrapped (the 0.4 re-render: 'Limit reache' off the card at 390 px)", () => {
  it("keeps every chip's words whole and lets the row wrap", async () => {
    const { Segmented } = await import("./controls/Segmented");
    render(
      <Segmented label="Show what needs you" value="all" wrap testId="chip" onChange={() => undefined}
        segments={[{ value: "all", label: "All 5" }, { value: "limit", label: "Limit reached 1" }]} />,
    );
    expect(screen.getByRole("radiogroup").className).toContain("flex-wrap");
    for (const radio of screen.getAllByRole("radio")) {
      expect(radio.className).not.toContain("truncate");
      expect(radio.className).toContain("whitespace-nowrap");
    }
  });
});
