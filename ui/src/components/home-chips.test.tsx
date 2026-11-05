/**
 * AJ-E2910 vitest — Home's Needs you chips: each kind under its chip
 * (failed, the Mac, heavy work and leftovers under All only), chips only
 * with two kinds or more, a filtered list saying what it hides, and Mark
 * all N reviewed on Pick up where you left off.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomeNeeds } from "./HomeNeeds";
import { HomeRecent } from "./HomeRecent";
import { chipOf, homeChips, type HomeNeed } from "../lib/home-needs";
import type { InstanceInfo } from "../lib/bridge";
import type { RailItem } from "../lib/rail-items";

afterEach(cleanup);

const asks = (key: string, need: "asks" | "stuck" | "review" = "asks"): HomeNeed => ({
  key,
  kind: "session",
  title: `Session ${key}`,
  detail: "shop",
  severe: false,
  item: { need, place: { key: "cli|x", kind: "cli", profile: "Work" }, accountLabel: "Work", session: null, conversation: null } as unknown as RailItem,
});
const signin: HomeNeed = { key: "signin:a", kind: "signin", title: "Work needs its sign-in", detail: "", severe: false };
const mac: HomeNeed = { key: "mac", kind: "mac", title: "This Mac is working hard", detail: "", severe: false };

describe("AJ-E2910 — Home's chips", () => {
  it("puts each need under its chip, and the rest under All only", () => {
    expect(chipOf(asks("a"))).toBe("asks");
    expect(chipOf(asks("b", "stuck"))).toBe("stuck");
    expect(chipOf(asks("c", "review"))).toBe("review");
    expect(chipOf(signin)).toBe("signin");
    expect(chipOf(mac)).toBeNull();
  });

  it("offers chips only when two kinds or more are there", () => {
    expect(homeChips([asks("a"), asks("b")])).toEqual([]);
    expect(homeChips([asks("a"), signin, mac]).map((chip) => chip.label)).toEqual(["All 3", "Asks you 1", "Sign-in 1"]);
    expect(homeChips([asks("a")], { asks: 0, review: 2 }).map((chip) => chip.label)).toEqual(["All 3", "Asks you 1", "To review 2"]);
  });

  it("narrows the list and says what it hides, never remembered", () => {
    const quota: HomeNeed = { key: "quota:a", kind: "quota", title: "Work is almost out of room", detail: "", severe: false };
    render(<HomeNeeds needs={[quota, signin, mac]} handlers={{ onOpenSession: vi.fn() }} />);
    expect(screen.getByTestId("home-chips")).toBeTruthy();
    fireEvent.click(screen.getByRole("radio", { name: /Sign-in 1/ }));
    expect(screen.queryByText("Work is almost out of room")).toBeNull();
    expect(screen.getByText("Work needs its sign-in")).toBeTruthy();
    expect(screen.getByTestId("home-chips-hidden").textContent).toBe("2 more under All");
    fireEvent.click(screen.getByTestId("home-chips-hidden"));
    expect(screen.getByText("Work is almost out of room")).toBeTruthy();
  });

  it("Mark all N reviewed covers the whole list", () => {
    const onReviewAll = vi.fn();
    const session = (id: string) => ({ id, status: "finished", started_at_ms: 0, account_label: "Work", conversation_id: null, account_id: "a", project_id: null, provider: "claude-code", git_branch: null, label: null, title: null }) as unknown as InstanceInfo;
    render(<HomeRecent recent={[session("s1"), session("s2"), session("s3")]} total={7} shown={3} showAll={false} onShowAll={vi.fn()} projects={[]} now={Date.now()} busy={null} onOpen={vi.fn()} onReviewed={vi.fn()} onReviewAll={onReviewAll} />);
    fireEvent.click(screen.getByTestId("home-review-all"));
    expect(screen.getByTestId("home-review-all").textContent).toBe("Mark all 7 reviewed");
    expect(onReviewAll).toHaveBeenCalledTimes(1);
  });
});
