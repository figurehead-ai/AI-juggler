/**
 * 0.4.1 (the rc12 walk): the switch dialog said "Room on f…@…: 78% left
 * this week" for an App New session calls not signed in. One rule for
 * both (New session's), and no account's details: it says "Not signed
 * in".
 */
import { cleanup, render, renderHook, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as data from "../../pages/session/data";
import * as subscriptions from "../../lib/subscriptions";
import { account } from "../../test/fixtures";
import { NOT_SIGNED_IN } from "../../lib/room-targets";
import { RoomNote, useTargetRoom } from "./TargetRoom";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("0.4.1 the switch's room line for an App that isn't signed in", () => {
  it("says Not signed in, with no account and no percentage", () => {
    vi.spyOn(data, "useProfiles").mockReturnValue([account({ id: "test", label: "aij-test", login_state: "logged_out" })]);
    vi.spyOn(data, "useRunways").mockReturnValue([]);
    vi.spyOn(subscriptions, "useSubscriptions").mockReturnValue(null);
    const { result } = renderHook(() => useTargetRoom("test"));
    expect(result.current).toMatchObject({ kind: "blocked", left: null, account: null, words: NOT_SIGNED_IN });
    render(<RoomNote room={result.current} target="Claude Code · aij-test" />);
    const line = screen.getByTestId("switch-room");
    expect(line.textContent).toBe(`Claude Code · aij-test: ${NOT_SIGNED_IN}`);
    expect(line.textContent).not.toMatch(/%|@|Room on/);
  });

  it("a signed-in one keeps its room line", () => {
    vi.spyOn(data, "useProfiles").mockReturnValue([account({ id: "test", login_state: "logged_in" })]);
    vi.spyOn(data, "useRunways").mockReturnValue([]);
    vi.spyOn(subscriptions, "useSubscriptions").mockReturnValue(null);
    const { result } = renderHook(() => useTargetRoom("test"));
    expect(result.current?.kind).not.toBe("blocked");
  });

  it("rc12: continuing in Desktop, the signed-in app profile's room is shown", () => {
    vi.spyOn(data, "useProfiles").mockReturnValue([account({ id: "test", login_state: "logged_out" })]);
    vi.spyOn(data, "useRunways").mockReturnValue([]);
    vi.spyOn(subscriptions, "useSubscriptions").mockReturnValue(null);
    const { result } = renderHook(() => useTargetRoom("test", true));
    expect(result.current?.kind).not.toBe("blocked");
  });
});
