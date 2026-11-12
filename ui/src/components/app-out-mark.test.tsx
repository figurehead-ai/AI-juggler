/**
 * 0.4.3 (the maintainer's 0.4.2 test): three running Claude app profiles
 * whose Accounts were out of weekly room showed no Out mark, because the
 * rail's header read it from its first open session, and they had none
 * open. The mark is the App's Account's, from the App's own folder: on an
 * Open now header with nothing open, on an Inactive apps row, and on the
 * Accounts screen's App row (spec 11, rule 6: one mark wherever the App
 * appears).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RoomMark } from "../lib/room-mark";

const OUT: RoomMark = { kind: "out", label: "Out · resets Sun 6:00 AM", short: "Out", title: "Out of room until Sun 6:00 AM", about: false, resetsAtMs: null };

vi.mock("./RoomChip", async (actual) => {
  const real = await actual<typeof import("./RoomChip")>();
  const mark = (folder: string | null | undefined) => (folder === "out-folder" ? OUT : null);
  return {
    ...real,
    useRoomMark: mark,
    ProfileRoomChip: ({ profileId, testId }: { profileId: string | null | undefined; testId?: string }) => <real.RoomChip mark={mark(profileId)} testId={testId} />,
  };
});

const { PlaceHeader } = await import("./PlaceHeader");
const { RailIdleApp } = await import("./RailIdleApp");

afterEach(cleanup);

const place = { key: "app|claude-app|South", app: "Claude app", appId: "claude-app", kind: "desktop" as const, profile: "South", appPid: 7, host: null };

describe("0.4.3 an App out of room wears the mark with nothing open in it", () => {
  it("an Open now header with nothing open: its App's folder's mark, and no fold when nothing unfolds", () => {
    render(<PlaceHeader place={place} label="Claude app" count={0} needs={0} folded={null} onToggle={() => {}} roomOf="out-folder" foldable={false} />);
    expect(screen.getByTestId("place-room").dataset.kind).toBe("out");
    // Said in its name too (the drawn mark is aria-hidden): a screen reader, and the real-Mac check, read it.
    expect(screen.getByRole("button", { name: "Claude app · South, nothing open, Out of room until Sun 6:00 AM" })).toBeTruthy();
    expect(screen.getByTestId("place-nothing-open").textContent).toBe("nothing open");
    expect(screen.queryByTestId("rail-fold")).toBeNull();
    expect(screen.getByTestId("place-no-fold")).toBeTruthy();
  });

  it("an Account with room: no mark, and a header that unfolds keeps its fold", () => {
    render(<PlaceHeader place={place} label="Claude app" count={0} needs={0} folded={null} onToggle={() => {}} roomOf="ok-folder" foldable />);
    expect(screen.queryByTestId("place-room")).toBeNull();
    expect(screen.getByTestId("rail-fold")).toBeTruthy();
  });

  it("an Inactive apps row: its own folder's mark, desktop or CLI", () => {
    const base = { name: "Claude app · South", app: "Claude app", profile: "South", appId: "claude-app", lastMs: 0, appPid: null, cannotStart: null, tool: null, sessions: [] };
    render(<RailIdleApp app={{ ...base, key: "app|claude-app|South", kind: "desktop", placeId: "app:claude-app:/p/South", accountId: null, folder: "out-folder" }} open={false} onToggle={() => {}} now={0} />);
    render(<RailIdleApp app={{ ...base, key: "cli|claude-code|out-folder", kind: "cli", name: "Claude Code CLI · South", placeId: null, accountId: "out-folder", folder: "out-folder", tool: "claude-code" }} open={false} onToggle={() => {}} now={0} />);
    render(<RailIdleApp app={{ ...base, key: "app|claude-app|North", kind: "desktop", placeId: "app:claude-app:/p/North", accountId: null, folder: "ok-folder" }} open={false} onToggle={() => {}} now={0} />);
    expect(screen.getAllByTestId("rail-idle-room").map((mark) => mark.dataset.kind)).toEqual(["out", "out"]);
    expect(screen.getAllByRole("button", { name: /Out of room until/ })).toHaveLength(2);
  });
});
